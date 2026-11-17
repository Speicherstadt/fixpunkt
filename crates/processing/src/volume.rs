//! Volumenberechnung: Aushub (Differenz zweier Aufnahmen) und Haufwerk (gegen Basisfläche).

use crate::geom::Polygon;
use crate::grid::Grid;
use anyhow::{bail, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VolumeReport {
    /// ausgewertete Fläche in m² (Zellen mit Daten)
    pub area_m2: f64,
    /// Abtrag in m³ (Gelände liegt danach tiefer bzw. unter der Basis)
    pub cut_m3: f64,
    /// Auftrag in m³ (Gelände liegt danach höher bzw. über der Basis)
    pub fill_m3: f64,
    /// Auftrag − Abtrag
    pub net_m3: f64,
    pub cells_used: usize,
    /// Zellen innerhalb der Grenze ohne Daten (nicht mitgerechnet)
    pub cells_missing: usize,
    pub cell_size_m: f64,
}

fn integrate(dz: impl Iterator<Item = Option<f64>>, cell_area: f64, cell: f64) -> VolumeReport {
    let (mut cut, mut fill, mut used, mut missing) = (0.0, 0.0, 0, 0);
    for d in dz {
        match d {
            Some(d) if d.is_finite() => {
                used += 1;
                if d > 0.0 {
                    fill += d * cell_area
                } else {
                    cut -= d * cell_area
                }
            }
            _ => missing += 1,
        }
    }
    VolumeReport {
        area_m2: used as f64 * cell_area,
        cut_m3: cut,
        fill_m3: fill,
        net_m3: fill - cut,
        cells_used: used,
        cells_missing: missing,
        cell_size_m: cell,
    }
}

/// Differenz `after − before` auf deckungsgleichen Rastern, optional nur innerhalb `boundary`.
/// Liefert Bericht und Differenzraster (für eine Höhendifferenz-Karte).
pub fn difference(
    before: &Grid,
    after: &Grid,
    boundary: Option<&Polygon>,
) -> Result<(VolumeReport, Grid)> {
    if before.spec != after.spec {
        bail!("Raster sind nicht deckungsgleich");
    }
    let spec = before.spec;
    let mask = spec.mask(boundary);
    let mut diff = Grid::new_nodata(spec);
    for (i, &inside) in mask.iter().enumerate() {
        if inside {
            diff.data[i] = after.data[i] - before.data[i];
        }
    }
    let dz = (0..spec.len())
        .filter(|&i| mask[i])
        .map(|i| Some(diff.data[i] as f64));
    Ok((integrate(dz, spec.cell_area(), spec.cell), diff))
}

/// Ergebnis für einen benannten Bereich
#[derive(Debug, Clone, Serialize)]
pub struct RegionReport {
    pub name: String,
    pub boundary_area_m2: f64,
    #[serde(flatten)]
    pub volume: VolumeReport,
}

/// Aushub/Auftrag getrennt je Bereich (Geofence) plus Summe über alle Bereiche.
pub fn difference_regions(
    before: &Grid,
    after: &Grid,
    regions: &[crate::geom::Region],
) -> Result<(Vec<RegionReport>, VolumeReport)> {
    let mut out = Vec::new();
    for r in regions {
        let (v, _) = difference(before, after, Some(&r.polygon))?;
        out.push(RegionReport {
            name: r.name.clone(),
            boundary_area_m2: r.polygon.area(),
            volume: v,
        });
    }
    let total = sum(out.iter().map(|r| &r.volume));
    Ok((out, total))
}

/// Summe mehrerer Berichte (gleiche Zellgröße vorausgesetzt).
pub fn sum<'a>(reports: impl Iterator<Item = &'a VolumeReport>) -> VolumeReport {
    let mut t = VolumeReport {
        area_m2: 0.0,
        cut_m3: 0.0,
        fill_m3: 0.0,
        net_m3: 0.0,
        cells_used: 0,
        cells_missing: 0,
        cell_size_m: 0.0,
    };
    for r in reports {
        t.area_m2 += r.area_m2;
        t.cut_m3 += r.cut_m3;
        t.fill_m3 += r.fill_m3;
        t.net_m3 += r.net_m3;
        t.cells_used += r.cells_used;
        t.cells_missing += r.cells_missing;
        t.cell_size_m = r.cell_size_m;
    }
    t
}

