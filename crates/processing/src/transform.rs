//! Georeferenzierung: Transformation aus identischen Punkten (Passpunkte / GCPs).
//!
//! Verfahren nach Umeyama (1991): kleinste Quadrate über alle Punktpaare, liefert
//! Drehung, Maßstab und Verschiebung (3D-Helmert, 7 Parameter) samt Restklaffen.

use anyhow::{bail, Context, Result};
use nalgebra::{Matrix3, Vector3};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Welche Parameter bestimmt werden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// 7 Parameter: Drehung, Verschiebung, Maßstab (Helmert)
    #[default]
    Similarity,
    /// 6 Parameter: Drehung, Verschiebung (Maßstab fest 1)
    Rigid,
    /// 3 Parameter: nur Verschiebung
    Translation,
}

/// Identischer Punkt: Koordinate im Quellsystem (z. B. Scan) und im Zielsystem (z. B. UTM32).
#[derive(Debug, Clone, PartialEq)]
pub struct ControlPoint {
    pub name: String,
    pub src: [f64; 3],
    pub dst: [f64; 3],
}

/// p_ziel = scale · R · (p_quelle − src_center) + dst_center
#[derive(Debug, Clone, Serialize)]
pub struct Transform {
    pub scale: f64,
    pub rotation: [[f64; 3]; 3],
    pub src_center: [f64; 3],
    pub dst_center: [f64; 3],
}

#[derive(Debug, Clone, Serialize)]
pub struct Residual {
    pub name: String,
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
    /// Lageabweichung
    pub d2: f64,
    /// Raumabweichung
    pub d3: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FitReport {
    pub mode: Mode,
    pub points: usize,
    pub transform: Transform,
    /// Maßstabsabweichung in ppm (mm/km)
    pub scale_ppm: f64,
    /// Drehwinkel um x, y, z in Grad (R = Rz·Ry·Rx)
    pub rotation_deg: [f64; 3],
    pub translation: [f64; 3],
    pub residuals: Vec<Residual>,
    pub rms_xy: f64,
    pub rms_z: f64,
    pub rms_3d: f64,
    pub max_3d: f64,
}

impl Transform {
    pub fn identity() -> Self {
        Transform {
            scale: 1.0,
            rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            src_center: [0.0; 3],
            dst_center: [0.0; 3],
        }
    }

    pub fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        let r = &self.rotation;
        let d = [
            p[0] - self.src_center[0],
            p[1] - self.src_center[1],
            p[2] - self.src_center[2],
        ];
        let mut out = [0.0; 3];
        for i in 0..3 {
            out[i] = self.scale * (r[i][0] * d[0] + r[i][1] * d[1] + r[i][2] * d[2])
                + self.dst_center[i];
        }
        out
    }

    /// Verschiebungsvektor in der Form p' = s·R·p + t
    pub fn translation(&self) -> [f64; 3] {
        let z = self.apply([0.0; 3]);
        [z[0], z[1], z[2]]
    }

    pub fn rotation_angles_deg(&self) -> [f64; 3] {
        let r = &self.rotation;
        let ry = (-r[2][0]).clamp(-1.0, 1.0).asin();
        let rx = r[2][1].atan2(r[2][2]);
        let rz = r[1][0].atan2(r[0][0]);
        [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()]
    }
}

