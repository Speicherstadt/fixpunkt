//! Punktwolken mit Farbe lesen und schreiben: LAS/LAZ (Punktformate mit RGB), Text mit sechs Spalten
//! `x y z r g b`, E57 (Laserscanner wie Leica BLK2GO, mit Standpunkt-Pose). Für die Rechnungen zählt nur die Lage;
//! die Farben braucht die 3D-Ansicht.

use crate::pointcloud::{Point3, PointCloud, ReadOptions};
use anyhow::{bail, Context, Result};
use las::point::Classification;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Punktwolke mit Farbe je Punkt (8 Bit), falls die Datei welche hat
#[derive(Debug, Clone)]
pub struct Colored {
    pub cloud: PointCloud,
    pub colors: Option<Vec<[u8; 3]>>,
}

fn ext_of(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default()
}

/// LAS-Farben sind 16 Bit; manche Programme schreiben trotzdem 0…255 – dann nicht verschieben
fn to_8bit(raw: Vec<[u16; 3]>) -> Vec<[u8; 3]> {
    let max = raw.iter().flatten().copied().max().unwrap_or(0);
    let shift = if max > 255 { 8 } else { 0 };
    raw.into_iter()
        .map(|c| c.map(|v| (v >> shift) as u8))
        .collect()
}

/// LAS/LAZ; Farben nur mit `want_colors` und wenn das Punktformat welche hat
pub fn read_las(path: &Path, opts: ReadOptions, want_colors: bool) -> Result<Colored> {
    let mut reader =
        las::Reader::from_path(path).with_context(|| format!("{} nicht lesbar", path.display()))?;
    let total = reader.header().number_of_points() as usize;
    let has_color = want_colors && reader.header().point_format().has_color;
    let mut points = Vec::with_capacity(total);
    let mut colors = Vec::with_capacity(if has_color { total } else { 0 });
    loop {
        let chunk = reader.read_points(1_000_000)?;
        if chunk.is_empty() {
            break;
        }
        for p in chunk.points() {
            let p = p?;
            if (opts.ground_only && p.classification != Classification::Ground) || p.is_withheld {
                continue;
            }
            points.push(Point3 {
                x: p.x,
                y: p.y,
                z: p.z,
            });
            if has_color {
                let c = p.color.unwrap_or_default();
                colors.push([c.red, c.green, c.blue]);
            }
        }
    }
    Ok(Colored {
        cloud: PointCloud::new(points),
        colors: has_color.then(|| to_8bit(colors)),
    })
}

/// Text: x y z, optional r g b (0…255 oder 0…1). Zeilen, die nicht mit einer Zahl beginnen, werden übersprungen.
pub fn read_text(path: &Path, want_colors: bool) -> Result<Colored> {
    let f =
        std::fs::File::open(path).with_context(|| format!("{} nicht lesbar", path.display()))?;
    let mut points = Vec::new();
    let mut rgb: Vec<[f64; 3]> = Vec::new();
    let mut all_colored = want_colors;
    for (no, line) in BufReader::new(f).lines().enumerate() {
        let line = line?;
        let vals: Vec<f64> = line
            .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
            .filter(|s| !s.is_empty())
            .take(6)
            .map_while(|s| s.parse().ok())
            .collect();
        match vals.len() {
            0 => continue,
            1 | 2 => bail!("{}:{}: erwartet x y z", path.display(), no + 1),
            n => {
                points.push(Point3 {
                    x: vals[0],
                    y: vals[1],
                    z: vals[2],
                });
                if all_colored && n == 6 {
                    rgb.push([vals[3], vals[4], vals[5]]);
                } else {
                    all_colored = false;
                }
            }
        }
    }
    let colors = all_colored.then(|| {
        let unit = rgb.iter().flatten().all(|v| *v <= 1.0);
        let k = if unit { 255.0 } else { 1.0 };
        rgb.iter()
            .map(|c| c.map(|v| (v * k).round().clamp(0.0, 255.0) as u8))
            .collect()
    });
    Ok(Colored {
        cloud: PointCloud::new(points),
        colors,
    })
}

