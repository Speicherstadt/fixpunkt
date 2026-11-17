//! Orthofoto (RGB- oder RGBA-GeoTIFF, z. B. `odm_orthophoto.tif`) verkleinert lesen: Stück für Stück (Kacheln
//! oder Streifen), ohne das ganze Bild in den Speicher zu laden – ein Orthofoto mit 20 000 × 20 000 Pixeln
//! bräuchte sonst 1,6 GB. Jedes Zielpixel ist der Mittelwert der Quellpixel mit Daten (Alpha > 0).

use crate::geom::{BBox, Polygon};
use crate::geotiff::{self, Georef};
use anyhow::{bail, Context, Result};
use std::path::Path;
use tiff::decoder::DecodingResult;
use tiff::tags::{PlanarConfiguration, Tag};

/// Verkleinertes Orthofoto
#[derive(Debug, Clone)]
pub struct Ortho {
    pub width: u32,
    pub height: u32,
    /// RGBA, Zeile 0 = Norden; Alpha 0 ohne Daten
    pub rgba: Vec<u8>,
    /// Lage des verkleinerten Bildes (Pixelgröße = Quellpixel × Faktor)
    pub georef: Georef,
    /// Größe des Originals
    pub source: (u32, u32),
}

/// Lage eines Stücks im Bild: (Spalte, Zeile) der linken oberen Ecke
pub fn chunk_origin(index: u32, chunk: (u32, u32), image_width: u32) -> (u32, u32) {
    let across = image_width.div_ceil(chunk.0).max(1);
    ((index % across) * chunk.0, (index / across) * chunk.1)
}

/// Werte eines Stücks als 8 Bit
fn to_u8(r: DecodingResult) -> Result<Vec<u8>> {
    Ok(match r {
        DecodingResult::U8(v) => v,
        DecodingResult::U16(v) => v.into_iter().map(|x| (x >> 8) as u8).collect(),
        _ => bail!("Orthofoto: nur 8 oder 16 Bit je Farbe"),
    })
}

/// Liest `path` und verkleinert auf höchstens `max_side` Pixel je Seite
pub fn read_downsampled(path: &Path, max_side: u32) -> Result<Ortho> {
    read_window(path, None, max_side)
}

/// Pixelfenster [Spalte0, Zeile0, Spalte1, Zeile1) eines Ausschnitts im Bild; leer, wenn er danebenliegt
pub fn pixel_window(g: &Georef, (w, h): (u32, u32), b: &BBox) -> Option<[u32; 4]> {
    let col = |x: f64| ((x - g.min_x) / g.cell).clamp(0.0, f64::from(w));
    let row = |y: f64| ((g.max_y - y) / g.cell).clamp(0.0, f64::from(h));
    let (c0, c1) = (col(b.min_x).floor() as u32, col(b.max_x).ceil() as u32);
    let (r0, r1) = (row(b.max_y).floor() as u32, row(b.min_y).ceil() as u32);
    (c1 > c0 && r1 > r0).then_some([c0, r0, c1, r1])
}

