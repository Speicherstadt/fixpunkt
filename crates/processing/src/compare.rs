//! Soll/Ist-Vergleich: Scan gegen Soll-Höhe, geneigte Soll-Ebene oder Soll-DGM (bzw. zweiten Scan).
//! Ergebnis: Abweichungsraster (für eine Heatmap) und Statistik, inkl. Mehr-/Mindermassen.

use crate::geom::Polygon;
use crate::grid::Grid;
use crate::volume::{self, VolumeReport};
use anyhow::{bail, Result};
use serde::Serialize;

/// Sollfläche
#[derive(Debug, Clone)]
pub enum Design {
    /// waagrechte Sohle auf fester Höhe
    Height(f64),
    /// geneigte Ebene: Höhe `z` am Bezugspunkt (`x`, `y`), Gefälle in % in Richtung `azimuth_deg`
    /// (0° = Nord, 90° = Ost; die Fläche fällt in diese Richtung)
    Slope {
        x: f64,
        y: f64,
        z: f64,
        percent: f64,
        azimuth_deg: f64,
    },
    /// Soll-DGM (muss deckungsgleich mit dem Scan sein; sonst wird bilinear abgetastet)
    Surface(Grid),
}

impl Design {
    pub fn height_at(&self, x: f64, y: f64) -> Option<f64> {
        match self {
            Design::Height(h) => Some(*h),
            Design::Slope {
                x: x0,
                y: y0,
                z,
                percent,
                azimuth_deg,
            } => {
                let (s, c) = azimuth_deg.to_radians().sin_cos();
                let along = (x - x0) * s + (y - y0) * c;
                Some(z - percent / 100.0 * along)
            }
            Design::Surface(g) => g.sample(x, y),
        }
    }

