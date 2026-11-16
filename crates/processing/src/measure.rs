//! Messen auf dem Höhenraster: Punkt, Strecke/Polylinie, Gefälle, Flächen, Profile.

use crate::geom::Polygon;
use crate::grid::Grid;
use anyhow::{bail, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PointMeasure {
    pub x: f64,
    pub y: f64,
    /// Höhe auf der Oberfläche (`None`: keine Daten)
    pub z: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LineMeasure {
    /// Strecke in der Lage (horizontal)
    pub length_2d: f64,
    /// Strecke entlang der Oberfläche
    pub length_3d: f64,
    /// Höhe am Anfang/Ende
    pub z_start: Option<f64>,
    pub z_end: Option<f64>,
    /// Höhenunterschied Ende − Anfang
    pub dz: Option<f64>,
    /// mittleres Gefälle in % (dz / length_2d)
    pub slope_percent: Option<f64>,
    pub z_min: Option<f64>,
    pub z_max: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfilePoint {
    /// Station (Abstand vom Anfang, horizontal)
    pub station: f64,
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AreaMeasure {
    /// horizontale (projizierte) Fläche der Grenze
    pub area_2d: f64,
    /// Oberfläche (schräg) innerhalb der Grenze, nur über Zellen mit Daten
    pub area_3d: f64,
    pub perimeter: f64,
    /// Zellen innerhalb der Grenze ohne Daten
    pub cells_missing: usize,
    pub z_min: Option<f64>,
    pub z_max: Option<f64>,
    pub z_mean: Option<f64>,
}

pub fn point(g: &Grid, x: f64, y: f64) -> PointMeasure {
    PointMeasure { x, y, z: g.sample(x, y) }
}

/// Punkte entlang einer Polylinie etwa alle `step` m, inkl. aller Stützpunkte und Endpunkt.
pub fn profile(g: &Grid, line: &[[f64; 2]], step: f64) -> Result<Vec<ProfilePoint>> {
    if line.len() < 2 {
        bail!("Linie braucht mindestens 2 Punkte");
    }
    if !(step > 0.0) {
        bail!("Schrittweite muss > 0 sein");
    }
    let mut out = Vec::new();
    let mut station = 0.0;
    for w in line.windows(2) {
        let ([x0, y0], [x1, y1]) = (w[0], w[1]);
        let len = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let k = ((len / step).ceil() as usize).max(1);
        for s in 0..k {
            let t = s as f64 / k as f64;
            let (x, y) = (x0 + t * (x1 - x0), y0 + t * (y1 - y0));
            out.push(ProfilePoint { station: station + t * len, x, y, z: g.sample(x, y) });
        }
        station += len;
    }
    let [x, y] = line[line.len() - 1];
    out.push(ProfilePoint { station, x, y, z: g.sample(x, y) });
    Ok(out)
}

/// Strecke bzw. Polylinie: Längen, Höhen, Gefälle. Die 3D-Länge folgt der Oberfläche
/// (abgetastet mit halber Zellgröße); Lücken zählen horizontal.
pub fn line(g: &Grid, pts: &[[f64; 2]]) -> Result<LineMeasure> {
    let prof = profile(g, pts, g.spec.cell / 2.0)?;
    let mut l3 = 0.0;
    for w in prof.windows(2) {
        let d2 = w[1].station - w[0].station;
        let dz = match (w[0].z, w[1].z) {
            (Some(a), Some(b)) => b - a,
            _ => 0.0,
        };
        l3 += (d2 * d2 + dz * dz).sqrt();
    }
    let l2 = prof.last().map(|p| p.station).unwrap_or(0.0);
    let (zs, ze) = (prof[0].z, prof[prof.len() - 1].z);
    let dz = zs.zip(ze).map(|(a, b)| b - a);
    let zv: Vec<f64> = prof.iter().filter_map(|p| p.z).collect();
    Ok(LineMeasure {
        length_2d: l2,
        length_3d: l3,
        z_start: zs,
        z_end: ze,
        dz,
        slope_percent: dz.filter(|_| l2 > 0.0).map(|d| d / l2 * 100.0),
        z_min: zv.iter().copied().reduce(f64::min),
        z_max: zv.iter().copied().reduce(f64::max),
    })
}

/// Fläche innerhalb einer Grenze: horizontal (exakt aus dem Polygon) und Oberfläche (aus dem Raster).
pub fn area(g: &Grid, boundary: &Polygon) -> AreaMeasure {
    let s = g.spec;
    let mask = s.mask(Some(boundary));
    let slope = crate::filter::slope_grid(g);
    let (mut a3, mut missing, mut zs) = (0.0, 0usize, Vec::new());
    for i in 0..s.len() {
        if !mask[i] {
            continue;
        }
        let z = g.data[i];
        if z.is_nan() {
            missing += 1;
            continue;
        }
        a3 += s.cell_area() * (1.0 + slope[i] * slope[i]).sqrt();
        zs.push(z as f64);
    }
    AreaMeasure {
        area_2d: boundary.area(),
        area_3d: a3,
        perimeter: boundary.perimeter(),
        cells_missing: missing,
        z_min: zs.iter().copied().reduce(f64::min),
        z_max: zs.iter().copied().reduce(f64::max),
        z_mean: (!zs.is_empty()).then(|| zs.iter().sum::<f64>() / zs.len() as f64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{Aggregate, GridSpec};
    use crate::pointcloud::{Point3, PointCloud};

    const X0: f64 = 500_000.0;
    const Y0: f64 = 5_400_000.0;

    fn plane(a: f64, b: f64) -> Grid {
        let mut pts = Vec::new();
        for j in 0..=200 {
            for i in 0..=200 {
                let (dx, dy) = (i as f64 * 0.1, j as f64 * 0.1);
                pts.push(Point3 { x: X0 + dx, y: Y0 + dy, z: 100.0 + a * dx + b * dy });
            }
        }
        let c = PointCloud::new(pts);
        let spec = GridSpec::covering(&c.bbox(), 0.25).unwrap();
        Grid::from_points(&c, spec, Aggregate::Mean)
    }

    #[test]
    fn line_on_inclined_plane() {
        let g = plane(0.02, 0.0); // 2 % Gefälle in x
        let m = line(&g, &[[X0 + 2.0, Y0 + 5.0], [X0 + 17.0, Y0 + 5.0]]).unwrap();
        assert!((m.length_2d - 15.0).abs() < 1e-9);
        assert!((m.length_3d - 15.0 * (1.0f64 + 0.0004).sqrt()).abs() < 1e-3, "{}", m.length_3d);
        assert!((m.slope_percent.unwrap() - 2.0).abs() < 0.01);
        assert!((m.dz.unwrap() - 0.3).abs() < 0.002);
    }

    #[test]
    fn polyline_profile_stations() {
        let g = plane(0.0, 0.0);
        let p = profile(&g, &[[X0 + 1.0, Y0 + 1.0], [X0 + 4.0, Y0 + 5.0], [X0 + 4.0, Y0 + 8.0]], 1.0).unwrap();
        assert_eq!(p.first().unwrap().station, 0.0);
        assert!((p.last().unwrap().station - 8.0).abs() < 1e-9);
        assert!(p.iter().all(|q| (q.z.unwrap() - 100.0).abs() < 1e-3));
        assert!(profile(&g, &[[X0, Y0]], 1.0).is_err());
    }

    #[test]
    fn surface_area_of_slope() {
        let g = plane(0.75, 0.0); // Neigung 3:4 → Oberfläche = 1,25 × Grundfläche
        let b = Polygon::rect(X0 + 4.0, Y0 + 4.0, X0 + 14.0, Y0 + 12.0);
        let a = area(&g, &b);
        assert!((a.area_2d - 80.0).abs() < 1e-9);
        assert!((a.area_3d - 100.0).abs() < 1.0, "{}", a.area_3d);
        assert_eq!(a.cells_missing, 0);
        assert!((a.perimeter - 36.0).abs() < 1e-9);
    }

    #[test]
    fn point_outside_has_no_height() {
        let g = plane(0.0, 0.0);
        assert!(point(&g, X0 + 5.0, Y0 + 5.0).z.is_some());
        assert!(point(&g, X0 - 50.0, Y0).z.is_none());
    }
}
