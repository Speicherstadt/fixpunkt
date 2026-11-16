//! GeoTIFF lesen und schreiben (Höhenraster).
//!
//! Lesen: einkanalige Raster beliebigen Zahlentyps, unkomprimiert/LZW/Deflate, Streifen oder Kacheln –
//! z. B. `odm_dem/dsm.tif` / `dtm.tif` aus OpenDroneMap. Schreiben: Float32, Deflate, mit EPSG-Code.

use crate::grid::{Grid, GridSpec};
use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tiff::decoder::{Decoder, DecodingResult, Limits};
use tiff::encoder::{colortype::Gray32Float, compression::DeflateLevel, Compression, TiffEncoder};
use tiff::tags::Tag;

const NODATA: f32 = -9999.0;

const KEY_MODEL_TYPE: u16 = 1024;
const KEY_RASTER_TYPE: u16 = 1025;
const KEY_PROJECTED_CRS: u16 = 3072;
const KEY_VERTICAL_CRS: u16 = 4096;

/// Gelesenes Raster mit Bezugssystem aus den GeoKeys.
#[derive(Debug, Clone)]
pub struct GeoRaster {
    pub grid: Grid,
    /// EPSG-Code des Lagesystems (z. B. 25832), falls angegeben
    pub epsg: Option<u16>,
}

pub fn read(path: &Path) -> Result<GeoRaster> {
    let f = File::open(path).with_context(|| format!("{} nicht lesbar", path.display()))?;
    let mut d = Decoder::new(BufReader::new(f))
        .with_context(|| format!("{} ist kein TIFF", path.display()))?
        .with_limits(Limits::unlimited());
    let (w, h) = d.dimensions()?;
    let spp = d.find_tag_unsigned::<u16>(Tag::SamplesPerPixel)?.unwrap_or(1) as usize;
    let scale = d.find_tag(Tag::ModelPixelScaleTag)?.context("GeoTIFF ohne ModelPixelScale")?.into_f64_vec()?;
    let tie = d.find_tag(Tag::ModelTiepointTag)?.context("GeoTIFF ohne ModelTiepoint")?.into_f64_vec()?;
    if scale.len() < 2 || tie.len() < 6 {
        bail!("GeoTIFF: unvollständige Georeferenz");
    }
    let (sx, sy) = (scale[0], scale[1]);
    if ((sx - sy) / sx).abs() > 1e-6 {
        bail!("GeoTIFF: nur quadratische Zellen werden unterstützt ({sx} × {sy})");
    }
    let keys: Vec<u16> = match d.find_tag(Tag::GeoKeyDirectoryTag)? {
        Some(v) => v.into_u16_vec()?,
        None => Vec::new(),
    };
    let key = |id: u16| keys.chunks(4).skip(1).find(|k| k.len() == 4 && k[0] == id && k[1] == 0).map(|k| k[3]);
    let pixel_is_point = key(KEY_RASTER_TYPE) == Some(2);
    let epsg = key(KEY_PROJECTED_CRS).filter(|&e| e != 0 && e != 32767);
    let nodata: Option<f64> = d
        .find_tag(Tag::GdalNodata)?
        .and_then(|v| v.into_string().ok())
        .and_then(|s| s.trim_matches(char::from(0)).trim().parse().ok());

    let half = if pixel_is_point { sx / 2.0 } else { 0.0 };
    let min_x = tie[3] - tie[0] * sx - half;
    let max_y = tie[4] + tie[1] * sy + half;
    let spec = GridSpec { min_x, max_y, cell: sx, cols: w as usize, rows: h as usize };

    let raw: Vec<f64> = match d.read_image()? {
        DecodingResult::F32(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::F64(v) => v,
        DecodingResult::U8(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::U16(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::U32(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::I8(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::I16(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::I32(v) => v.into_iter().map(f64::from).collect(),
        _ => bail!("GeoTIFF: Datentyp nicht unterstützt"),
    };
    if raw.len() < spec.len() * spp {
        bail!("GeoTIFF: zu wenige Pixel");
    }
    let data = raw
        .iter()
        .step_by(spp)
        .take(spec.len())
        .map(|&v| {
            let bad = !v.is_finite() || nodata.is_some_and(|n| (v - n).abs() < 1e-6 * n.abs().max(1.0));
            if bad { f32::NAN } else { v as f32 }
        })
        .collect();
    Ok(GeoRaster { grid: Grid { spec, data }, epsg })
}

/// Schreibt ein Float32-GeoTIFF (Deflate). `epsg` z. B. 25832; `vertical_epsg` z. B. 7837 (DHHN2016).
pub fn write(g: &Grid, path: &Path, epsg: Option<u16>, vertical_epsg: Option<u16>) -> Result<()> {
    let f = File::create(path).with_context(|| format!("{} nicht schreibbar", path.display()))?;
    let mut enc = TiffEncoder::new(std::io::BufWriter::new(f))?.with_compression(Compression::Deflate(DeflateLevel::Balanced));
    let s = g.spec;
    let mut img = enc.new_image::<Gray32Float>(s.cols as u32, s.rows as u32)?;
    {
        let e = img.encoder();
        e.write_tag(Tag::ModelPixelScaleTag, &[s.cell, s.cell, 0.0][..])?;
        e.write_tag(Tag::ModelTiepointTag, &[0.0, 0.0, 0.0, s.min_x, s.max_y, 0.0][..])?;
        let mut keys: Vec<[u16; 4]> = vec![[KEY_MODEL_TYPE, 0, 1, 1], [KEY_RASTER_TYPE, 0, 1, 1]];
        if let Some(c) = epsg {
            keys.push([KEY_PROJECTED_CRS, 0, 1, c]);
        }
        if let Some(c) = vertical_epsg {
            keys.push([KEY_VERTICAL_CRS, 0, 1, c]);
        }
        let mut dir = vec![1u16, 1, 0, keys.len() as u16];
        dir.extend(keys.iter().flatten());
        e.write_tag(Tag::GeoKeyDirectoryTag, &dir[..])?;
        e.write_tag(Tag::GdalNodata, "-9999")?;
    }
    let data: Vec<f32> = g.data.iter().map(|v| if v.is_nan() { NODATA } else { *v }).collect();
    img.write_data(&data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_keeps_georef_nodata_and_epsg() {
        let spec = GridSpec { min_x: 512_000.0, max_y: 5_401_000.0, cell: 0.5, cols: 7, rows: 5 };
        let mut g = Grid::new_nodata(spec);
        for r in 0..5 {
            for c in 0..7 {
                g.set(c, r, 300.0 + c as f32 * 0.1 - r as f32 * 0.25);
            }
        }
        g.set(3, 2, f32::NAN);
        let p = std::env::temp_dir().join(format!("fixpunkt-{}.tif", std::process::id()));
        write(&g, &p, Some(25832), Some(7837)).unwrap();
        let back = read(&p).unwrap();
        std::fs::remove_file(&p).ok();
        assert_eq!(back.epsg, Some(25832));
        assert_eq!(back.grid.spec, spec);
        assert!(back.grid.get(3, 2).is_nan());
        assert_eq!(back.grid.get(6, 4), g.get(6, 4));
        assert_eq!(back.grid.valid_cells(), 34);
    }
}
