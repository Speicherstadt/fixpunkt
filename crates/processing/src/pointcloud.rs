//! Punktwolken einlesen und schreiben: LAS/LAZ (z. B. aus OpenDroneMap) und Text (XYZ/CSV).

use crate::geom::BBox;
use anyhow::{bail, Context, Result};
use las::point::Classification;
use std::io::{BufRead, BufReader};
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

    /// Liest anhand der Dateiendung: `.las`/`.laz` oder `.xyz`/`.txt`/`.csv`/`.pts`.
    pub fn read(path: &Path, opts: ReadOptions) -> Result<Self> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        let cloud = match ext.as_str() {
            "las" | "laz" => Self::read_las(path, opts),
            "xyz" | "txt" | "csv" | "pts" => Self::read_text(path),
            _ => bail!("unbekanntes Format: {} (erwartet .las, .laz, .xyz, .csv, .txt)", path.display()),
        }?;
        if cloud.is_empty() {
            bail!("{} enthält keine (passenden) Punkte", path.display());
        }
        Ok(cloud)
    }

    pub fn read_las(path: &Path, opts: ReadOptions) -> Result<Self> {
        let mut reader = las::Reader::from_path(path)
            .with_context(|| format!("{} nicht lesbar", path.display()))?;
        let total = reader.header().number_of_points();
        let mut points = Vec::with_capacity(total as usize);
        loop {
            let chunk = reader.read_points(1_000_000)?;
            if chunk.is_empty() {
                break;
            }
            for p in chunk.points() {
                let p = p?;
                if opts.ground_only && p.classification != Classification::Ground {
                    continue;
                }
                if p.is_withheld {
                    continue;
                }
                points.push(Point3 { x: p.x, y: p.y, z: p.z });
            }
        }
        Ok(PointCloud { points })
    }

    /// Textdatei mit mindestens drei Spalten x y z (Trennzeichen: Leerzeichen, Tab, Komma, Semikolon).
    /// Zeilen, die nicht mit einer Zahl beginnen (Kopfzeile, Kommentare), werden übersprungen.
    pub fn read_text(path: &Path) -> Result<Self> {
        let f = std::fs::File::open(path).with_context(|| format!("{} nicht lesbar", path.display()))?;
        let mut points = Vec::new();
        for (no, line) in BufReader::new(f).lines().enumerate() {
            let line = line?;
            let mut it = line
                .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
                .filter(|s| !s.is_empty());
            let vals: Vec<f64> = it.by_ref().take(3).map_while(|s| s.parse().ok()).collect();
            match vals.len() {
                3 => points.push(Point3 { x: vals[0], y: vals[1], z: vals[2] }),
                0 => continue,
                _ => bail!("{}:{}: erwartet x y z", path.display(), no + 1),
            }
        }
        Ok(PointCloud { points })
    }

    /// Schreibt LAS bzw. LAZ (je nach Endung), Auflösung 1 mm.
    pub fn write_las(&self, path: &Path) -> Result<()> {
        let b = self.bbox();
        let (zlo, _) = self.z_range().unwrap_or((0.0, 0.0));
        let mut builder = las::Builder::from((1, 2));
        builder.generating_software = "fixpunkt".into();
        builder.transforms = las::Vector {
            x: las::Transform { scale: 0.001, offset: b.min_x.floor() },
            y: las::Transform { scale: 0.001, offset: b.min_y.floor() },
            z: las::Transform { scale: 0.001, offset: zlo.floor() },
        };
        let header = builder.into_header()?;
        let mut w = las::Writer::from_path(path, header)
            .with_context(|| format!("{} nicht schreibbar", path.display()))?;
        for p in &self.points {
            w.write_point(las::Point { x: p.x, y: p.y, z: p.z, ..Default::default() })?;
        }
        w.close()?;
        Ok(())
    }
}
