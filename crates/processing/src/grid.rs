//! Höhenraster (DGM/DOM): Punktwolke → regelmäßiges Gitter.
//!
//! Zeile 0 liegt im Norden (wie bei GeoTIFF). Zellwert = Höhe der Zellmitte, `NaN` = keine Daten.

use crate::geom::{BBox, Polygon};
use crate::pointcloud::PointCloud;
use anyhow::{bail, Context, Result};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

/// Lage und Auflösung eines Rasters. Zwei Raster mit gleicher `GridSpec` sind zellgenau deckungsgleich.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridSpec {
    /// Rechtswert der linken Rasterkante
    pub min_x: f64,
    /// Hochwert der oberen Rasterkante
    pub max_y: f64,
    /// Zellgröße in m
    pub cell: f64,
    pub cols: usize,
    pub rows: usize,
}

impl GridSpec {
    /// Raster, das `bbox` überdeckt; die Kanten werden auf Vielfache der Zellgröße gerundet,
    /// damit verschiedene Aufnahmen immer auf dasselbe Gitter fallen.
    pub fn covering(bbox: &BBox, cell: f64) -> Result<Self> {
        if cell.is_nan() || cell <= 0.0 {
            bail!("Zellgröße muss > 0 sein");
        }
        if bbox.is_empty() {
            bail!("leerer Bereich – überlappen die Daten bzw. liegt die Grenze in der Punktwolke?");
        }
        let min_x = (bbox.min_x / cell).floor() * cell;
        let min_y = (bbox.min_y / cell).floor() * cell;
        let max_x = (bbox.max_x / cell).ceil() * cell;
        let max_y = (bbox.max_y / cell).ceil() * cell;
        let cols = (((max_x - min_x) / cell).round() as usize).max(1);
        let rows = (((max_y - min_y) / cell).round() as usize).max(1);
        let cells = cols as u128 * rows as u128;
        if cells > 1_000_000_000 {
            bail!(
                "Raster zu groß ({cols} × {rows} Zellen) – größere Zellgröße oder Grenze verwenden"
            );
        }
        Ok(GridSpec {
            min_x,
            max_y,
            cell,
            cols,
            rows,
        })
    }

    pub fn len(&self) -> usize {
        self.cols * self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn cell_area(&self) -> f64 {
        self.cell * self.cell
    }

    pub fn min_y(&self) -> f64 {
        self.max_y - self.rows as f64 * self.cell
    }

    pub fn max_x(&self) -> f64 {
        self.min_x + self.cols as f64 * self.cell
    }

    pub fn center(&self, col: usize, row: usize) -> (f64, f64) {
        (
            self.min_x + (col as f64 + 0.5) * self.cell,
            self.max_y - (row as f64 + 0.5) * self.cell,
        )
    }

    /// Zelle, in die (x, y) fällt.
    pub fn locate(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        let c = ((x - self.min_x) / self.cell).floor();
        let r = ((self.max_y - y) / self.cell).floor();
        if c < 0.0 || r < 0.0 || c >= self.cols as f64 || r >= self.rows as f64 {
            return None;
        }
        Some((c as usize, r as usize))
    }

    /// Maske: welche Zellmitten liegen im Polygon (ohne Polygon: alle).
    pub fn mask(&self, boundary: Option<&Polygon>) -> Vec<bool> {
        let Some(poly) = boundary else {
            return vec![true; self.len()];
        };
        let mut m = vec![false; self.len()];
        for r in 0..self.rows {
            for c in 0..self.cols {
                let (x, y) = self.center(c, r);
                m[r * self.cols + c] = poly.contains(x, y);
            }
        }
        m
    }
}

/// Wie Punkte innerhalb einer Zelle zu einer Höhe zusammengefasst werden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Aggregate {
    /// Mittelwert – Standard für Photogrammetrie-Wolken
    #[default]
    Mean,
    /// Tiefster Punkt – unterdrückt Bewuchs/Maschinen etwas
    Min,
    /// Höchster Punkt
    Max,
}

#[derive(Debug, Clone)]
pub struct Grid {
    pub spec: GridSpec,
    pub data: Vec<f32>,
}

impl Grid {
    pub fn new_nodata(spec: GridSpec) -> Self {
        Grid {
            data: vec![f32::NAN; spec.len()],
            spec,
        }
    }

    pub fn get(&self, col: usize, row: usize) -> f32 {
        self.data[row * self.spec.cols + col]
    }

    pub fn set(&mut self, col: usize, row: usize, v: f32) {
        self.data[row * self.spec.cols + col] = v;
    }

    pub fn valid_cells(&self) -> usize {
        self.data.iter().filter(|v| !v.is_nan()).count()
    }

