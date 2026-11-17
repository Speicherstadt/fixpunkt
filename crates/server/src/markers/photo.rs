//! Fotos für die Dokumentation: Position und Aufnahmezeit aus EXIF, Vorschaubild.

use anyhow::{Context, Result};
use exif::{In, Reader, Tag, Value};
use image::ImageFormat;
use std::io::Cursor;

/// Längste Seite des Vorschaubilds in Pixeln
pub const THUMB: u32 = 480;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Exif {
    /// Breite, Länge in Grad (WGS84)
    pub lat_lon: Option<(f64, f64)>,
    /// GPS-Höhe in m (ellipsoidisch oder über NN – je nach Gerät, nur zur Information)
    pub altitude: Option<f64>,
    /// Aufnahmezeit, ms seit 1970
    pub taken_at: Option<i64>,
    /// Blickrichtung in Grad von Nord im Uhrzeigersinn (GPSImgDirection; „M“ magnetisch wird wie geografisch
    /// behandelt, in Deutschland 3–4° Unterschied)
    pub direction: Option<f64>,
}

fn rationals(v: &Value) -> Option<Vec<f64>> {
    match v {
        Value::Rational(r) => Some(r.iter().map(|x| x.to_f64()).collect()),
        _ => None,
    }
}

fn ascii(v: &Value) -> Option<String> {
    match v {
        Value::Ascii(a) => a
            .first()
            .map(|s| String::from_utf8_lossy(s).trim().to_string()),
        _ => None,
    }
}

/// Grad, Minuten, Sekunden → Grad; Süd und West negativ
pub fn degrees(dms: &[f64], reference: &str) -> Option<f64> {
    let [d, m, s] = dms else { return None };
    let v = d + m / 60.0 + s / 3600.0;
    Some(if matches!(reference, "S" | "W") {
        -v
    } else {
        v
    })
}

/// „2026:11:17 09:30:00“ mit Versatz „+01:00“ (fehlt er: UTC) → ms seit 1970
pub fn exif_time(date_time: &str, offset: Option<&str>) -> Option<i64> {
    let fmt = time::macros::format_description!("[year]:[month]:[day] [hour]:[minute]:[second]");
    let t = time::PrimitiveDateTime::parse(date_time.trim(), &fmt).ok()?;
    let off = offset
        .and_then(|o| {
            let sign = if o.starts_with('-') { -1 } else { 1 };
            let (h, m) = o.trim_start_matches(['+', '-']).split_once(':')?;
            time::UtcOffset::from_hms(
                sign * h.parse::<i8>().ok()?,
                sign * m.parse::<i8>().ok()?,
                0,
            )
            .ok()
        })
        .unwrap_or(time::UtcOffset::UTC);
    Some((t.assume_offset(off).unix_timestamp_nanos() / 1_000_000) as i64)
}

/// EXIF eines Fotos; ohne EXIF alles leer
pub fn read_exif(bytes: &[u8]) -> Exif {
    let none = Exif {
        lat_lon: None,
        altitude: None,
        taken_at: None,
        direction: None,
    };
    let Ok(e) = Reader::new().read_from_container(&mut Cursor::new(bytes)) else {
        return none;
    };
    let field = |t: Tag| e.get_field(t, In::PRIMARY).map(|f| &f.value);
    let coord = |v: Tag, r: Tag| degrees(&rationals(field(v)?)?, &ascii(field(r)?)?);
    let lat_lon = coord(Tag::GPSLatitude, Tag::GPSLatitudeRef)
        .zip(coord(Tag::GPSLongitude, Tag::GPSLongitudeRef))
        .filter(|(la, lo)| la.abs() <= 90.0 && lo.abs() <= 180.0 && (*la, *lo) != (0.0, 0.0));
    let altitude = field(Tag::GPSAltitude)
        .and_then(rationals)
        .and_then(|v| v.first().copied())
        .map(|a| {
            // GPSAltitudeRef 1: unter dem Bezugsniveau
            let below =
                matches!(field(Tag::GPSAltitudeRef), Some(Value::Byte(b)) if b.first() == Some(&1));
            if below {
                -a
            } else {
                a
            }
        });
    let taken_at = field(Tag::DateTimeOriginal).and_then(ascii).and_then(|t| {
        exif_time(
            &t,
            field(Tag::OffsetTimeOriginal).and_then(ascii).as_deref(),
        )
    });
    let direction = field(Tag::GPSImgDirection)
        .and_then(rationals)
        .and_then(|v| v.first().copied())
        .filter(|d| d.is_finite() && (0.0..=360.0).contains(d))
        .map(|d| d % 360.0);
    Exif {
        lat_lon,
        altitude,
        taken_at,
        direction,
    }
}

