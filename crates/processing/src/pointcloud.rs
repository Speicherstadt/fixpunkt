//! Punktwolken einlesen und schreiben: LAS/LAZ (z. B. aus OpenDroneMap) und Text (XYZ/CSV).

use crate::geom::BBox;
pub use crate::las_crs::{epsg_from_geokeys, epsg_from_wkt, las_epsg};
use crate::las_crs::{geokeys, GEOKEY_DIRECTORY};
use anyhow::{bail, Context, Result};
use las::point::Classification;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Default)]
pub struct PointCloud {
    pub points: Vec<Point3>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ReadOptions {
    /// Nur Bodenpunkte (LAS-Klasse 2) verwenden – sinnvoll, wenn die Wolke klassifiziert ist
    /// (z. B. ODM mit `--pc-classify`). Bei Textdateien ohne Wirkung.
    pub ground_only: bool,
}

impl PointCloud {
    pub fn new(points: Vec<Point3>) -> Self {
        PointCloud { points }
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn bbox(&self) -> BBox {
        let mut b = BBox::empty();
        for p in &self.points {
            b.extend(p.x, p.y);
        }
        b
    }

    pub fn z_range(&self) -> Option<(f64, f64)> {
        if self.points.is_empty() {
            return None;
        }
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in &self.points {
            lo = lo.min(p.z);
            hi = hi.max(p.z);
        }
        Some((lo, hi))
    }

    /// Liest anhand der Dateiendung: `.las`/`.laz`, `.e57` oder `.xyz`/`.txt`/`.csv`/`.pts` (nur die Lage;
    /// mit Farben: [`crate::colored::read_colored`]).
    pub fn read(path: &Path, opts: ReadOptions) -> Result<Self> {
        let cloud = match ext_of(path).as_str() {
            "las" | "laz" => Self::read_las(path, opts),
            "xyz" | "txt" | "csv" | "pts" => Self::read_text(path),
            "e57" => Ok(crate::colored::read_e57(path, false)?.cloud),
            _ => bail!(
                "unbekanntes Format: {} (erwartet .las, .laz, .e57, .xyz, .csv, .txt)",
                path.display()
            ),
        }?;
        if cloud.is_empty() {
            bail!("{} enthält keine (passenden) Punkte", path.display());
        }
        Ok(cloud)
    }

    pub fn read_las(path: &Path, opts: ReadOptions) -> Result<Self> {
        Ok(crate::colored::read_las(path, opts, false)?.cloud)
    }

    /// Textdatei mit mindestens drei Spalten x y z (Trennzeichen: Leerzeichen, Tab, Komma, Semikolon).
    /// Zeilen, die nicht mit einer Zahl beginnen (Kopfzeile, Kommentare), werden übersprungen.
    pub fn read_text(path: &Path) -> Result<Self> {
        Ok(crate::colored::read_text(path, false)?.cloud)
    }

    /// Schreibt LAS bzw. LAZ (je nach Endung), Auflösung 1 mm.
    pub fn write_las(&self, path: &Path) -> Result<()> {
        self.write_las_classified(path, None)
    }

    /// Wie [`write_las`](Self::write_las), optional mit LAS-Klasse je Punkt (2 = Boden).
    pub fn write_las_classified(&self, path: &Path, classes: Option<&[u8]>) -> Result<()> {
        self.write_las_georef(path, classes, None)
    }

    /// LAS/LAZ mit Klassen und Bezugssystem (EPSG als GeoKeys, lesbar mit [`las_epsg`])
    pub fn write_las_georef(
        &self,
        path: &Path,
        classes: Option<&[u8]>,
        epsg: Option<u16>,
    ) -> Result<()> {
        self.write_las_with(path, classes, epsg, None)
    }

    /// LAS/LAZ mit allem, was Fixpunkt schreibt: Klassen, Bezugssystem, Farben (dann Punktformat 2)
    pub fn write_las_with(
        &self,
        path: &Path,
        classes: Option<&[u8]>,
        epsg: Option<u16>,
        colors: Option<&[[u8; 3]]>,
    ) -> Result<()> {
        if let Some(c) = classes {
            if c.len() != self.len() {
                bail!(
                    "Klassen ({}) passen nicht zur Punktzahl ({})",
                    c.len(),
                    self.len()
                );
            }
        }
        let b = self.bbox();
        let (zlo, _) = self.z_range().unwrap_or((0.0, 0.0));
        let mut builder = las::Builder::from((1, 2));
        builder.generating_software = "fixpunkt".into();
        if colors.is_some() {
            // Punktformat 2: mit RGB
            builder.point_format = las::point::Format::new(2)?;
        }
        if let Some(code) = epsg {
            builder.vlrs.push(las::Vlr {
                user_id: "LASF_Projection".into(),
                record_id: GEOKEY_DIRECTORY,
                description: "GeoKeyDirectoryTag".into(),
                data: geokeys(code),
            });
        }
        builder.transforms = las::Vector {
            x: las::Transform {
                scale: 0.001,
                offset: b.min_x.floor(),
            },
            y: las::Transform {
                scale: 0.001,
                offset: b.min_y.floor(),
            },
            z: las::Transform {
                scale: 0.001,
                offset: zlo.floor(),
            },
        };
        let header = builder.into_header()?;
        let mut w = las::Writer::from_path(path, header)
            .with_context(|| format!("{} nicht schreibbar", path.display()))?;
        for (i, p) in self.points.iter().enumerate() {
            let classification = Classification::new(classes.map_or(0, |c| c[i]))?;
            // 8 → 16 Bit, wie LAS es vorsieht (255 → 65535)
            let color = colors.map(|c| {
                let [r, g, b] = c[i].map(|v| u16::from(v) * 257);
                las::Color::new(r, g, b)
            });
            w.write_point(las::Point {
                x: p.x,
                y: p.y,
                z: p.z,
                classification,
                color,
                ..Default::default()
            })?;
        }
        w.close()?;
        Ok(())
    }

    /// Punkte verschieben, z. B. in eine andere UTM-Zone oder auf einen anderen Höhenbezug
    pub fn map(&mut self, f: impl Fn(Point3) -> Point3 + Sync) {
        for p in &mut self.points {
            *p = f(*p);
        }
    }

    /// Schreibt Text `x y z` (eine Zeile pro Punkt, mm-genau).
    pub fn write_xyz(&self, path: &Path) -> Result<()> {
        use std::io::Write;
        let f = std::fs::File::create(path)
            .with_context(|| format!("{} nicht schreibbar", path.display()))?;
        let mut w = std::io::BufWriter::new(f);
        for p in &self.points {
            writeln!(w, "{:.3} {:.3} {:.3}", p.x, p.y, p.z)?;
        }
        w.flush()?;
        Ok(())
    }

    /// Schreibt je nach Endung LAS/LAZ oder Text.
    pub fn write(&self, path: &Path, classes: Option<&[u8]>) -> Result<()> {
        match ext_of(path).as_str() {
            "las" | "laz" => self.write_las_classified(path, classes),
            "xyz" | "txt" | "csv" | "pts" => self.write_xyz(path),
            _ => bail!("unbekanntes Ausgabeformat: {}", path.display()),
        }
    }
}

fn ext_of(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default()
}

/// Liest eine LAS/LAZ-Datei Punkt für Punkt und schreibt sie neu – **alle Attribute bleiben erhalten**
/// (Farbe, Intensität, GPS-Zeit, VLRs …).
///
/// `f(idx, punkt)` darf den Punkt ändern und gibt zurück, ob er behalten wird. `idx` ist der Index
/// unter den nicht gelöschten Punkten – also derselbe wie in [`PointCloud::read_las`] ohne
/// `ground_only`; für als gelöscht markierte Punkte ist er `None`.
/// `map_bounds` bildet eine Koordinate ab, um den neuen LAS-Offset zu bestimmen (z. B. die Transformation).
pub fn rewrite_las(
    input: &Path,
    output: &Path,
    map_bounds: Option<&dyn Fn([f64; 3]) -> [f64; 3]>,
    mut f: impl FnMut(Option<usize>, &mut las::Point) -> bool,
) -> Result<u64> {
    let mut reader = las::Reader::from_path(input)
        .with_context(|| format!("{} nicht lesbar", input.display()))?;
    let header = reader.header().clone();
    let mut builder = las::Builder::from(header.clone());
    builder.generating_software = "fixpunkt".into();
    if let Some(m) = map_bounds {
        let b = header.bounds();
        let mut lo = [f64::INFINITY; 3];
        for x in [b.min.x, b.max.x] {
            for y in [b.min.y, b.max.y] {
                for z in [b.min.z, b.max.z] {
                    let q = m([x, y, z]);
                    for k in 0..3 {
                        lo[k] = lo[k].min(q[k]);
                    }
                }
            }
        }
        let t = header.transforms();
        builder.transforms = las::Vector {
            x: las::Transform {
                scale: t.x.scale,
                offset: lo[0].floor(),
            },
            y: las::Transform {
                scale: t.y.scale,
                offset: lo[1].floor(),
            },
            z: las::Transform {
                scale: t.z.scale,
                offset: lo[2].floor(),
            },
        };
    }
    let mut w = las::Writer::from_path(output, builder.into_header()?)
        .with_context(|| format!("{} nicht schreibbar", output.display()))?;
    let (mut idx, mut written) = (0usize, 0u64);
    loop {
        let chunk = reader.read_points(1_000_000)?;
        if chunk.is_empty() {
            break;
        }
        for p in chunk.points() {
            let mut p = p?;
            let i = if p.is_withheld {
                None
            } else {
                idx += 1;
                Some(idx - 1)
            };
            if f(i, &mut p) {
                w.write_point(p)?;
                written += 1;
            }
        }
    }
    w.close()?;
    Ok(written)
}

/// LAS oder LAZ nach der Endung
pub fn is_las(p: &Path) -> bool {
    ext_of(p) == "las" || ext_of(p) == "laz"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("fixpunkt-pc-{}-{name}", std::process::id()))
    }