    /// Rastert eine Punktwolke auf `spec`. Punkte außerhalb werden ignoriert.
    ///
    /// Höhen werden relativ zu `z_ref` aufsummiert, damit f32 bei großen absoluten Höhen
    /// keine Genauigkeit verliert.
    pub fn from_points(cloud: &PointCloud, spec: GridSpec, agg: Aggregate) -> Self {
        let n = spec.len();
        let z_ref = cloud.z_range().map(|(lo, _)| lo).unwrap_or(0.0);
        let mut acc = vec![0.0f64; n];
        let mut cnt = vec![0u32; n];
        for p in &cloud.points {
            let Some((c, r)) = spec.locate(p.x, p.y) else {
                continue;
            };
            let i = r * spec.cols + c;
            let z = p.z - z_ref;
            if cnt[i] == 0 {
                acc[i] = z;
            } else {
                match agg {
                    Aggregate::Mean => acc[i] += z,
                    Aggregate::Min => acc[i] = acc[i].min(z),
                    Aggregate::Max => acc[i] = acc[i].max(z),
                }
            }
            cnt[i] += 1;
        }
        let data = acc
            .iter()
            .zip(&cnt)
            .map(|(&a, &k)| match (k, agg) {
                (0, _) => f32::NAN,
                (k, Aggregate::Mean) => (a / k as f64 + z_ref) as f32,
                _ => (a + z_ref) as f32,
            })
            .collect();
        Grid { spec, data }
    }

    /// Schließt Datenlücken durch wiederholtes Mitteln der Nachbarzellen (8er-Nachbarschaft).
    /// `max_steps` begrenzt, wie weit (in Zellen) in eine Lücke hinein aufgefüllt wird.
    /// Nur Zellen, für die `limit` `true` ist, werden gefüllt (z. B. innerhalb der Grenze).
    /// Gibt die Zahl der gefüllten Zellen zurück.
    pub fn fill_gaps(&mut self, max_steps: usize, limit: Option<&[bool]>) -> usize {
        let (cols, rows) = (self.spec.cols, self.spec.rows);
        let mut filled = 0;
        for _ in 0..max_steps {
            let mut updates = Vec::new();
            for r in 0..rows {
                for c in 0..cols {
                    let i = r * cols + c;
                    if !self.data[i].is_nan() || limit.is_some_and(|m| !m[i]) {
                        continue;
                    }
                    let (mut s, mut k) = (0.0f64, 0u32);
                    for dr in -1i64..=1 {
                        for dc in -1i64..=1 {
                            let (rr, cc) = (r as i64 + dr, c as i64 + dc);
                            if (dr, dc) == (0, 0)
                                || rr < 0
                                || cc < 0
                                || rr >= rows as i64
                                || cc >= cols as i64
                            {
                                continue;
                            }
                            let v = self.data[rr as usize * cols + cc as usize];
                            if !v.is_nan() {
                                s += v as f64;
                                k += 1;
                            }
                        }
                    }
                    if k >= 2 {
                        updates.push((i, (s / k as f64) as f32));
                    }
                }
            }
            if updates.is_empty() {
                break;
            }
            filled += updates.len();
            for (i, v) in updates {
                self.data[i] = v;
            }
        }
        filled
    }

    /// Bilinear interpolierte Höhe an (x, y); `None` außerhalb oder bei Datenlücke.
    pub fn sample(&self, x: f64, y: f64) -> Option<f64> {
        let s = &self.spec;
        let fc = (x - s.min_x) / s.cell - 0.5;
        let fr = (s.max_y - y) / s.cell - 0.5;
        let c0 = fc.floor().clamp(0.0, (s.cols.saturating_sub(2)) as f64) as usize;
        let r0 = fr.floor().clamp(0.0, (s.rows.saturating_sub(2)) as f64) as usize;
        let c1 = (c0 + 1).min(s.cols - 1);
        let r1 = (r0 + 1).min(s.rows - 1);
        let tx = (fc - c0 as f64).clamp(0.0, 1.0);
        let ty = (fr - r0 as f64).clamp(0.0, 1.0);
        let (x0, y0) = (s.min_x, s.min_y());
        if x < x0 || x > s.max_x() || y < y0 || y > s.max_y {
            return None;
        }
        let v = [
            self.get(c0, r0),
            self.get(c1, r0),
            self.get(c0, r1),
            self.get(c1, r1),
        ];
        if v.iter().any(|v| v.is_nan()) {
            // Rand einer Lücke: nächste gültige Zelle
            let (c, r) = s.locate(x.min(s.max_x() - 1e-9), y.max(y0 + 1e-9))?;
            let z = self.get(c, r);
            return (!z.is_nan()).then_some(z as f64);
        }
        let v: [f64; 4] = v.map(|z| z as f64);
        Some(
            v[0] * (1.0 - tx) * (1.0 - ty)
                + v[1] * tx * (1.0 - ty)
                + v[2] * (1.0 - tx) * ty
                + v[3] * tx * ty,
        )
    }

