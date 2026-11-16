//! Punktwolken aufbereiten: ausdünnen, Ausreißer entfernen, Boden klassifizieren.
//!
//! Der Bodenfilter folgt dem *Simple Morphological Filter* (SMRF, Pingel et al. 2013):
//! Minimum-Raster → morphologische Öffnung mit wachsendem Fenster → alles, was stärker als
//! die zulässige Geländeneigung hervorsteht (Bagger, Container, Bewuchs), gilt als Objekt.
//! Damit lassen sich auch unklassifizierte Drohnenwolken für die Aushubberechnung nutzen.

use crate::grid::{Aggregate, Grid, GridSpec};
use crate::pointcloud::{Point3, PointCloud};
use anyhow::{bail, Result};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;

/// LAS-Klassen (ASPRS)
pub const CLASS_UNCLASSIFIED: u8 = 1;
pub const CLASS_GROUND: u8 = 2;
pub const CLASS_LOW_NOISE: u8 = 7;

/// Behält je Voxel (Würfel mit Kantenlänge `size`) den Punkt, der der Voxelmitte am nächsten liegt.
/// Gibt die Indizes der behaltenen Punkte in Originalreihenfolge zurück.
pub fn voxel_thin(cloud: &PointCloud, size: f64) -> Result<Vec<usize>> {
    if !(size > 0.0) {
        bail!("Voxelgröße muss > 0 sein");
    }
    let mut best: HashMap<(i64, i64, i64), (usize, f64)> = HashMap::new();
    for (i, p) in cloud.points.iter().enumerate() {
        let k = ((p.x / size).floor() as i64, (p.y / size).floor() as i64, (p.z / size).floor() as i64);
        let c = |v: i64| (v as f64 + 0.5) * size;
        let d = (p.x - c(k.0)).powi(2) + (p.y - c(k.1)).powi(2) + (p.z - c(k.2)).powi(2);
        best.entry(k).and_modify(|e| if d < e.1 { *e = (i, d) }).or_insert((i, d));
    }
    let mut keep: Vec<usize> = best.into_values().map(|(i, _)| i).collect();
    keep.sort_unstable();
    Ok(keep)
}

/// Räumlicher Hash für Nachbarschaftssuchen.
struct SpatialHash<'a> {
    pts: &'a [Point3],
    h: f64,
    cells: HashMap<(i64, i64, i64), Vec<u32>>,
}

impl<'a> SpatialHash<'a> {
    fn new(pts: &'a [Point3], h: f64) -> Self {
        let mut cells: HashMap<(i64, i64, i64), Vec<u32>> = HashMap::new();
        for (i, p) in pts.iter().enumerate() {
            cells.entry(Self::key(p, h)).or_default().push(i as u32);
        }
        SpatialHash { pts, h, cells }
    }

    fn key(p: &Point3, h: f64) -> (i64, i64, i64) {
        ((p.x / h).floor() as i64, (p.y / h).floor() as i64, (p.z / h).floor() as i64)
    }

