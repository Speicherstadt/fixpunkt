//! Punktwolke aus NodeODM auf das Bezugssystem der Baustelle bringen. ODM rechnet in der UTM-Zone der Fotos
//! (östlich von 12° Ost Zone 33) und gibt Höhen so aus, wie die Drohne sie misst. Fixpunkt rechnet in der Zone
//! der Baustelle; ein Höhenversatz (z. B. die Höhenanomalie aus GCG2016) wird abgezogen.

use anyhow::{bail, Result};
use fixpunkt::colored::{read_colored, Colored};
use fixpunkt::crs::{utm_zone, Utm};
use fixpunkt::pointcloud::{las_epsg, Point3, ReadOptions};
use std::path::Path;

/// Was mit der Wolke geschehen ist
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    /// Bezugssystem laut Datei (`None`: keins angegeben, Zone der Baustelle angenommen)
    pub source_epsg: Option<u32>,
    /// umgerechnet (Zone oder Höhe) statt unverändert übernommen
    pub converted: bool,
}

/// Ziel-Zone und Umrechnung: `None`, wenn nichts zu tun ist
fn plan(source: Option<u32>, target: u32, offset: f64) -> Result<Option<(Utm, Utm)>> {
    let Some(to) = utm_zone(target) else {
        bail!("Bezugssystem der Baustelle EPSG:{target} ist kein UTM");
    };
    let from = match source {
        None => to,
        Some(code) => match utm_zone(code) {
            Some(z) => z,
            None => bail!(
                "Bezugssystem der Punktwolke EPSG:{code} wird nicht unterstützt (erwartet UTM)"
            ),
        },
    };
    if from == to && offset == 0.0 {
        return Ok(None);
    }
    Ok(Some((Utm::new(from)?, Utm::new(to)?)))
}

/// `src` (LAS/LAZ aus ODM) als `dest` im Bezugssystem `target_epsg` ablegen, `offset` von z abgezogen
pub fn prepare(src: &Path, dest: &Path, target_epsg: u32, offset: f64) -> Result<Prepared> {
    let source_epsg = las_epsg(src)?;
    let Some((from, to)) = plan(source_epsg, target_epsg, offset)? else {
        std::fs::rename(src, dest)?;
        return Ok(Prepared {
            source_epsg,
            converted: false,
        });
    };
    // ODM-Punktwolken haben Farben aus den Fotos: mitnehmen
    let Colored { mut cloud, colors } = read_colored(src, ReadOptions::default())?;
    cloud.map(|p| {
        let (x, y) = from.to_zone(&to, p.x, p.y);
        Point3 {
            x,
            y,
            z: p.z - offset,
        }
    });
    cloud.write_las_with(
        dest,
        None,
        u16::try_from(target_epsg).ok(),
        colors.as_deref(),
    )?;
    std::fs::remove_file(src)?;
    Ok(Prepared {
        source_epsg,
        converted: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixpunkt::pointcloud::PointCloud;

    fn cloud_at(zone: &Utm) -> PointCloud {
        // Raster um 50° N, 12,001° O (gerade in Zone 33), Höhe 350 m ellipsoidisch
        let mut pts = Vec::new();
        for i in 0..10 {
            for j in 0..10 {
                let (e, n) = zone.forward(50.0 + j as f64 * 1e-5, 12.001 + i as f64 * 1e-5);
                pts.push(Point3 {
                    x: e,
                    y: n,
                    z: 350.0,
                });
            }
        }
        PointCloud::new(pts)
    }

    #[test]
    fn zone_33_cloud_moves_to_zone_32_with_height_offset() {
        let dir = tempfile::tempdir().unwrap();
        let (z32, z33) = (Utm::new(32).unwrap(), Utm::new(33).unwrap());
        let src = dir.path().join("odm.laz");
        cloud_at(&z33)
            .write_las_georef(&src, None, Some(32633))
            .unwrap();
        let dest = dir.path().join("drohne.laz");
        let r = prepare(&src, &dest, 25832, 47.5).unwrap();
        assert_eq!(
            r,
            Prepared {
                source_epsg: Some(32633),
                converted: true
            }
        );
        assert!(!src.exists());
        assert_eq!(las_epsg(&dest).unwrap(), Some(25832));
        // unabhängig: dieselben Längen/Breiten direkt in Zone 32 abgebildet
        let got = PointCloud::read(&dest, ReadOptions::default()).unwrap();
        let want = cloud_at(&z32);
        for (g, w) in got.points.iter().zip(&want.points) {
            assert!(
                (g.x - w.x).abs() < 0.002 && (g.y - w.y).abs() < 0.002,
                "{g:?} {w:?}"
            );
            assert!((g.z - 302.5).abs() < 0.002);
        }
    }

    #[test]
    fn same_zone_without_offset_is_moved_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let z32 = Utm::new(32).unwrap();
        let src = dir.path().join("odm.laz");
        cloud_at(&z32).write_las(&src).unwrap();
        let bytes = std::fs::read(&src).unwrap();
        let dest = dir.path().join("drohne.laz");
        let r = prepare(&src, &dest, 25832, 0.0).unwrap();
        assert_eq!(
            r,
            Prepared {
                source_epsg: None,
                converted: false
            }
        );
        assert_eq!(std::fs::read(&dest).unwrap(), bytes);
    }

    #[test]
    fn non_utm_is_refused() {
        assert!(plan(Some(4326), 25832, 0.0).is_err());
        assert!(plan(None, 31467, 0.0).is_err());
        assert!(plan(Some(32632), 25832, 0.0).unwrap().is_none());
        assert!(plan(Some(32632), 25832, 1.0).unwrap().is_some());
    }
}