    /// Überträgt das Raster bilinear auf ein anderes Gitter (z. B. ODM-DGM auf das Abrechnungsraster).
    pub fn resample(&self, spec: GridSpec) -> Grid {
        if spec == self.spec {
            return self.clone();
        }
        let mut out = Grid::new_nodata(spec);
        for r in 0..spec.rows {
            for c in 0..spec.cols {
                let (x, y) = spec.center(c, r);
                if let Some(z) = self.sample(x, y) {
                    out.set(c, r, z as f32);
                }
            }
        }
        out
    }

    /// Minimum, Maximum, Mittel der gültigen Zellen
    pub fn z_stats(&self) -> Option<(f64, f64, f64)> {
        let v: Vec<f64> = self
            .data
            .iter()
            .filter(|v| !v.is_nan())
            .map(|&v| v as f64)
            .collect();
        if v.is_empty() {
            return None;
        }
        let lo = v.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Some((lo, hi, v.iter().sum::<f64>() / v.len() as f64))
    }

    pub fn bbox(&self) -> BBox {
        BBox {
            min_x: self.spec.min_x,
            min_y: self.spec.min_y(),
            max_x: self.spec.max_x(),
            max_y: self.spec.max_y,
        }
    }

    /// Schreibt ein ESRI-ASCII-Grid (`.asc`), das QGIS & Co. direkt öffnen.
    pub fn write_asc(&self, path: &Path) -> Result<()> {
        let f = std::fs::File::create(path)
            .with_context(|| format!("{} nicht schreibbar", path.display()))?;
        let mut w = BufWriter::new(f);
        let s = &self.spec;
        writeln!(w, "ncols {}", s.cols)?;
        writeln!(w, "nrows {}", s.rows)?;
        writeln!(w, "xllcorner {}", s.min_x)?;
        writeln!(w, "yllcorner {}", s.min_y())?;
        writeln!(w, "cellsize {}", s.cell)?;
        writeln!(w, "NODATA_value -9999")?;
        for r in 0..s.rows {
            let line: Vec<String> = (0..s.cols)
                .map(|c| {
                    let v = self.get(c, r);
                    if v.is_nan() {
                        "-9999".to_string()
                    } else {
                        format!("{v:.3}")
                    }
                })
                .collect();
            writeln!(w, "{}", line.join(" "))?;
        }
        w.flush()?;
        Ok(())
    }

