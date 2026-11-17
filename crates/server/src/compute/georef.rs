//! Georeferenzierung über Passpunkte: Transformation bestimmen (Helmert nach Umeyama, im Rechenkern) und auf
//! die Punktwolke anwenden. Das Ergebnis wird eine neue Aufnahme; die alte bleibt unverändert.

use anyhow::{bail, Result};
use fixpunkt::transform::{self, ControlPoint, FitReport, Mode};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Passpunkt, wie die UI ihn schickt: Koordinate in der Aufnahme (`src`) und Sollkoordinate (`dst`)
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Gcp {
    pub name: String,
    pub src: [f64; 3],
    pub dst: [f64; 3],
}

fn control_points(points: &[Gcp]) -> Result<Vec<ControlPoint>> {
    if points
        .iter()
        .any(|p| p.src.iter().chain(&p.dst).any(|v| !v.is_finite()))
    {
        bail!("Koordinaten müssen Zahlen sein");
    }
    Ok(points
        .iter()
        .map(|p| ControlPoint {
            name: p.name.clone(),
            src: p.src,
            dst: p.dst,
        })
        .collect())
}

/// `POST /api/georef/fit`: Vorschau beim Eingeben der Passpunkte
#[derive(Debug, Deserialize)]
pub struct FitRequest {
    #[serde(default)]
    pub mode: Mode,
    pub points: Vec<Gcp>,
}

pub async fn fit_handler(
    axum::Json(r): axum::Json<FitRequest>,
) -> crate::error::ApiResult<axum::Json<FitReport>> {
    fit(&r.points, r.mode).map(axum::Json).map_err(|e| {
        crate::error::ApiError::bad_request(crate::error::code::GEOREF_INVALID)
            .with_detail(format!("{e:#}"))
    })
}

/// Nur bestimmen, nichts anwenden – für die Vorschau beim Eingeben
pub fn fit(points: &[Gcp], mode: Mode) -> Result<FitReport> {
    transform::fit(&control_points(points)?, mode)
}

/// Name der neuen Datei: LAS/LAZ behalten ihr Format, Text wird LAZ
pub fn output_name(input: &Path) -> String {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("aufnahme");
    match input.extension().and_then(|e| e.to_str()) {
        Some(e) if e.eq_ignore_ascii_case("las") || e.eq_ignore_ascii_case("laz") => {
            format!("{stem}-georef.{}", e.to_ascii_lowercase())
        }
        _ => format!("{stem}-georef.laz"),
    }
}

/// Transformation bestimmen und auf `input` anwenden; schreibt nach `out_dir`
pub fn georef(
    input: &Path,
    points: &[Gcp],
    mode: Mode,
    out_dir: &Path,
) -> Result<(FitReport, PathBuf)> {
    if fixpunkt::surface::is_raster(input) {
        bail!("Georeferenzieren geht nur mit Punktwolken");
    }
    let report = fit(points, mode)?;
    std::fs::create_dir_all(out_dir)?;
    let out = out_dir.join(output_name(input));
    transform::apply_to_file(&report.transform, input, &out)?;
    Ok((report, out))
}

#[cfg(test)]
mod tests {
    use super::super::testdata::{write_cloud, X0, Y0};
    use super::*;
    use fixpunkt::pointcloud::{PointCloud, ReadOptions};

    /// bekannte Verschiebung: Scan liegt 100 m östlich, 50 m südlich und 2 m zu hoch
    const SHIFT: [f64; 3] = [100.0, -50.0, 2.0];

    fn gcps() -> Vec<Gcp> {
        [
            (0.0, 0.0, 300.0),
            (30.0, 0.0, 300.3),
            (30.0, 30.0, 300.3),
            (0.0, 30.0, 300.0),
        ]
        .iter()
        .enumerate()
        .map(|(i, &(dx, dy, z))| {
            let dst = [X0 + dx, Y0 + dy, z];
            Gcp {
                name: format!("P{}", i + 1),
                src: [dst[0] + SHIFT[0], dst[1] + SHIFT[1], dst[2] + SHIFT[2]],
                dst,
            }
        })
        .collect()
    }

    #[test]
    fn shift_is_found_and_applied() {
        let r = fit(&gcps(), Mode::Translation).unwrap();
        for (t, s) in r.translation.iter().zip(SHIFT) {
            assert!((t + s).abs() < 1e-9, "{:?}", r.translation);
        }
        assert!(r.rms_3d < 1e-9);

        let dir = tempfile::tempdir().unwrap();
        let scan = dir.path().join("scan.laz");
        write_cloud(&scan, false);
        let truth = PointCloud::read(&scan, ReadOptions::default()).unwrap();
        // Scan verschieben wie im Feld gemessen, dann zurück über die Passpunkte
        let shifted = dir.path().join("verschoben.laz");
        let back = fit(
            &gcps()
                .into_iter()
                .map(|g| Gcp {
                    src: g.dst,
                    dst: g.src,
                    ..g
                })
                .collect::<Vec<_>>(),
            Mode::Translation,
        )
        .unwrap();
        transform::apply_to_file(&back.transform, &scan, &shifted).unwrap();
        let (_, out) =
            georef(&shifted, &gcps(), Mode::Similarity, &dir.path().join("job")).unwrap();
        assert_eq!(out.file_name().unwrap(), "verschoben-georef.laz");
        let got = PointCloud::read(&out, ReadOptions::default()).unwrap();
        assert_eq!(got.len(), truth.len());
        for (g, t) in got.points.iter().zip(&truth.points).step_by(97) {
            assert!(
                (g.x - t.x).abs() < 0.002 && (g.y - t.y).abs() < 0.002 && (g.z - t.z).abs() < 0.002
            );
        }
    }

    #[test]
    fn names_and_bad_input() {
        assert_eq!(output_name(Path::new("a/flug.LAS")), "flug-georef.las");
        assert_eq!(output_name(Path::new("flug.xyz")), "flug-georef.laz");
        let mut p = gcps();
        p[0].src[0] = f64::NAN;
        assert!(fit(&p, Mode::Similarity).is_err());
        assert!(
            fit(&gcps()[..2], Mode::Similarity).is_err(),
            "Helmert braucht 3 Punkte"
        );
    }
}