/// Vorschaubild als JPEG, längste Seite [`THUMB`] Pixel (kleinere Fotos bleiben, wie sie sind)
pub fn thumbnail(bytes: &[u8]) -> Result<Vec<u8>> {
    let img = image::load_from_memory(bytes).context("Bild nicht lesbar")?;
    let small = if img.width().max(img.height()) > THUMB {
        img.thumbnail(THUMB, THUMB)
    } else {
        img
    };
    let mut out = Cursor::new(Vec::new());
    small.to_rgb8().write_to(&mut out, ImageFormat::Jpeg)?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Foto mit GPS, erzeugt mit Pillow (e2e/data/foto-gps.jpg): 48° 46′ 48,0708″ N, 9° 9′ 49,2676″ O – die
    /// Stelle E 512 025, N 5 403 015 im Testgelände –, 348,5 m, 17.11.2026 10:30 +01:00
    const PHOTO: &[u8] = include_bytes!("../../../../e2e/data/foto-gps.jpg");

    #[test]
    fn position_and_time_from_exif() {
        let e = read_exif(PHOTO);
        let (lat, lon) = e.lat_lon.unwrap();
        assert!(
            (lat - (48.0 + 46.0 / 60.0 + 48.0708 / 3600.0)).abs() < 1e-9,
            "{lat}"
        );
        assert!(
            (lon - (9.0 + 9.0 / 60.0 + 49.2676 / 3600.0)).abs() < 1e-9,
            "{lon}"
        );
        assert!((e.altitude.unwrap() - 348.5).abs() < 1e-9);
        // 10:30 MEZ = 09:30 UTC
        assert_eq!(e.taken_at, Some(1_794_907_800_000));
        // GPSImgDirection 1355/10, Bezug „T“ (geografisch)
        assert_eq!(e.direction, Some(135.5));
    }

    #[test]
    fn dms_and_times() {
        assert_eq!(degrees(&[48.0, 30.0, 36.0], "N"), Some(48.51));
        assert_eq!(degrees(&[9.0, 0.0, 0.0], "W"), Some(-9.0));
        assert_eq!(degrees(&[1.0], "N"), None);
        assert_eq!(
            exif_time("2026:11:17 09:30:00", None),
            Some(1_794_907_800_000)
        );
        assert_eq!(
            exif_time("2026:11:17 10:30:00", Some("+01:00")),
            Some(1_794_907_800_000)
        );
        assert_eq!(exif_time("kaputt", None), None);
    }

    #[test]
    fn photos_without_exif_and_thumbnails() {
        let mut png = Cursor::new(Vec::new());
        image::RgbImage::from_pixel(1200, 600, image::Rgb([200, 100, 50]))
            .write_to(&mut png, ImageFormat::Png)
            .unwrap();
        let png = png.into_inner();
        assert_eq!(
            read_exif(&png),
            Exif {
                lat_lon: None,
                altitude: None,
                taken_at: None,
                direction: None
            }
        );
        let t = image::load_from_memory(&thumbnail(&png).unwrap()).unwrap();
        assert_eq!((t.width(), t.height()), (480, 240));
        assert!(thumbnail(b"kein Bild").is_err());
    }
}