/// Bestimmt die Transformation aus mindestens 3 (Translation: 1) identischen Punkten.
pub fn fit(points: &[ControlPoint], mode: Mode) -> Result<FitReport> {
    let min = if mode == Mode::Translation { 1 } else { 3 };
    if points.len() < min {
        bail!(
            "mindestens {min} Passpunkte nötig, {} vorhanden",
            points.len()
        );
    }
    let n = points.len() as f64;
    let v = |a: [f64; 3]| Vector3::new(a[0], a[1], a[2]);
    let mu_s = points.iter().map(|p| v(p.src)).sum::<Vector3<f64>>() / n;
    let mu_d = points.iter().map(|p| v(p.dst)).sum::<Vector3<f64>>() / n;

    let (rot, scale) = if mode == Mode::Translation {
        (Matrix3::identity(), 1.0)
    } else {
        let mut cov = Matrix3::zeros();
        let mut var_s = 0.0;
        for p in points {
            let (s, d) = (v(p.src) - mu_s, v(p.dst) - mu_d);
            cov += d * s.transpose();
            var_s += s.norm_squared();
        }
        cov /= n;
        var_s /= n;
        let svd = cov.svd(true, true);
        let (u, vt) = (svd.u.context("SVD")?, svd.v_t.context("SVD")?);
        let mut sign = Matrix3::identity();
        if (u.determinant() * vt.determinant()) < 0.0 {
            sign[(2, 2)] = -1.0;
        }
        // Rang prüfen: Punkte dürfen nicht (fast) auf einer Geraden liegen
        let sv = svd.singular_values;
        if sv[1] < 1e-10 * sv[0].max(1e-300) {
            bail!("Passpunkte liegen auf einer Geraden – Drehung nicht bestimmbar");
        }
        let rot = u * sign * vt;
        let scale = if mode == Mode::Similarity {
            (svd.singular_values.component_mul(&sign.diagonal())).sum() / var_s
        } else {
            1.0
        };
        (rot, scale)
    };

    let mut rotation = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            rotation[i][j] = rot[(i, j)];
        }
    }
    let t = Transform {
        scale,
        rotation,
        src_center: [mu_s.x, mu_s.y, mu_s.z],
        dst_center: [mu_d.x, mu_d.y, mu_d.z],
    };

    let residuals: Vec<Residual> = points
        .iter()
        .map(|p| {
            let q = t.apply(p.src);
            let (dx, dy, dz) = (p.dst[0] - q[0], p.dst[1] - q[1], p.dst[2] - q[2]);
            Residual {
                name: p.name.clone(),
                dx,
                dy,
                dz,
                d2: (dx * dx + dy * dy).sqrt(),
                d3: (dx * dx + dy * dy + dz * dz).sqrt(),
            }
        })
        .collect();
    let rms = |f: &dyn Fn(&Residual) -> f64| {
        (residuals.iter().map(|r| f(r).powi(2)).sum::<f64>() / n).sqrt()
    };
    Ok(FitReport {
        mode,
        points: points.len(),
        scale_ppm: (scale - 1.0) * 1e6,
        rotation_deg: t.rotation_angles_deg(),
        translation: t.translation(),
        rms_xy: rms(&|r| r.d2),
        rms_z: rms(&|r| r.dz),
        rms_3d: rms(&|r| r.d3),
        max_3d: residuals.iter().map(|r| r.d3).fold(0.0, f64::max),
        residuals,
        transform: t,
    })
}

/// Wendet die Transformation auf eine Punktwolke an: LAS/LAZ → LAS/LAZ mit allen Attributen, sonst über
/// die Punkte (Text rein, LAS/LAZ oder Text raus je nach Endung von `output`). Gibt die Zahl der Punkte.
pub fn apply_to_file(t: &Transform, input: &Path, output: &Path) -> Result<u64> {
    if crate::pointcloud::is_las(input) && crate::pointcloud::is_las(output) {
        let f = |p: [f64; 3]| t.apply(p);
        return crate::pointcloud::rewrite_las(input, output, Some(&f), |_, p| {
            let q = t.apply([p.x, p.y, p.z]);
            (p.x, p.y, p.z) = (q[0], q[1], q[2]);
            true
        });
    }
    // Text oder E57: Farben mitnehmen, wenn die Ausgabe LAS/LAZ ist
    let crate::colored::Colored {
        cloud: mut c,
        colors,
    } = crate::colored::read_colored(input, Default::default())?;
    c.map(|p| {
        let q = t.apply([p.x, p.y, p.z]);
        crate::pointcloud::Point3 {
            x: q[0],
            y: q[1],
            z: q[2],
        }
    });
    match colors {
        Some(rgb) if crate::pointcloud::is_las(output) => {
            crate::colored::write_las_colored(&c, &rgb, output, None)?
        }
        _ => c.write(output, None)?,
    }
    Ok(c.len() as u64)
}

/// Liest Passpunkte aus CSV: `name, x_quelle, y_quelle, z_quelle, x_ziel, y_ziel, z_ziel`
/// (Trennzeichen Komma, Semikolon, Tab oder Leerzeichen; Kopfzeile wird übersprungen).
pub fn read_control_points(path: &Path) -> Result<Vec<ControlPoint>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("{} nicht lesbar", path.display()))?;
    parse_control_points(&text)
}

