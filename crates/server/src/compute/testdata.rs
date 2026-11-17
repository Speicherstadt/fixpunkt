//! Synthetische Aufnahmen für die Tests der Rechenaufträge.

use fixpunkt::pointcloud::{Point3, PointCloud};
use std::path::Path;

pub const X0: f64 = 512_000.0;
pub const Y0: f64 = 5_403_000.0;

/// 40 × 40 m eben auf 300 m, Raster 0,2 m; mit `pit` eine Grube 20 × 20 × 2 m in der Mitte
pub fn write_cloud(path: &Path, pit: bool) {
    write_with(path, |dx, dy| {
        let deep = pit && (10.0..30.0).contains(&dx) && (10.0..30.0).contains(&dy);
        300.0 - if deep { 2.0 } else { 0.0 }
    });
}

pub fn write_with(path: &Path, z: impl Fn(f64, f64) -> f64) {
    let pts: Vec<Point3> = (0..200)
        .flat_map(|j| (0..200).map(move |i| (i, j)))
        .map(|(i, j)| {
            let (dx, dy) = ((i as f64 + 0.5) * 0.2, (j as f64 + 0.5) * 0.2);
            Point3 {
                x: X0 + dx,
                y: Y0 + dy,
                z: z(dx, dy),
            }
        })
        .collect();
    PointCloud::new(pts).write(path, None).unwrap();
}