/// Eine Aufnahme einer Zeitreihe
#[derive(Debug, Clone, Serialize)]
pub struct SeriesStep {
    pub label: String,
    /// gegenüber der Erstaufnahme (Urgelände)
    pub since_start: VolumeReport,
    /// gegenüber der vorherigen Aufnahme
    pub since_previous: VolumeReport,
}

/// Fortschritt über die Bauphase: jede Aufnahme gegen Urgelände und gegen die vorherige.
pub fn series(
    base: &Grid,
    epochs: &[(String, Grid)],
    boundary: Option<&Polygon>,
) -> Result<Vec<SeriesStep>> {
    let mut prev = base;
    let mut out = Vec::new();
    for (label, g) in epochs {
        out.push(SeriesStep {
            label: label.clone(),
            since_start: difference(base, g, boundary)?.0,
            since_previous: difference(prev, g, boundary)?.0,
        });
        prev = g;
    }
    Ok(out)
}

/// Basisfläche unter einem Haufwerk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Base {
    /// Ausgleichsebene durch die Geländehöhen entlang der Grenze
    Plane,
    /// tiefster Punkt entlang der Grenze
    Lowest,
    /// feste Höhe
    Height(f64),
}

/// Volumen eines Haufwerks (bzw. einer Grube) innerhalb `boundary` gegen die Basisfläche.
pub fn stockpile(surface: &Grid, boundary: &Polygon, base: Base) -> Result<VolumeReport> {
    let spec = surface.spec;
    let edge: Vec<[f64; 3]> = boundary
        .densify_exterior(spec.cell)
        .into_iter()
        .filter_map(|[x, y]| surface.sample(x, y).map(|z| [x, y, z]))
        .collect();
    if edge.len() < 3 && !matches!(base, Base::Height(_)) {
        bail!("Grenze liegt nicht (ausreichend) auf der Oberfläche");
    }
    let base_z: Box<dyn Fn(f64, f64) -> f64> = match base {
        Base::Height(h) => Box::new(move |_, _| h),
        Base::Lowest => {
            let lo = edge.iter().map(|p| p[2]).fold(f64::INFINITY, f64::min);
            Box::new(move |_, _| lo)
        }
        Base::Plane => {
            let [a, b, c] = fit_plane(&edge)?;
            let (ox, oy) = (edge[0][0], edge[0][1]);
            Box::new(move |x, y| a * (x - ox) + b * (y - oy) + c)
        }
    };
    let mask = spec.mask(Some(boundary));
    let dz = (0..spec.rows)
        .flat_map(|r| (0..spec.cols).map(move |c| (c, r)))
        .filter_map(|(c, r)| {
            if !mask[r * spec.cols + c] {
                return None;
            }
            let z = surface.get(c, r);
            let (x, y) = spec.center(c, r);
            Some((!z.is_nan()).then(|| z as f64 - base_z(x, y)))
        });
    Ok(integrate(dz, spec.cell_area(), spec.cell))
}