    #[test]
    fn las_laz_xyz_roundtrip_and_rewrite() {
        let pts: Vec<Point3> = (0..50)
            .map(|i| Point3 {
                x: 500_000.0 + i as f64 * 0.5,
                y: 5_400_000.0 + (i % 7) as f64,
                z: 300.0 + i as f64 * 0.01,
            })
            .collect();
        let cloud = PointCloud::new(pts.clone());
        let classes: Vec<u8> = (0..50).map(|i| if i % 2 == 0 { 2 } else { 1 }).collect();
        let laz = tmp("a.laz");
        cloud.write(&laz, Some(&classes)).unwrap();
        let back = PointCloud::read(&laz, ReadOptions::default()).unwrap();
        assert_eq!(back.len(), 50);
        assert!(
            (back.points[13].x - pts[13].x).abs() < 1e-3
                && (back.points[13].z - pts[13].z).abs() < 1e-3
        );
        let ground = PointCloud::read(&laz, ReadOptions { ground_only: true }).unwrap();
        assert_eq!(ground.len(), 25);

        let out = tmp("b.las");
        let shift = |p: [f64; 3]| [p[0] + 1000.0, p[1], p[2] + 0.5];
        let n = rewrite_las(&laz, &out, Some(&shift), |i, p| {
            p.x += 1000.0;
            p.z += 0.5;
            i.unwrap() < 40
        })
        .unwrap();
        assert_eq!(n, 40);
        let moved = PointCloud::read(&out, ReadOptions { ground_only: true }).unwrap();
        assert_eq!(moved.len(), 20);
        assert!((moved.points[0].x - 501_000.0).abs() < 1e-3);

        let xyz = tmp("c.xyz");
        cloud.write(&xyz, None).unwrap();
        assert_eq!(
            PointCloud::read(&xyz, ReadOptions::default())
                .unwrap()
                .len(),
            50
        );
        for p in [laz, out, xyz] {
            std::fs::remove_file(p).ok();
        }
    }
}