pub fn parse_control_points(text: &str) -> Result<Vec<ControlPoint>> {
    let mut out = Vec::new();
    for (no, line) in text.lines().enumerate() {
        let f: Vec<&str> = line
            .split([',', ';', '\t', ' '])
            .filter(|s| !s.is_empty())
            .collect();
        if f.is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if f.len() < 7 {
            bail!("Zeile {}: erwartet name + 6 Koordinaten", no + 1);
        }
        let nums: Result<Vec<f64>, _> = f[1..7].iter().map(|s| s.parse::<f64>()).collect();
        match nums {
            Ok(v) => out.push(ControlPoint {
                name: f[0].to_string(),
                src: [v[0], v[1], v[2]],
                dst: [v[3], v[4], v[5]],
            }),
            Err(_) if out.is_empty() => continue, // Kopfzeile
            Err(_) => bail!("Zeile {}: ungültige Zahl", no + 1),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rot(rx: f64, ry: f64, rz: f64) -> Matrix3<f64> {
        let (rx, ry, rz) = (rx.to_radians(), ry.to_radians(), rz.to_radians());
        Matrix3::new(
            rz.cos(),
            -rz.sin(),
            0.0,
            rz.sin(),
            rz.cos(),
            0.0,
            0.0,
            0.0,
            1.0,
        ) * Matrix3::new(
            ry.cos(),
            0.0,
            ry.sin(),
            0.0,
            1.0,
            0.0,
            -ry.sin(),
            0.0,
            ry.cos(),
        ) * Matrix3::new(
            1.0,
            0.0,
            0.0,
            0.0,
            rx.cos(),
            -rx.sin(),
            0.0,
            rx.sin(),
            rx.cos(),
        )
    }

    fn make(s: f64, r: Matrix3<f64>, t: Vector3<f64>, noise: f64) -> Vec<ControlPoint> {
        let src = [
            [0.0, 0.0, 0.0],
            [30.0, 2.0, 1.0],
            [5.0, 40.0, -2.0],
            [35.0, 45.0, 3.0],
            [18.0, 20.0, 6.0],
        ];
        src.iter()
            .enumerate()
            .map(|(i, p)| {
                let q = s * r * Vector3::new(p[0], p[1], p[2]) + t;
                let e = noise * [1.0, -1.0, 0.5, -0.5, 0.0][i];
                ControlPoint {
                    name: format!("P{i}"),
                    src: *p,
                    dst: [q.x + e, q.y - e, q.z + e],
                }
            })
            .collect()
    }

    #[test]
    fn recovers_helmert_in_utm() {
        let r = rot(0.3, -0.2, 37.0);
        let t = Vector3::new(512_345.678, 5_412_345.321, 312.5);
        let gcps = make(1.0002, r, t, 0.0);
        let rep = fit(&gcps, Mode::Similarity).unwrap();
        assert!(rep.rms_3d < 1e-6, "{}", rep.rms_3d);
        assert!((rep.scale_ppm - 200.0).abs() < 1e-3);
        let [ax, ay, az] = rep.rotation_deg;
        assert!((ax - 0.3).abs() < 1e-8 && (ay + 0.2).abs() < 1e-8 && (az - 37.0).abs() < 1e-8);
        let q = rep.transform.apply([10.0, 10.0, 1.0]);
        let e = 1.0002 * r * Vector3::new(10.0, 10.0, 1.0) + t;
        assert!(
            (q[0] - e.x).abs() < 1e-6 && (q[1] - e.y).abs() < 1e-6 && (q[2] - e.z).abs() < 1e-6
        );
    }

    #[test]
    fn rigid_keeps_scale_and_reports_residuals() {
        let gcps = make(
            1.0,
            rot(0.0, 0.0, -12.0),
            Vector3::new(500_000.0, 5_400_000.0, 250.0),
            0.01,
        );
        let rep = fit(&gcps, Mode::Rigid).unwrap();
        assert_eq!(rep.transform.scale, 1.0);
        assert!(rep.rms_3d > 0.001 && rep.rms_3d < 0.02, "{}", rep.rms_3d);
        assert_eq!(rep.residuals.len(), 5);
    }

    #[test]
    fn translation_only_and_errors() {
        let gcps = make(1.0, Matrix3::identity(), Vector3::new(1.0, 2.0, 3.0), 0.0);
        let rep = fit(&gcps[..1], Mode::Translation).unwrap();
        assert!((rep.translation[2] - 3.0).abs() < 1e-12);
        assert!(fit(&gcps[..2], Mode::Similarity).is_err());
        let line: Vec<_> = (0..4)
            .map(|i| ControlPoint {
                name: "".into(),
                src: [i as f64, 0.0, 0.0],
                dst: [i as f64, 0.0, 0.0],
            })
            .collect();
        assert!(fit(&line, Mode::Rigid).is_err());
    }

    #[test]
    fn parse_csv_with_header() {
        let t = "name;xs;ys;zs;xd;yd;zd\nA;0;0;0;500000;5400000;300\n# Kommentar\nB 1 0 0 500001 5400000 300\n";
        let p = parse_control_points(t).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[1].dst, [500_001.0, 5_400_000.0, 300.0]);
    }
}