/// Ausschnitt `window` (Lagekoordinaten des Orthofotos; `None`: alles), verkleinert auf höchstens `max_side`
/// Pixel je Seite; liest nur die Kacheln oder Streifen, die den Ausschnitt berühren
pub fn read_window(path: &Path, window: Option<&BBox>, max_side: u32) -> Result<Ortho> {
    let mut d = geotiff::open(path)?;
    let (w, h) = d.dimensions()?;
    let spp = d
        .find_tag_unsigned::<u16>(Tag::SamplesPerPixel)?
        .unwrap_or(1) as usize;
    if !(1..=4).contains(&spp) {
        bail!("Orthofoto: {spp} Kanäle nicht unterstützt");
    }
    let planar = d
        .find_tag_unsigned::<u16>(Tag::PlanarConfiguration)?
        .unwrap_or(1);
    if planar != PlanarConfiguration::Chunky.to_u16() {
        bail!("Orthofoto: Farben je Ebene getrennt (PlanarConfiguration 2) nicht unterstützt");
    }
    let georef = geotiff::georef(&mut d)?;
    let [c0, r0, c1, r1] = match window {
        Some(b) => {
            pixel_window(&georef, (w, h), b).context("Ausschnitt liegt außerhalb des Orthofotos")?
        }
        None => [0, 0, w, h],
    };
    let (ww, wh) = (c1 - c0, r1 - r0);
    let k = ww.max(wh).div_ceil(max_side.max(1)).max(1);
    let (ow, oh) = (ww.div_ceil(k), wh.div_ceil(k));
    // Summen je Zielpixel: r, g, b, Anzahl
    let mut acc = vec![0u32; (ow * oh * 4) as usize];
    let chunk = d.chunk_dimensions();
    let count = match d.get_chunk_type() {
        tiff::decoder::ChunkType::Tile => d.tile_count()?,
        tiff::decoder::ChunkType::Strip => d.strip_count()?,
    };
    for i in 0..count {
        let (cx, cy) = chunk_origin(i, chunk, w);
        let (dw, dh) = d.chunk_data_dimensions(i);
        // Stücke außerhalb des Fensters gar nicht erst entpacken
        if cx >= c1 || cy >= r1 || cx + dw <= c0 || cy + dh <= r0 {
            continue;
        }
        let v = to_u8(d.read_chunk(i)?)?;
        // Kacheln am Rand sind voll groß gespeichert, Streifen nur so breit wie das Bild
        let stride = if d.get_chunk_type() == tiff::decoder::ChunkType::Tile {
            chunk.0
        } else {
            dw
        } as usize;
        for py in 0..dh {
            for px in 0..dw {
                let (x, y) = (cx + px, cy + py);
                if x < c0 || y < r0 || x >= c1 || y >= r1 {
                    continue;
                }
                let (x, y) = (x - c0, y - r0);
                let s = (py as usize * stride + px as usize) * spp;
                let Some(p) = v.get(s..s + spp) else { continue };
                let (r, g, b, a) = match spp {
                    1 => (p[0], p[0], p[0], 255),
                    2 => (p[0], p[0], p[0], p[1]),
                    3 => (p[0], p[1], p[2], 255),
                    _ => (p[0], p[1], p[2], p[3]),
                };
                if a == 0 {
                    continue;
                }
                let o = (((y / k) * ow + x / k) * 4) as usize;
                acc[o] += u32::from(r);
                acc[o + 1] += u32::from(g);
                acc[o + 2] += u32::from(b);
                acc[o + 3] += 1;
            }
        }
    }
    let rgba = acc
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|a| match a[3] {
            0 => [0, 0, 0, 0],
            n => [(a[0] / n) as u8, (a[1] / n) as u8, (a[2] / n) as u8, 255],
        })
        .collect();
    Ok(Ortho {
        width: ow,
        height: oh,
        rgba,
        georef: Georef {
            min_x: georef.min_x + f64::from(c0) * georef.cell,
            max_y: georef.max_y - f64::from(r0) * georef.cell,
            cell: georef.cell * f64::from(k),
            ..georef
        },
        source: (w, h),
    })
}

