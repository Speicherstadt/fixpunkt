//! Punktwolke für die 3D-Ansicht: gleichmäßig ausgedünnt auf höchstens [`MAX_POINTS`], Lage als Float32 relativ zu
//! einem Ursprung (sonst frisst Float32 bei 500 000 m die Zentimeter), danach RGB und Intensität (u16), falls die
//! Datei welche hat. Anzahl und Farbe in den Kopfzeilen `x-points-*`. Einmal gerechnet, liegt das Ergebnis als
//! `punkte-2.bin` bei der Aufnahme (Format 2: mit Intensität).

use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::jobs::capture_path;
use crate::state::AppRef;
use anyhow::{bail, Result};
use axum::extract::{Path, State};
use axum::http::header::{self, HeaderName};
use axum::response::{IntoResponse, Response};
use fixpunkt::colored::{read_colored, thin, Colored};
use fixpunkt::pointcloud::ReadOptions;
use std::path::Path as FsPath;

/// Mehr Punkte zeichnet ein Tablet nicht flüssig (15 MB)
pub const MAX_POINTS: usize = 1_000_000;
pub const CONTENT_TYPE: &str = "application/x-fixpunkt-points";
const CACHE: &str = "punkte-2.bin";
/// Merker im Kopf
pub const HAS_RGB: u8 = 1;
pub const HAS_INTENSITY: u8 = 2;

/// Kopf: Anzahl (u32), Merker (u8: 1 Farbe, 2 Intensität), 3 × Ursprung (f64); dann Lage, Farben, Intensität
pub fn encode(c: &Colored) -> Vec<u8> {
    let pts = &c.cloud.points;
    let b = c.cloud.bbox();
    let z0 = c.cloud.z_range().map_or(0.0, |r| r.0);
    let origin = [(b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0, z0];
    let mut out = Vec::with_capacity(29 + pts.len() * 17);
    out.extend_from_slice(&(pts.len() as u32).to_le_bytes());
    let flags = if c.colors.is_some() { HAS_RGB } else { 0 }
        | if c.intensity.is_some() {
            HAS_INTENSITY
        } else {
            0
        };
    out.push(flags);
    for o in origin {
        out.extend_from_slice(&o.to_le_bytes());
    }
    for p in pts {
        for v in [p.x - origin[0], p.y - origin[1], p.z - origin[2]] {
            out.extend_from_slice(&(v as f32).to_le_bytes());
        }
    }
    if let Some(rgb) = &c.colors {
        out.extend(rgb.iter().flatten());
    }
    if let Some(i) = &c.intensity {
        for v in i {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    out
}

/// Kopf lesen: (Anzahl, Merker, Ursprung)
pub fn header(bytes: &[u8]) -> Result<(usize, u8, [f64; 3])> {
    if bytes.len() < 29 {
        bail!("Punkte: Datei zu kurz");
    }
    let n = u32::from_le_bytes(bytes[0..4].try_into()?) as usize;
    let flags = bytes[4];
    let f = |i: usize| -> Result<f64> {
        Ok(f64::from_le_bytes(bytes[5 + 8 * i..13 + 8 * i].try_into()?))
    };
    let rgb = if flags & HAS_RGB != 0 { n * 3 } else { 0 };
    let intensity = if flags & HAS_INTENSITY != 0 { n * 2 } else { 0 };
    if flags > HAS_RGB | HAS_INTENSITY || bytes.len() != 29 + n * 12 + rgb + intensity {
        bail!("Punkte: Länge passt nicht");
    }
    Ok((n, flags, [f(0)?, f(1)?, f(2)?]))
}

fn make(path: &FsPath, dir: &FsPath) -> Result<Vec<u8>> {
    let cache = dir.join(CACHE);
    if let Ok(b) = std::fs::read(&cache) {
        if header(&b).is_ok() {
            return Ok(b);
        }
    }
    let c = read_colored(path, ReadOptions::default())?;
    let bytes = encode(&thin(&c, MAX_POINTS));
    std::fs::write(&cache, &bytes)?;
    Ok(bytes)
}

/// `GET /api/captures/{id}/points`; Höhenmodelle haben keine Punkte (404)
pub async fn capture_points(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<Response> {
    let c = db::get_capture(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    if fixpunkt::surface::is_raster(&capture_path(&app, &c)) {
        return Err(ApiError::not_found());
    }
    let (path, dir) = (capture_path(&app, &c), app.cfg.capture_dir(id));
    let bytes = tokio::task::spawn_blocking(move || make(&path, &dir))
        .await
        .map_err(anyhow::Error::from)??;
    let (n, flags, _) = header(&bytes)?;
    Ok((
        [
            (header::CONTENT_TYPE, CONTENT_TYPE.to_string()),
            (HeaderName::from_static("x-points-count"), n.to_string()),
            (
                HeaderName::from_static("x-points-colors"),
                (flags & HAS_RGB).to_string(),
            ),
            (header::CACHE_CONTROL, "private, max-age=86400".to_string()),
        ],
        bytes,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixpunkt::pointcloud::{Point3, PointCloud};

    #[test]
    fn roundtrip_relative_to_origin() {
        let pts = vec![
            Point3 {
                x: 512_000.0,
                y: 5_403_000.0,
                z: 300.0,
            },
            Point3 {
                x: 512_040.0,
                y: 5_403_040.0,
                z: 302.5,
            },
        ];
        let c = Colored {
            cloud: PointCloud::new(pts),
            colors: Some(vec![[1, 2, 3], [4, 5, 6]]),
            intensity: None,
        };
        let b = encode(&c);
        let (n, flags, origin) = header(&b).unwrap();
        assert_eq!((n, flags), (2, HAS_RGB));
        assert_eq!(origin, [512_020.0, 5_403_020.0, 300.0]);
        let f = |i: usize| f32::from_le_bytes(b[29 + 4 * i..33 + 4 * i].try_into().unwrap());
        // zweiter Punkt: +20 / +20 / +2,5 m – mm-genau trotz Float32
        assert_eq!([f(3), f(4), f(5)], [20.0, 20.0, 2.5]);
        assert_eq!(&b[b.len() - 6..], &[1, 2, 3, 4, 5, 6]);
        let with_i = encode(&Colored {
            intensity: Some(vec![0, 65535]),
            ..c.clone()
        });
        assert_eq!(header(&with_i).unwrap().1, HAS_RGB | HAS_INTENSITY);
        // hinter den Farben: 0 und 65535 als u16 little-endian
        assert_eq!(&with_i[with_i.len() - 4..], &[0, 0, 0xff, 0xff]);
        let plain = encode(&Colored { colors: None, ..c });
        assert_eq!(header(&plain).unwrap().1, 0);
        assert!(header(&plain[..30]).is_err());
    }
}