    /// Liest ein ESRI-ASCII-Grid (z. B. ein exportiertes DGM).
    pub fn read_asc(path: &Path) -> Result<Self> {
        let f = std::fs::File::open(path)
            .with_context(|| format!("{} nicht lesbar", path.display()))?;
        let mut lines = BufReader::new(f).lines();
        let mut hdr = std::collections::HashMap::new();
        let mut first_data = None;
        for line in lines.by_ref() {
            let line = line?;
            let mut it = line.split_whitespace();
            let (Some(k), Some(v)) = (it.next(), it.next()) else {
                continue;
            };
            if k.parse::<f64>().is_ok() {
                first_data = Some(line);
                break;
            }
            hdr.insert(
                k.to_ascii_lowercase(),
                v.parse::<f64>().with_context(|| format!("Header {k}"))?,
            );
            if hdr.len() == 6 {
                break;
            }
        }
        let get = |k: &str| {
            hdr.get(k)
                .copied()
                .with_context(|| format!("ASC-Header {k} fehlt"))
        };
        let cols = get("ncols")? as usize;
        let rows = get("nrows")? as usize;
        let cell = get("cellsize")?;
        let min_x = hdr
            .get("xllcorner")
            .copied()
            .or(hdr.get("xllcenter").map(|v| v - cell / 2.0))
            .context("xllcorner fehlt")?;
        let min_y = hdr
            .get("yllcorner")
            .copied()
            .or(hdr.get("yllcenter").map(|v| v - cell / 2.0))
            .context("yllcorner fehlt")?;
        let nodata = hdr.get("nodata_value").copied().unwrap_or(-9999.0);
        let spec = GridSpec {
            min_x,
            max_y: min_y + rows as f64 * cell,
            cell,
            cols,
            rows,
        };
        let mut data = Vec::with_capacity(spec.len());
        let rest = first_data.into_iter().map(Ok).chain(lines);
        for line in rest {
            for tok in line?.split_whitespace() {
                let v: f64 = tok
                    .parse()
                    .with_context(|| format!("ungültiger Wert {tok}"))?;
                data.push(if v == nodata { f32::NAN } else { v as f32 });
            }
        }
        if data.len() != spec.len() {
            bail!("ASC: {} Werte, erwartet {}", data.len(), spec.len());
        }
        Ok(Grid { spec, data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pointcloud::Point3;

    fn plane_cloud(z: impl Fn(f64, f64) -> f64, step: f64, w: f64, h: f64) -> PointCloud {
        let mut pts = Vec::new();
        let (nx, ny) = ((w / step) as usize, (h / step) as usize);
        for j in 0..=ny {
            for i in 0..=nx {
                let (x, y) = (500_000.0 + i as f64 * step, 5_400_000.0 + j as f64 * step);
                pts.push(Point3 { x, y, z: z(x, y) });
            }
        }
        PointCloud::new(pts)
    }

    #[test]
    fn spec_is_snapped() {
        let b = BBox {
            min_x: 10.13,
            min_y: 20.07,
            max_x: 12.01,
            max_y: 21.0,
        };
        let s = GridSpec::covering(&b, 0.5).unwrap();
        assert_eq!((s.min_x, s.max_y, s.cols, s.rows), (10.0, 21.0, 5, 2));
    }

    #[test]
    fn rasterize_and_sample_plane() {
        let f = |x: f64, y: f64| 300.0 + 0.1 * (x - 500_000.0) - 0.05 * (y - 5_400_000.0);
        let cloud = plane_cloud(f, 0.1, 20.0, 20.0);
        let spec = GridSpec::covering(&cloud.bbox(), 0.5).unwrap();
        let g = Grid::from_points(&cloud, spec, Aggregate::Mean);
        let z = g.sample(500_010.0, 5_400_010.0).unwrap();
        assert!((z - f(500_010.0, 5_400_010.0)).abs() < 0.01, "{z}");
    }

    #[test]
    fn fill_gaps_closes_hole() {
        let cloud = plane_cloud(|_, _| 5.0, 0.2, 10.0, 10.0);
        let spec = GridSpec::covering(&cloud.bbox(), 0.5).unwrap();
        let mut g = Grid::from_points(&cloud, spec, Aggregate::Mean);
        for r in 8..12 {
            for c in 8..12 {
                g.set(c, r, f32::NAN);
            }
        }
        let filled = g.fill_gaps(10, None);
        assert_eq!(filled, 16);
        assert!(g.data.iter().all(|v| (v - 5.0).abs() < 1e-4));
    }

    #[test]
    fn resample_plane_to_finer_grid() {
        let f = |x: f64, y: f64| 10.0 + 0.2 * (x - 500_000.0) + 0.1 * (y - 5_400_000.0);
        let mut g = Grid::new_nodata(GridSpec {
            min_x: 500_000.0,
            max_y: 5_400_020.0,
            cell: 1.0,
            cols: 20,
            rows: 20,
        });
        for r in 0..20 {
            for c in 0..20 {
                let (x, y) = g.spec.center(c, r);
                g.set(c, r, f(x, y) as f32);
            }
        }
        let b = BBox {
            min_x: 500_005.0,
            min_y: 5_400_005.0,
            max_x: 500_015.0,
            max_y: 5_400_015.0,
        };
        let fine = g.resample(GridSpec::covering(&b, 0.5).unwrap());
        assert_eq!(fine.valid_cells(), 400);
        let (x, y) = fine.spec.center(7, 3);
        assert!((fine.get(7, 3) as f64 - f(x, y)).abs() < 1e-4);
        let (lo, hi, _) = fine.z_stats().unwrap();
        assert!(lo < hi);
    }

    #[test]
    fn asc_roundtrip() {
        let cloud = plane_cloud(|x, _| x - 500_000.0, 0.25, 4.0, 3.0);
        let spec = GridSpec::covering(&cloud.bbox(), 1.0).unwrap();
        let mut g = Grid::from_points(&cloud, spec, Aggregate::Max);
        g.set(0, 0, f32::NAN);
        let p = std::env::temp_dir().join(format!("fixpunkt-test-{}.asc", std::process::id()));
        g.write_asc(&p).unwrap();
        let g2 = Grid::read_asc(&p).unwrap();
        std::fs::remove_file(&p).ok();
        assert_eq!(g.spec, g2.spec);
        assert!(g2.get(0, 0).is_nan());
        assert!((g.get(2, 1) - g2.get(2, 1)).abs() < 1e-3);
    }
}
