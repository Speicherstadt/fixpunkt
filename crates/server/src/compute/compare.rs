//! Soll/Ist: Aufnahme gegen Sohlhöhe, geneigte Ebene oder ein Soll-Geländemodell.

use super::{surface_box, SurfaceParams};
use anyhow::{bail, Result};
use fixpunkt::compare::{self, Design};
use fixpunkt::geom::Region;
use fixpunkt::geotiff;
use fixpunkt::grid::GridSpec;
use fixpunkt::surface;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Sollfläche, wie sie der Auftrag beschreibt (Soll-DGM schon als Datei)
#[derive(Debug, Clone, PartialEq)]
pub enum DesignInput {
    Height(f64),
    Slope {
        x: f64,
        y: f64,
        z: f64,
        percent: f64,
        azimuth_deg: f64,
    },
    Surface(PathBuf),
}

/// Abweichung Ist − Soll je Zelle, Statistik, noch ab-/aufzutragende Massen; schreibt `abweichung.tif`.
pub fn compare(
    scan: &Path,
    design: &DesignInput,
    region: Option<&Region>,
    tolerance: f64,
    p: &SurfaceParams,
    epsg: u16,
    out_dir: &Path,
) -> Result<Value> {
    p.validate()?;
    if !(0.0..=1.0).contains(&tolerance) {
        bail!("Toleranz muss zwischen 0 und 1 m liegen");
    }
    let o = p.options();
    let regions: Vec<Region> = region.cloned().into_iter().collect();
    let spec = GridSpec::covering(&surface_box(scan, &regions, &o)?, o.cell)?;
    let g = surface::load(scan, Some(spec), None, &o)?;
    let d = match design {
        DesignInput::Height(h) => Design::Height(*h),
        DesignInput::Slope {
            x,
            y,
            z,
            percent,
            azimuth_deg,
        } => Design::Slope {
            x: *x,
            y: *y,
            z: *z,
            percent: *percent,
            azimuth_deg: *azimuth_deg,
        },
        DesignInput::Surface(path) => Design::Surface(surface::load(path, Some(spec), None, &o)?),
    };
    let (report, diff) = compare::compare(&g, &d, region.map(|r| &r.polygon), tolerance)?;
    std::fs::create_dir_all(out_dir)?;
    geotiff::write(&diff, &out_dir.join("abweichung.tif"), Some(epsg), None)?;
    Ok(json!({
        "deviation": report,
        "region": region.map(|r| r.name.clone()),
        "cell": p.cell,
        "files": ["abweichung.tif"],
    }))
}

#[cfg(test)]
mod tests {
    use super::super::testdata::{write_cloud, write_with, X0, Y0};
    use super::*;
    use fixpunkt::geom::Polygon;

    #[test]
    fn sole_height_slope_and_design_surface() {
        let dir = tempfile::tempdir().unwrap();
        let scan = dir.path().join("ist.laz");
        // Sohle 3 cm zu hoch auf 296,03 m
        write_with(&scan, |_, _| 296.03);
        let sohle = Region {
            name: "Sohle".into(),
            polygon: Polygon::rect(X0 + 5.0, Y0 + 5.0, X0 + 25.0, Y0 + 25.0),
        };
        let out = dir.path().join("job");
        let p = SurfaceParams::default();
        let r = compare(
            &scan,
            &DesignInput::Height(296.0),
            Some(&sohle),
            0.05,
            &p,
            25832,
            &out,
        )
        .unwrap();
        let d = &r["deviation"];
        assert!((d["mean"].as_f64().unwrap() - 0.03).abs() < 1e-3, "{r}");
        assert_eq!(d["within_tolerance_percent"], 100.0);
        // 20 × 20 m × 3 cm = 12 m³ noch abzutragen
        assert!(
            (d["volume"]["fill_m3"].as_f64().unwrap() - 12.0).abs() < 0.2,
            "{r}"
        );
        assert!(out.join("abweichung.tif").exists());

        // geneigte Ebene: 1 % fallend nach Osten ab x0 auf 296,03 → am Ostrand 0,4 m darunter
        let slope = DesignInput::Slope {
            x: X0,
            y: Y0,
            z: 296.03,
            percent: 1.0,
            azimuth_deg: 90.0,
        };
        let r = compare(&scan, &slope, None, 0.01, &p, 25832, &out).unwrap();
        assert!(r["deviation"]["max"].as_f64().unwrap() > 0.35, "{r}");

        // Soll-DGM: Urgelände gegen Grube
        let soll = dir.path().join("soll.laz");
        let ist = dir.path().join("grube.laz");
        write_cloud(&soll, true);
        write_cloud(&ist, false);
        let r = compare(
            &ist,
            &DesignInput::Surface(soll),
            None,
            0.05,
            &p,
            25832,
            &out,
        )
        .unwrap();
        // noch 800 m³ auszuheben
        assert!(
            (r["deviation"]["volume"]["fill_m3"].as_f64().unwrap() - 800.0).abs() < 10.0,
            "{r}"
        );
        assert!(compare(&ist, &DesignInput::Height(1.0), None, 5.0, &p, 25832, &out).is_err());
    }
}