    /// Mittlere Distanz zu den `k` nächsten Nachbarn (ohne den Punkt selbst).
    fn mean_knn_distance(&self, i: usize, k: usize) -> f64 {
        let p = &self.pts[i];
        let (cx, cy, cz) = Self::key(p, self.h);
        let mut d: Vec<f64> = Vec::new();
        for ring in 0i64..64 {
            for dx in -ring..=ring {
                for dy in -ring..=ring {
                    for dz in -ring..=ring {
                        if dx.abs().max(dy.abs()).max(dz.abs()) != ring {
                            continue;
                        }
                        if let Some(v) = self.cells.get(&(cx + dx, cy + dy, cz + dz)) {
                            for &j in v {
                                if j as usize != i {
                                    let q = &self.pts[j as usize];
                                    d.push(((p.x - q.x).powi(2) + (p.y - q.y).powi(2) + (p.z - q.z).powi(2)).sqrt());
                                }
                            }
                        }
                    }
                }
            }
            // Alles innerhalb von ring·h ist vollständig durchsucht
            if d.len() >= k {
                d.sort_by(f64::total_cmp);
                if d[k - 1] <= ring as f64 * self.h {
                    break;
                }
            }
        }
        d.sort_by(f64::total_cmp);
        let n = k.min(d.len());
        if n == 0 {
            return f64::INFINITY;
        }
        d[..n].iter().sum::<f64>() / n as f64
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OutlierReport {
    pub points: usize,
    pub removed: usize,
    pub mean_knn_distance: f64,
    pub threshold: f64,
}

/// Statistischer Ausreißerfilter: entfernt Punkte, deren mittlere Distanz zu den `k` nächsten
/// Nachbarn größer ist als Mittelwert + `std_ratio` · Standardabweichung über alle Punkte.
/// Gibt eine Maske zurück (`true` = behalten).
pub fn statistical_outliers(cloud: &PointCloud, k: usize, std_ratio: f64) -> Result<(Vec<bool>, OutlierReport)> {
    if k == 0 {
        bail!("k muss ≥ 1 sein");
    }
    let n = cloud.len();
    if n <= k {
        bail!("zu wenige Punkte für k = {k}");
    }
    // Zellgröße so, dass im Mittel etwa k Punkte in einer Zelle liegen (über die Grundfläche)
    let b = cloud.bbox();
    let area = (b.width() * b.height()).max(1e-6);
    let h = (area * k as f64 / n as f64).sqrt().max(1e-3);
    let hash = SpatialHash::new(&cloud.points, h);
    let md: Vec<f64> = (0..n).into_par_iter().map(|i| hash.mean_knn_distance(i, k)).collect();
    let finite: Vec<f64> = md.iter().copied().filter(|v| v.is_finite()).collect();
    let mean = finite.iter().sum::<f64>() / finite.len() as f64;
    let std = (finite.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / finite.len() as f64).sqrt();
    let threshold = mean + std_ratio * std;
    let keep: Vec<bool> = md.iter().map(|&d| d <= threshold).collect();
    let removed = keep.iter().filter(|k| !**k).count();
    Ok((keep, OutlierReport { points: n, removed, mean_knn_distance: mean, threshold }))
}

/// Parameter des Bodenfilters (Standardwerte für Baustellen/Drohne).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct GroundParams {
    /// Zellgröße des Minimum-Rasters in m
    pub cell: f64,
    /// maximale Geländeneigung (Höhe/Strecke), Böschungen 1:1,5 ≈ 0,67
    pub slope: f64,
    /// größtes Objekt (Fensterradius) in m – etwas größer als der größte Bagger/Container
    pub max_window: f64,
    /// Höhentoleranz in m für die Punktklassifizierung
    pub elevation_threshold: f64,
    /// Zusatz-Toleranz je Einheit Geländeneigung
    pub elevation_scalar: f64,
}

impl Default for GroundParams {
    fn default() -> Self {
        GroundParams { cell: 1.0, slope: 0.7, max_window: 10.0, elevation_threshold: 0.3, elevation_scalar: 1.0 }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GroundReport {
    pub points: usize,
    pub ground: usize,
    pub non_ground: usize,
    pub low_noise: usize,
    pub params: GroundParams,
}

/// Klassifiziert jeden Punkt als Boden (2), Nicht-Boden (1) oder Tiefpunkt/Rauschen (7).
/// Liefert zusätzlich das vorläufige Gelände-Raster (DGM) in Zellgröße `params.cell`.
pub fn classify_ground(cloud: &PointCloud, params: GroundParams) -> Result<(Vec<u8>, Grid, GroundReport)> {
    if cloud.is_empty() {
        bail!("leere Punktwolke");
    }
    let spec = GridSpec::covering(&cloud.bbox(), params.cell)?;
    let mut zmin = Grid::from_points(cloud, spec, Aggregate::Min);
    zmin.fill_gaps(spec.cols + spec.rows, None);

    let max_r = (params.max_window / params.cell).ceil().max(1.0) as usize;
    let mut last = zmin.data.clone();
    let mut object = vec![false; spec.len()];
    for r in 1..=max_r {
        let opened = dilate(&erode(&zmin.data, spec, r), spec, r);
        let thr = (params.slope * r as f64 * params.cell) as f32;
        for i in 0..spec.len() {
            if last[i] - opened[i] > thr {
                object[i] = true;
            }
        }
        last = opened;
    }
    let mut dtm = zmin.clone();
    for (v, &o) in dtm.data.iter_mut().zip(&object) {
        if o {
            *v = f32::NAN;
        }
    }
    dtm.fill_gaps(spec.cols + spec.rows, None);
    if dtm.valid_cells() == 0 {
        bail!("kein Boden gefunden – Parameter prüfen (slope, max_window)");
    }
    let slope = slope_grid(&dtm);

    let classes: Vec<u8> = cloud
        .points
        .par_iter()
        .map(|p| {
            let Some(g) = dtm.sample(p.x, p.y) else { return CLASS_UNCLASSIFIED };
            let s = spec.locate(p.x, p.y).map(|(c, r)| slope[r * spec.cols + c]).unwrap_or(0.0);
            let tol = params.elevation_threshold + params.elevation_scalar * s * params.cell;
            let dz = p.z - g;
            if dz < -tol - 0.5 * params.max_window * params.slope {
                CLASS_LOW_NOISE
            } else if dz.abs() <= tol {
                CLASS_GROUND
            } else {
                CLASS_UNCLASSIFIED
            }
        })
        .collect();
    let count = |c: u8| classes.iter().filter(|&&k| k == c).count();
    let rep = GroundReport {
        points: cloud.len(),
        ground: count(CLASS_GROUND),
        non_ground: count(CLASS_UNCLASSIFIED),
        low_noise: count(CLASS_LOW_NOISE),
        params,
    };
    Ok((classes, dtm, rep))
}

/// Gefälle (Betrag, Höhe/Strecke) je Zelle aus zentralen Differenzen.
pub fn slope_grid(g: &Grid) -> Vec<f64> {
    let s = g.spec;
    let at = |c: i64, r: i64| -> Option<f64> {
        if c < 0 || r < 0 || c >= s.cols as i64 || r >= s.rows as i64 {
            return None;
        }
        let v = g.get(c as usize, r as usize);
        (!v.is_nan()).then_some(v as f64)
    };
    let mut out = vec![0.0; s.len()];
    for r in 0..s.rows as i64 {
        for c in 0..s.cols as i64 {
            let Some(z) = at(c, r) else { continue };
            let d = |a: Option<f64>, b: Option<f64>| match (a, b) {
                (Some(a), Some(b)) => (b - a) / (2.0 * s.cell),
                (Some(a), None) => (z - a) / s.cell,
                (None, Some(b)) => (b - z) / s.cell,
                _ => 0.0,
            };
            let gx = d(at(c - 1, r), at(c + 1, r));
            let gy = d(at(c, r + 1), at(c, r - 1));
            out[(r * s.cols as i64 + c) as usize] = (gx * gx + gy * gy).sqrt();
        }
    }
    out
}

/// Quadratisches Fenster (Radius r Zellen), getrennt nach Zeilen und Spalten; NaN wird ignoriert.
fn window(data: &[f32], spec: GridSpec, r: usize, pick: fn(f32, f32) -> f32) -> Vec<f32> {
    let (cols, rows) = (spec.cols, spec.rows);
    let pass = |src: &[f32], horizontal: bool| -> Vec<f32> {
        let (n_lines, len) = if horizontal { (rows, cols) } else { (cols, rows) };
        let idx = move |line: usize, k: usize| if horizontal { line * cols + k } else { k * cols + line };
        let lines: Vec<Vec<f32>> = (0..n_lines)
            .into_par_iter()
            .map(|line| {
                (0..len)
                    .map(|k| {
                        let (a, b) = (k.saturating_sub(r), (k + r).min(len - 1));
                        (a..=b).map(|j| src[idx(line, j)]).filter(|v| !v.is_nan()).reduce(pick).unwrap_or(f32::NAN)
                    })
                    .collect()
            })
            .collect();
        let mut out = vec![f32::NAN; src.len()];
        for (line, vals) in lines.into_iter().enumerate() {
            for (k, v) in vals.into_iter().enumerate() {
                out[idx(line, k)] = v;
            }
        }
        out
    };
    pass(&pass(data, true), false)
}

fn erode(data: &[f32], spec: GridSpec, r: usize) -> Vec<f32> {
    window(data, spec, r, f32::min)
}

fn dilate(data: &[f32], spec: GridSpec, r: usize) -> Vec<f32> {
    window(data, spec, r, f32::max)
}

/// Neue Punktwolke aus einer Maske.
pub fn select(cloud: &PointCloud, keep: &[bool]) -> PointCloud {
    PointCloud::new(cloud.points.iter().zip(keep).filter(|(_, &k)| k).map(|(p, _)| *p).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const X0: f64 = 500_000.0;
    const Y0: f64 = 5_400_000.0;

    fn terrain(step: f64, w: f64, z: impl Fn(f64, f64) -> f64) -> Vec<Point3> {
        let n = (w / step) as usize;
        let mut pts = Vec::new();
        for j in 0..=n {
            for i in 0..=n {
                let (dx, dy) = (i as f64 * step, j as f64 * step);
                pts.push(Point3 { x: X0 + dx, y: Y0 + dy, z: z(dx, dy) });
            }
        }
        pts
    }

    #[test]
    fn thinning_keeps_one_per_voxel() {
        let cloud = PointCloud::new(terrain(0.1, 10.0, |_, _| 0.05));
        let keep = voxel_thin(&cloud, 1.0).unwrap();
        assert_eq!(keep.len(), 100 + 2 * 10 + 1); // 10×10 Voxel + Randzeile/-spalte bei x,y = 10
        assert!(keep.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn outliers_are_removed() {
        let mut pts = terrain(0.25, 20.0, |_, _| 300.0);
        let n_ground = pts.len();
        pts.push(Point3 { x: X0 + 5.0, y: Y0 + 5.0, z: 315.0 }); // Vogel / Fehlmessung
        pts.push(Point3 { x: X0 + 12.0, y: Y0 + 8.0, z: 290.0 });
        let (keep, rep) = statistical_outliers(&PointCloud::new(pts), 8, 2.0).unwrap();
        assert!(!keep[n_ground] && !keep[n_ground + 1]);
        assert_eq!(rep.removed, 2, "{rep:?}");
    }

    #[test]
    fn ground_filter_removes_excavator_keeps_slope() {
        // Geneigtes Gelände (10 %) mit Grube und einem 3 m hohen „Bagger“ (4 × 6 m)
        let surface = |dx: f64, dy: f64| {
            let pit = if (20.0..30.0).contains(&dx) && (20.0..30.0).contains(&dy) { -2.0 } else { 0.0 };
            300.0 + 0.1 * dx + pit
        };
        let mut pts = terrain(0.25, 50.0, surface);
        let n_ground = pts.len();
        for j in 0..=24 {
            for i in 0..=16 {
                let (dx, dy) = (8.0 + i as f64 * 0.25, 30.0 + j as f64 * 0.25);
                pts.push(Point3 { x: X0 + dx, y: Y0 + dy, z: surface(dx, dy) + 3.0 });
            }
        }
        let cloud = PointCloud::new(pts);
        let (cls, dtm, rep) = classify_ground(&cloud, GroundParams { cell: 0.5, ..Default::default() }).unwrap();
        let machine_ground = cls[n_ground..].iter().filter(|&&c| c == CLASS_GROUND).count();
        assert_eq!(machine_ground, 0, "{rep:?}");
        let terrain_ground = cls[..n_ground].iter().filter(|&&c| c == CLASS_GROUND).count();
        assert!(terrain_ground as f64 > 0.98 * n_ground as f64, "{terrain_ground}/{n_ground}");
        // DGM unter dem Bagger ≈ Gelände
        let z = dtm.sample(X0 + 10.0, Y0 + 33.0).unwrap();
        assert!((z - surface(10.0, 33.0)).abs() < 0.3, "{z}");
    }

    #[test]
    fn slope_of_plane() {
        let cloud = PointCloud::new(terrain(0.25, 10.0, |dx, dy| 0.3 * dx + 0.4 * dy));
        let spec = GridSpec::covering(&cloud.bbox(), 1.0).unwrap();
        let g = Grid::from_points(&cloud, spec, Aggregate::Mean);
        let s = slope_grid(&g);
        assert!((s[spec.cols * 4 + 4] - 0.5).abs() < 0.02, "{}", s[spec.cols * 4 + 4]);
    }
}