/// Kleinste-Quadrate-Ebene z = a·dx + b·dy + c (relativ zum ersten Punkt).
fn fit_plane(p: &[[f64; 3]]) -> Result<[f64; 3]> {
    let (ox, oy) = (p[0][0], p[0][1]);
    let mut m = [[0.0f64; 3]; 3];
    let mut v = [0.0f64; 3];
    for q in p {
        let r = [q[0] - ox, q[1] - oy, 1.0];
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += r[i] * r[j];
            }
            v[i] += r[i] * q[2];
        }
    }
    let det = |m: &[[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let d = det(&m);
    if d.abs() < 1e-12 {
        bail!("Ebene nicht bestimmbar (Grenzpunkte kollinear)");
    }
    let mut out = [0.0; 3];
    for k in 0..3 {
        let mut mk = m;
        for i in 0..3 {
            mk[i][k] = v[i];
        }
        out[k] = det(&mk) / d;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{Aggregate, GridSpec};
    use crate::pointcloud::{Point3, PointCloud};

    const X0: f64 = 500_000.0;
    const Y0: f64 = 5_400_000.0;

    fn cloud(f: impl Fn(f64, f64) -> f64) -> PointCloud {
        let mut pts = Vec::new();
        for j in 0..=500 {
            for i in 0..=500 {
                let (dx, dy) = (i as f64 * 0.1, j as f64 * 0.1);
                pts.push(Point3 {
                    x: X0 + dx,
                    y: Y0 + dy,
                    z: f(dx, dy),
                });
            }
        }
        PointCloud::new(pts)
    }

    fn grid(c: &PointCloud) -> Grid {
        Grid::from_points(
            c,
            GridSpec::covering(&c.bbox(), 0.25).unwrap(),
            Aggregate::Mean,
        )
    }

    #[test]
    fn pit_frustum_matches_prismoidal_formula() {
        // Grube: oben 20×20 m, unten 14×14 m, 3 m tief (Böschung 1:1)
        let terrain = |dx: f64, _dy: f64| 250.0 + 0.02 * dx;
        let pit = |dx: f64, dy: f64| {
            let d = (dx - 25.0).abs().max((dy - 25.0).abs());
            terrain(dx, dy) - (10.0 - d).clamp(0.0, 3.0)
        };
        let before = grid(&cloud(terrain));
        let after = grid(&cloud(pit));
        let b = Polygon::rect(X0 + 12.0, Y0 + 12.0, X0 + 38.0, Y0 + 38.0);
        let (rep, _) = difference(&before, &after, Some(&b)).unwrap();
        let exact = 3.0 / 6.0 * (400.0 + 196.0 + 4.0 * 289.0);
        assert!(
            (rep.cut_m3 - exact).abs() / exact < 0.01,
            "{} vs {exact}",
            rep.cut_m3
        );
        assert!(rep.fill_m3 < 1.0);
    }

    #[test]
    fn cone_on_slope_with_plane_base() {
        let (r, h) = (8.0, 4.0);
        let surface = grid(&cloud(|dx, dy| {
            let d = ((dx - 25.0).powi(2) + (dy - 25.0).powi(2)).sqrt();
            300.0 + 0.05 * dx - 0.03 * dy + (h * (1.0 - d / r)).max(0.0)
        }));
        let b = Polygon::rect(X0 + 15.0, Y0 + 15.0, X0 + 35.0, Y0 + 35.0);
        let rep = stockpile(&surface, &b, Base::Plane).unwrap();
        let exact = std::f64::consts::PI * r * r * h / 3.0;
        assert!(
            (rep.fill_m3 - exact).abs() / exact < 0.01,
            "{} vs {exact}",
            rep.fill_m3
        );
    }

    #[test]
    fn regions_and_series() {
        let flat = grid(&cloud(|_, _| 300.0));
        let step1 = grid(&cloud(|dx, _| if dx < 25.0 { 299.0 } else { 300.0 }));
        let step2 = grid(&cloud(|_, _| 299.0));
        let regions = vec![
            crate::geom::Region {
                name: "West".into(),
                polygon: Polygon::rect(X0 + 5.0, Y0 + 5.0, X0 + 15.0, Y0 + 15.0),
            },
            crate::geom::Region {
                name: "Ost".into(),
                polygon: Polygon::rect(X0 + 35.0, Y0 + 5.0, X0 + 45.0, Y0 + 15.0),
            },
        ];
        let (per, total) = difference_regions(&flat, &step1, &regions).unwrap();
        assert!(
            (per[0].volume.cut_m3 - 100.0).abs() < 1.0,
            "{}",
            per[0].volume.cut_m3
        );
        assert!(per[1].volume.cut_m3 < 0.5);
        assert!((total.cut_m3 - per[0].volume.cut_m3 - per[1].volume.cut_m3).abs() < 1e-9);
        assert_eq!(per[1].name, "Ost");

        let b = Polygon::rect(X0 + 5.0, Y0 + 5.0, X0 + 45.0, Y0 + 45.0);
        let s = series(
            &flat,
            &[("KW 10".into(), step1), ("KW 11".into(), step2)],
            Some(&b),
        )
        .unwrap();
        assert!(
            (s[1].since_start.cut_m3 - 1600.0).abs() < 2.0,
            "{}",
            s[1].since_start.cut_m3
        );
        assert!(
            (s[0].since_start.cut_m3 + s[1].since_previous.cut_m3 - s[1].since_start.cut_m3).abs()
                < 1e-6
        );
    }

    #[test]
    fn fixed_height_base() {
        let surface = grid(&cloud(|_, _| 10.0));
        let b = Polygon::rect(X0 + 10.0, Y0 + 10.0, X0 + 20.0, Y0 + 20.0);
        let rep = stockpile(&surface, &b, Base::Height(7.0)).unwrap();
        assert!((rep.fill_m3 - 300.0).abs() < 1e-3);
    }
}