/// E57: alle Scans, Pose angewandt (lokale Scannerkoordinaten → Projektkoordinaten), kugelförmige Koordinaten
/// umgerechnet; Punkte ohne gültige Lage fallen weg
pub fn read_e57(path: &Path, want_colors: bool) -> Result<Colored> {
    let mut reader = e57::E57Reader::from_file(path)
        .with_context(|| format!("{} ist keine lesbare E57", path.display()))?;
    let scans = reader.pointclouds();
    let colored = want_colors && scans.iter().all(|s| s.has_color());
    let mut points = Vec::new();
    let mut colors = Vec::new();
    for scan in &scans {
        let mut it = reader.pointcloud_simple(scan)?;
        it.spherical_to_cartesian(true);
        it.apply_pose(true);
        for p in it {
            let p = p?;
            let e57::CartesianCoordinate::Valid { x, y, z } = p.cartesian else {
                continue;
            };
            points.push(Point3 { x, y, z });
            if colored {
                let c = p.color.map_or([0.0; 3], |c| [c.red, c.green, c.blue]);
                colors.push(c.map(|v| (v * 255.0).round().clamp(0.0, 255.0) as u8));
            }
        }
    }
    if points.is_empty() {
        bail!("{} enthält keine Punkte", path.display());
    }
    Ok(Colored {
        cloud: PointCloud::new(points),
        colors: colored.then_some(colors),
    })
}

/// Liest mit Farben, wo die Datei welche hat
pub fn read_colored(path: &Path, opts: ReadOptions) -> Result<Colored> {
    let c = match ext_of(path).as_str() {
        "las" | "laz" => read_las(path, opts, true)?,
        "xyz" | "txt" | "csv" | "pts" => read_text(path, true)?,
        "e57" => read_e57(path, true)?,
        _ => bail!("unbekanntes Format: {}", path.display()),
    };
    if c.cloud.is_empty() {
        bail!("{} enthält keine (passenden) Punkte", path.display());
    }
    Ok(c)
}

/// Gleichmäßig ausdünnen auf höchstens `max` Punkte (jeder k-te; Scans liegen zeilenweise, das verteilt sich)
pub fn thin(c: &Colored, max: usize) -> Colored {
    let k = c.cloud.len().div_ceil(max.max(1)).max(1);
    Colored {
        cloud: PointCloud::new(c.cloud.points.iter().step_by(k).copied().collect()),
        colors: c
            .colors
            .as_ref()
            .map(|v| v.iter().step_by(k).copied().collect()),
    }
}

/// LAS/LAZ mit Farbe (Punktformat 2) und Bezugssystem
pub fn write_las_colored(
    c: &PointCloud,
    colors: &[[u8; 3]],
    path: &Path,
    epsg: Option<u16>,
) -> Result<()> {
    if colors.len() != c.len() {
        bail!(
            "Farben ({}) passen nicht zur Punktzahl ({})",
            colors.len(),
            c.len()
        );
    }
    c.write_las_with(path, None, epsg, Some(colors))
}

#[cfg(test)]
mod tests {
    use super::*;
    use e57::{E57Writer, Quaternion, Record, RecordValue, Transform, Translation};

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("fixpunkt-colored-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d.join(name)
    }

    #[test]
    fn text_with_and_without_colors() {
        let p = tmp("farbe.xyz");
        std::fs::write(&p, "x y z r g b\n1 2 3 255 128 0\n4 5 6 10 20 30\n").unwrap();
        let c = read_colored(&p, ReadOptions::default()).unwrap();
        assert_eq!(c.colors, Some(vec![[255, 128, 0], [10, 20, 30]]));
        std::fs::write(&p, "1 2 3 1.0 0.5 0\n4 5 6 0 0 1\n").unwrap();
        assert_eq!(
            read_colored(&p, ReadOptions::default()).unwrap().colors,
            Some(vec![[255, 128, 0], [0, 0, 255]])
        );
        // eine Zeile ohne Farbe: keine Farben für die Wolke
        std::fs::write(&p, "1 2 3 255 0 0\n4 5 6\n").unwrap();
        let c = read_colored(&p, ReadOptions::default()).unwrap();
        assert!(c.colors.is_none());
        assert_eq!(c.cloud.len(), 2);
    }

