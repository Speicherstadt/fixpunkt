//! Oberflächen laden: Punktwolke (LAS/LAZ/XYZ, gerastert) oder fertiges Raster (GeoTIFF, ASC).

use crate::filter::{self, GroundParams};
use crate::geom::BBox;
use crate::grid::{Aggregate, Grid, GridSpec};
use crate::pointcloud::{PointCloud, ReadOptions};
use anyhow::{bail, Result};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct SurfaceOptions {
    /// Zellgröße beim Rastern von Punktwolken
    pub cell: f64,
    /// nur bereits als Boden klassifizierte Punkte (LAS-Klasse 2)
    pub ground_only: bool,
    /// Bodenfilter selbst rechnen (für unklassifizierte Wolken)
    pub classify_ground: Option<GroundParams>,
    pub aggregate: Aggregate,
    /// Lücken bis zu so vielen Zellen schließen
    pub fill: usize,
}

impl Default for SurfaceOptions {
    fn default() -> Self {
        SurfaceOptions { cell: 0.25, ground_only: false, classify_ground: None, aggregate: Aggregate::Mean, fill: 4 }
    }
}

pub fn is_raster(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(),
        Some("tif" | "tiff" | "asc")
    )
}

/// Liest nur die Raster-Datei (ohne Umrechnung).
pub fn read_raster(path: &Path) -> Result<Grid> {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("tif" | "tiff") => Ok(crate::geotiff::read(path)?.grid),
        Some("asc") => Grid::read_asc(path),
        _ => bail!("{} ist kein Raster (.tif, .asc)", path.display()),
    }
}

/// Liest eine Punktwolke. Mit `ground_only`, aber ohne einen einzigen Bodenpunkt in der Datei
/// (unklassifiziert), werden alle Punkte verwendet und ein Hinweis auf stderr ausgegeben.
pub fn read_cloud(path: &Path, o: &SurfaceOptions) -> Result<PointCloud> {
    if o.ground_only {
        let all = PointCloud::read(path, ReadOptions { ground_only: false })?;
        if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("las") || e.eq_ignore_ascii_case("laz")) {
            match PointCloud::read(path, ReadOptions { ground_only: true }) {
                Ok(g) => return Ok(g),
                Err(_) => eprintln!("Hinweis: {} enthält keine Bodenklasse – alle Punkte werden verwendet", path.display()),
            }
        }
        return Ok(all);
    }
    PointCloud::read(path, ReadOptions { ground_only: false })
}

/// Ausdehnung einer Eingabe (für Raster ohne Einlesen der Punkte nicht möglich → Punkte werden gelesen).
pub fn extent(path: &Path, o: &SurfaceOptions) -> Result<BBox> {
    if is_raster(path) {
        return Ok(read_raster(path)?.bbox());
    }
    Ok(read_cloud(path, o)?.bbox())
}

/// Lädt eine Oberfläche. Mit `spec` landet das Ergebnis genau auf diesem Gitter (Raster werden
/// bilinear übertragen), sonst auf einem Gitter über `bbox` bzw. die ganze Eingabe.
pub fn load(path: &Path, spec: Option<GridSpec>, bbox: Option<BBox>, o: &SurfaceOptions) -> Result<Grid> {
    if is_raster(path) {
        let g = read_raster(path)?;
        return Ok(match (spec, bbox) {
            (Some(s), _) => g.resample(s),
            (None, Some(b)) => g.resample(GridSpec::covering(&b, g.spec.cell)?),
            _ => g,
        });
    }
    let mut cloud = read_cloud(path, o)?;
    if let Some(p) = o.classify_ground {
        let (cls, _, _) = filter::classify_ground(&cloud, p)?;
        let keep: Vec<bool> = cls.iter().map(|&c| c == filter::CLASS_GROUND).collect();
        cloud = filter::select(&cloud, &keep);
        if cloud.is_empty() {
            bail!("{}: keine Bodenpunkte gefunden", path.display());
        }
    }
    let spec = match spec {
        Some(s) => s,
        None => GridSpec::covering(&bbox.unwrap_or_else(|| cloud.bbox()), o.cell)?,
    };
    let mut g = Grid::from_points(&cloud, spec, o.aggregate);
    g.fill_gaps(o.fill, None);
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pointcloud::Point3;

    #[test]
    fn cloud_and_raster_give_same_surface() {
        let pts: Vec<Point3> = (0..=80)
            .flat_map(|j| (0..=80).map(move |i| (i, j)))
            .map(|(i, j)| {
                let (dx, dy) = (i as f64 * 0.25, j as f64 * 0.25);
                Point3 { x: 500_000.0 + dx, y: 5_400_000.0 + dy, z: 200.0 + 0.1 * dx }
            })
            .collect();
        let dir = std::env::temp_dir().join(format!("fixpunkt-surface-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let las = dir.join("p.laz");
        PointCloud::new(pts).write(&las, None).unwrap();
        let o = SurfaceOptions { cell: 0.5, ..Default::default() };
        let g = load(&las, None, None, &o).unwrap();
        let tif = dir.join("g.tif");
        crate::geotiff::write(&g, &tif, Some(25832), None).unwrap();
        let b = BBox { min_x: 500_005.0, min_y: 5_400_005.0, max_x: 500_010.0, max_y: 5_400_010.0 };
        let spec = GridSpec::covering(&b, 0.5).unwrap();
        let a = load(&las, Some(spec), None, &o).unwrap();
        let r = load(&tif, Some(spec), None, &o).unwrap();
        assert!(is_raster(&tif) && !is_raster(&las));
        for i in 0..spec.len() {
            assert!((a.data[i] - r.data[i]).abs() < 0.01);
        }
        assert!((extent(&tif, &o).unwrap().min_x - 500_000.0).abs() < 1e-9);
        // unklassifiziert + ground_only → Rückfall auf alle Punkte
        let go = SurfaceOptions { ground_only: true, ..o };
        assert_eq!(read_cloud(&las, &go).unwrap().len(), 81 * 81);
        std::fs::remove_dir_all(&dir).ok();
    }
}