/// Auf Bereiche zuschneiden (Lagekoordinaten des Orthofotos): Ausschnitt um alle Bereiche, außerhalb durchsichtig
pub fn clip(path: &Path, areas: &[Polygon], max_side: u32) -> Result<Ortho> {
    let mut b = BBox::empty();
    for a in areas {
        let ab = a.bbox();
        b.extend(ab.min_x, ab.min_y);
        b.extend(ab.max_x, ab.max_y);
    }
    let mut o = read_window(path, (!areas.is_empty()).then_some(&b), max_side)?;
    if areas.is_empty() {
        return Ok(o);
    }
    let g = o.georef;
    for row in 0..o.height {
        for col in 0..o.width {
            let x = g.min_x + (f64::from(col) + 0.5) * g.cell;
            let y = g.max_y - (f64::from(row) + 0.5) * g.cell;
            if !areas.iter().any(|a| a.contains(x, y)) {
                let i = ((row * o.width + col) * 4) as usize;
                o.rgba[i..i + 4].fill(0);
            }
        }
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiff::encoder::{colortype, TiffEncoder};

    /// RGBA-GeoTIFF wie aus ODM: links rot, rechts blau, ab Zeile `empty_from` ohne Daten; 0,05 m Pixel
    fn write_ortho(path: &Path, w: u32, h: u32, empty_from: u32, epsg: u16) {
        let mut px = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let a = if y >= empty_from { 0 } else { 255 };
                let rgba: [u8; 4] = if x < w / 2 {
                    [200, 20, 20, a]
                } else {
                    [20, 20, 200, a]
                };
                px.extend_from_slice(&rgba);
            }
        }
        let f = std::fs::File::create(path).unwrap();
        let mut e = TiffEncoder::new(f).unwrap();
        let mut img = e.new_image::<colortype::RGBA8>(w, h).unwrap();
        let enc = img.encoder();
        enc.write_tag(Tag::ModelPixelScaleTag, &[0.05, 0.05, 0.0][..])
            .unwrap();
        enc.write_tag(
            Tag::ModelTiepointTag,
            &[0.0, 0.0, 0.0, 512_000.0, 5_403_040.0, 0.0][..],
        )
        .unwrap();
        enc.write_tag(
            Tag::GeoKeyDirectoryTag,
            &[1u16, 1, 0, 2, 1024, 0, 1, 1, 3072, 0, 1, epsg][..],
        )
        .unwrap();
        img.rows_per_strip(7).unwrap();
        img.write_data(&px).unwrap();
    }

    #[test]
    fn downsampled_in_strips_with_transparency() {
        let dir = std::env::temp_dir().join(format!("fixpunkt-ortho-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("ortho.tif");
        // 800 × 801 Pixel = 40 × 40,05 m, auf höchstens 100 → Faktor 9
        write_ortho(&p, 800, 801, 792, 25832);
        let o = read_downsampled(&p, 100).unwrap();
        assert_eq!((o.width, o.height), (89, 89));
        assert_eq!(o.source, (800, 801));
        assert_eq!(o.georef.epsg, Some(25832));
        assert!((o.georef.cell - 0.45).abs() < 1e-12);
        assert_eq!((o.georef.min_x, o.georef.max_y), (512_000.0, 5_403_040.0));
        let at = |x: u32, y: u32| &o.rgba[((y * o.width + x) * 4) as usize..][..4];
        assert_eq!(at(0, 0), &[200, 20, 20, 255]);
        assert_eq!(at(88, 0), &[20, 20, 200, 255]);
        // Zielzeile 88 = Quellzeilen 792 … 800, alle ohne Daten → durchsichtig; Zeile 87 hat Daten
        assert_eq!(at(10, 88)[3], 0);
        assert_eq!(at(10, 87)[3], 255);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn window_mask_and_geotiff_roundtrip() {
        // 200 × 100 Pixel à 5 cm ab (512 000 | 5 403 040): links rot, rechts blau (ab x = 512 005)
        let dir = std::env::temp_dir().join(format!("fixpunkt-ortho-clip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("o.tif");
        write_ortho(&src, 200, 100, 100, 25832);

        // Ausschnitt 2 × 1 m rechts oben: 40 × 20 Pixel, Ecke genau auf dem Raster
        let b = BBox {
            min_x: 512_006.0,
            min_y: 5_403_038.0,
            max_x: 512_008.0,
            max_y: 5_403_039.0,
        };
        let g = geotiff::georef(&mut geotiff::open(&src).unwrap()).unwrap();
        assert_eq!(pixel_window(&g, (200, 100), &b), Some([120, 20, 160, 40]));
        let w = read_window(&src, Some(&b), 4096).unwrap();
        assert_eq!((w.width, w.height), (40, 20));
        assert!(
            (w.georef.min_x - 512_006.0).abs() < 1e-9
                && (w.georef.max_y - 5_403_039.0).abs() < 1e-9
        );
        assert!(
            w.rgba.chunks(4).all(|p| p == [20, 20, 200, 255]),
            "nur Blau"
        );
        assert!(pixel_window(
            &g,
            (200, 100),
            &BBox {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 1.0,
                max_y: 1.0
            }
        )
        .is_none());

        // Dreieck über die Grenze rot/blau: außerhalb durchsichtig, innen beide Farben
        let tri = Polygon::new(vec![
            [512_004.0, 5_403_036.0],
            [512_006.0, 5_403_036.0],
            [512_005.0, 5_403_038.0],
        ])
        .unwrap();
        let c = clip(&src, std::slice::from_ref(&tri), 4096).unwrap();
        assert_eq!((c.width, c.height), (40, 40));
        let px: Vec<&[u8]> = c.rgba.chunks(4).collect();
        let inside = px.iter().filter(|p| p[3] == 255).count();
        // Fläche 2 m² von 4 m² Ausschnitt: etwa die Hälfte der 1600 Pixel
        assert!((inside as i64 - 800).abs() < 40, "{inside}");
        assert!(
            px.iter().any(|p| p[..3] == [200, 20, 20])
                && px.iter().any(|p| p[..3] == [20, 20, 200])
        );
        // Ecken des Ausschnitts liegen außerhalb des Dreiecks
        assert_eq!(px[0][3], 0);

        // als GeoTIFF geschrieben und wieder gelesen: gleiche Lage, gleiche Pixel
        let out = dir.join("clip.tif");
        geotiff::write_rgba(&out, c.width, c.height, &c.rgba, &c.georef).unwrap();
        let back = read_downsampled(&out, 4096).unwrap();
        assert_eq!(back.georef, c.georef);
        assert_eq!(back.rgba, c.rgba);
    }

    #[test]
    fn chunk_positions() {
        assert_eq!(chunk_origin(0, (256, 256), 600), (0, 0));
        assert_eq!(chunk_origin(2, (256, 256), 600), (512, 0));
        assert_eq!(chunk_origin(3, (256, 256), 600), (0, 256));
        // Streifen: so breit wie das Bild
        assert_eq!(chunk_origin(4, (600, 7), 600), (0, 28));
    }
}