    #[test]
    fn las_colors_and_8bit_detection() {
        assert_eq!(to_8bit(vec![[65535, 32768, 0]]), vec![[255, 128, 0]]);
        assert_eq!(
            to_8bit(vec![[255, 128, 0]]),
            vec![[255, 128, 0]],
            "schon 8 Bit"
        );
        let p = tmp("bunt.las");
        let c = PointCloud::new(vec![
            Point3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            Point3 {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            },
        ]);
        write_las_colored(&c, &[[200, 100, 50], [1, 2, 3]], &p, Some(25832)).unwrap();
        let r = read_colored(&p, ReadOptions::default()).unwrap();
        assert_eq!(r.colors, Some(vec![[200, 100, 50], [1, 2, 3]]));
        assert_eq!(crate::pointcloud::las_epsg(&p).unwrap(), Some(25832));
        // ohne Farben gelesen (Rechnungen): gleiche Punkte
        assert_eq!(
            PointCloud::read(&p, ReadOptions::default()).unwrap().points,
            c.points
        );
    }

    #[test]
    fn e57_with_pose_and_colors() {
        // Scan im Scannersystem, Pose: 90° um z gedreht und nach (512 000 | 5 403 000 | 300) verschoben
        let p = tmp("scan.e57");
        {
            let mut w = E57Writer::from_file(&p, "test-guid").unwrap();
            let proto = vec![
                Record::CARTESIAN_X_F64,
                Record::CARTESIAN_Y_F64,
                Record::CARTESIAN_Z_F64,
                Record::COLOR_RED_U8,
                Record::COLOR_GREEN_U8,
                Record::COLOR_BLUE_U8,
            ];
            let mut pc = w.add_pointcloud("scan-1", proto).unwrap();
            let s = std::f64::consts::FRAC_1_SQRT_2;
            pc.set_transform(Some(Transform {
                rotation: Quaternion {
                    w: s,
                    x: 0.0,
                    y: 0.0,
                    z: s,
                },
                translation: Translation {
                    x: 512_000.0,
                    y: 5_403_000.0,
                    z: 300.0,
                },
            }));
            for (x, y, z, r) in [(1.0, 0.0, 0.5, 255), (0.0, 2.0, -0.5, 0)] {
                pc.add_point(vec![
                    RecordValue::Double(x),
                    RecordValue::Double(y),
                    RecordValue::Double(z),
                    RecordValue::Integer(r),
                    RecordValue::Integer(128),
                    RecordValue::Integer(0),
                ])
                .unwrap();
            }
            pc.finalize().unwrap();
            w.finalize().unwrap();
        }
        let c = read_colored(&p, ReadOptions::default()).unwrap();
        // (1|0) um 90° gedreht → (0|1); (0|2) → (−2|0)
        let pts: Vec<[f64; 3]> = c.cloud.points.iter().map(|q| [q.x, q.y, q.z]).collect();
        for (got, want) in pts.iter().zip([
            [512_000.0, 5_403_001.0, 300.5],
            [511_998.0, 5_403_000.0, 299.5],
        ]) {
            for k in 0..3 {
                assert!((got[k] - want[k]).abs() < 1e-6, "{got:?} {want:?}");
            }
        }
        assert_eq!(c.colors, Some(vec![[255, 128, 0], [0, 128, 0]]));
        // Rechnungen lesen E57 genauso
        assert_eq!(
            PointCloud::read(&p, ReadOptions::default()).unwrap().len(),
            2
        );
    }

    #[test]
    fn thinning_keeps_colors_aligned() {
        let pts: Vec<Point3> = (0..10)
            .map(|i| Point3 {
                x: f64::from(i),
                y: 0.0,
                z: 0.0,
            })
            .collect();
        let colors: Vec<[u8; 3]> = (0..10).map(|i| [i as u8, 0, 0]).collect();
        let t = thin(
            &Colored {
                cloud: PointCloud::new(pts),
                colors: Some(colors),
            },
            4,
        );
        assert_eq!(
            t.cloud.points.iter().map(|p| p.x).collect::<Vec<_>>(),
            [0.0, 3.0, 6.0, 9.0]
        );
        assert_eq!(
            t.colors.unwrap().iter().map(|c| c[0]).collect::<Vec<_>>(),
            [0, 3, 6, 9]
        );
    }
}
