//! Orthofoto (RGB- oder RGBA-GeoTIFF, z. B. `odm_orthophoto.tif`) verkleinert lesen: Stück für Stück (Kacheln
//! oder Streifen), ohne das ganze Bild in den Speicher zu laden – ein Orthofoto mit 20 000 × 20 000 Pixeln
//! bräuchte sonst 1,6 GB. Jedes Zielpixel ist der Mittelwert der Quellpixel mit Daten (Alpha > 0).

use crate::geotiff::{self, Georef};
use anyhow::{bail, Result};
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
    let k = w.max(h).div_ceil(max_side.max(1)).max(1);
    let (ow, oh) = (w.div_ceil(k), h.div_ceil(k));
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
                if x >= w || y >= h {
                    continue;
                }
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
            cell: georef.cell * f64::from(k),
            ..georef
        },
        source: (w, h),
    })
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
    fn chunk_positions() {
        assert_eq!(chunk_origin(0, (256, 256), 600), (0, 0));
        assert_eq!(chunk_origin(2, (256, 256), 600), (512, 0));
        assert_eq!(chunk_origin(3, (256, 256), 600), (0, 256));
        // Streifen: so breit wie das Bild
        assert_eq!(chunk_origin(4, (600, 7), 600), (0, 28));
    }
}