    /// Sollfläche als Raster auf dem Gitter von `like`.
    pub fn rasterize(&self, like: &Grid) -> Grid {
        if let Design::Surface(g) = self {
            if g.spec == like.spec {
                return g.clone();
            }
        }
        let s = like.spec;
        let mut out = Grid::new_nodata(s);
        for r in 0..s.rows {
            for c in 0..s.cols {
                let (x, y) = s.center(c, r);
                if let Some(z) = self.height_at(x, y) {
                    out.set(c, r, z as f32);
                }
            }
        }
        out
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HistogramBin {
    pub from: f64,
    pub to: f64,
    pub cells: usize,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviationReport {
    /// Ist − Soll: positiv = zu hoch (noch abtragen), negativ = zu tief (auffüllen)
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub rms: f64,
    pub std_dev: f64,
    pub tolerance: f64,
    /// Anteil der Fläche innerhalb ±Toleranz
    pub within_tolerance_percent: f64,
    pub above_tolerance_percent: f64,
    pub below_tolerance_percent: f64,
    pub histogram: Vec<HistogramBin>,
    /// noch abzutragen (Ist über Soll) = `fill_m3`, noch aufzufüllen = `cut_m3`
    pub volume: VolumeReport,
}

/// Vergleicht `scan` mit `design` innerhalb `boundary`. `tolerance` in m (z. B. 0,03).
pub fn compare(
    scan: &Grid,
    design: &Design,
    boundary: Option<&Polygon>,
    tolerance: f64,
) -> Result<(DeviationReport, Grid)> {
    let soll = design.rasterize(scan);
    let (volume, diff) = volume::difference(&soll, scan, boundary)?;
    let d: Vec<f64> = diff
        .data
        .iter()
        .filter(|v| v.is_finite())
        .map(|&v| v as f64)
        .collect();
    if d.is_empty() {
        bail!("keine überlappenden Daten zwischen Scan und Soll");
    }
    let n = d.len() as f64;
    let mean = d.iter().sum::<f64>() / n;
    let rms = (d.iter().map(|v| v * v).sum::<f64>() / n).sqrt();
    let std_dev = (d.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n).sqrt();
    let pct = |f: &dyn Fn(f64) -> bool| d.iter().filter(|&&v| f(v)).count() as f64 / n * 100.0;
    let (min, max) = (
        d.iter().copied().fold(f64::INFINITY, f64::min),
        d.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );
    Ok((
        DeviationReport {
            min,
            max,
            mean,
            rms,
            std_dev,
            tolerance,
            within_tolerance_percent: pct(&|v| v.abs() <= tolerance),
            above_tolerance_percent: pct(&|v| v > tolerance),
            below_tolerance_percent: pct(&|v| v < -tolerance),
            histogram: histogram(&d, tolerance),
            volume,
        },
        diff,
    ))
}

/// Klassen für die Legende: ±Toleranz in der Mitte, nach außen verdoppelnd (bis ±16·tol), Rest offen.
fn histogram(d: &[f64], tol: f64) -> Vec<HistogramBin> {
    let tol = if tol > 0.0 { tol } else { 0.01 };
    let mut edges = vec![f64::NEG_INFINITY];
    for k in [16.0, 8.0, 4.0, 2.0, 1.0] {
        edges.push(-k * tol);
    }
    for k in [1.0, 2.0, 4.0, 8.0, 16.0] {
        edges.push(k * tol);
    }
    edges.push(f64::INFINITY);
    let n = d.len() as f64;
    edges
        .windows(2)
        .map(|w| {
            let cells = d
                .iter()
                .filter(|&&v| v >= w[0] && v < w[1] || (w[1].is_infinite() && v >= w[0]))
                .count();
            HistogramBin {
                from: w[0],
                to: w[1],
                cells,
                percent: cells as f64 / n * 100.0,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridSpec;

    fn grid(f: impl Fn(f64, f64) -> f64) -> Grid {
        let spec = GridSpec {
            min_x: 500_000.0,
            max_y: 5_400_020.0,
            cell: 0.5,
            cols: 40,
            rows: 40,
        };
        let mut g = Grid::new_nodata(spec);
        for r in 0..spec.rows {
            for c in 0..spec.cols {
                let (x, y) = spec.center(c, r);
                g.set(c, r, f(x - 500_000.0, y - 5_400_000.0) as f32);
            }
        }
        g
    }

    #[test]
    fn sole_too_high_by_5cm() {
        let scan = grid(|_, _| 296.05);
        let (rep, diff) = compare(&scan, &Design::Height(296.0), None, 0.03).unwrap();
        assert!((rep.mean - 0.05).abs() < 1e-4);
        assert!((rep.above_tolerance_percent - 100.0).abs() < 1e-9);
        // 20 m × 20 m × 5 cm = 20 m³ noch abzutragen
        assert!(
            (rep.volume.fill_m3 - 20.0).abs() < 0.01,
            "{}",
            rep.volume.fill_m3
        );
        assert_eq!(diff.valid_cells(), 1600);
        let total: usize = rep.histogram.iter().map(|b| b.cells).sum();
        assert_eq!(total, 1600);
    }

    #[test]
    fn inclined_design_matches_scan() {
        // Fläche fällt nach Osten (90°) mit 2 %
        let scan = grid(|dx, _| 300.0 - 0.02 * dx);
        let design = Design::Slope {
            x: 500_000.0,
            y: 5_400_000.0,
            z: 300.0,
            percent: 2.0,
            azimuth_deg: 90.0,
        };
        let (rep, _) = compare(&scan, &design, None, 0.01).unwrap();
        assert!(rep.rms < 1e-3, "{}", rep.rms);
        assert!((rep.within_tolerance_percent - 100.0).abs() < 1e-9);
    }

    #[test]
    fn surface_design_with_boundary() {
        let scan = grid(|dx, _| if dx < 10.0 { 1.1 } else { 0.9 });
        let soll = grid(|_, _| 1.0);
        let b = Polygon::rect(500_005.0, 5_400_005.0, 500_015.0, 5_400_015.0);
        let (rep, _) = compare(&scan, &Design::Surface(soll), Some(&b), 0.02).unwrap();
        assert!((rep.above_tolerance_percent - 50.0).abs() < 1.0);
        assert!((rep.below_tolerance_percent - 50.0).abs() < 1.0);
        assert!((rep.volume.fill_m3 - 5.0).abs() < 0.3 && (rep.volume.cut_m3 - 5.0).abs() < 0.3);
    }
}
