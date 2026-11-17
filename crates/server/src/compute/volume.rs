//! Massen: Aushub (Differenz), Haufwerk, Zeitreihe, Höhenraster.

use super::{common_spec, mask_to_regions, union_bbox, SurfaceParams};
use anyhow::{bail, Result};
use fixpunkt::geom::Region;
use fixpunkt::geotiff;
use fixpunkt::surface;
use fixpunkt::volume::{self, Base};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Schichten für den Abtrag: Namen (einer mehr als Grenzen) und Unterkanten in m unter `before`
pub type Layers = (Vec<String>, Vec<f64>);

/// Aushub/Auftrag zwischen zwei Aufnahmen, je Bereich; schreibt `diff.tif` nach `out_dir`. Mit Schichten steht
/// je Bereich zusätzlich der Abtrag je Schicht unter `layers`.
pub fn volume(
    before: &Path,
    after: &Path,
    regions: &[Region],
    p: &SurfaceParams,
    epsg: u16,
    out_dir: &Path,
    layers: Option<&Layers>,
) -> Result<Value> {
    p.validate()?;
    let o = p.options();
    let spec = common_spec(&[before, after], regions, &o)?;
    let g0 = surface::load(before, Some(spec), None, &o)?;
    let g1 = surface::load(after, Some(spec), None, &o)?;
    let (whole, mut diff) = volume::difference(&g0, &g1, None)?;
    mask_to_regions(&mut diff, regions);
    std::fs::create_dir_all(out_dir)?;
    geotiff::write(&diff, &out_dir.join("diff.tif"), Some(epsg), None)?;
    let (per, total) = if regions.is_empty() {
        (Vec::new(), whole)
    } else {
        volume::difference_regions(&g0, &g1, regions)?
    };
    let mut per = serde_json::to_value(per)?;
    if let (Some((names, bottoms)), Some(list)) = (layers, per.as_array_mut()) {
        for (v, r) in list.iter_mut().zip(regions) {
            let cut = volume::cut_by_layers(&g0, &g1, Some(&r.polygon), bottoms)?;
            v["layers"] = names
                .iter()
                .zip(cut)
                .map(|(n, c)| json!({"name": n, "cut_m3": c}))
                .collect();
        }
    }
    Ok(json!({ "regions": per, "total": total, "cell": p.cell, "files": ["diff.tif"] }))
}

/// Haufwerk(e) gegen eine Basisfläche
pub fn stockpile(
    surface_path: &Path,
    regions: &[Region],
    base: Base,
    p: &SurfaceParams,
) -> Result<Value> {
    p.validate()?;
    if regions.is_empty() {
        bail!("mindestens ein Bereich nötig");
    }
    let g = surface::load(surface_path, None, union_bbox(regions), &p.options())?;
    let mut per = Vec::new();
    for r in regions {
        per.push(volume::RegionReport {
            name: r.name.clone(),
            boundary_area_m2: r.polygon.area(),
            volume: volume::stockpile(&g, &r.polygon, base)?,
        });
    }
    let total = volume::sum(per.iter().map(|r| &r.volume));
    Ok(json!({ "regions": per, "total": total, "cell": p.cell }))
}

/// Höhenraster als GeoTIFF (`dem.tif`)
pub fn dem(input: &Path, p: &SurfaceParams, epsg: u16, out_dir: &Path) -> Result<Value> {
    p.validate()?;
    let g = surface::load(input, None, None, &p.options())?;
    std::fs::create_dir_all(out_dir)?;
    geotiff::write(&g, &out_dir.join("dem.tif"), Some(epsg), None)?;
    let st = g.z_stats();
    Ok(json!({
        "cols": g.spec.cols, "rows": g.spec.rows, "cell": g.spec.cell, "validCells": g.valid_cells(),
        "zMin": st.map(|s| s.0), "zMax": st.map(|s| s.1), "files": ["dem.tif"],
    }))
}

/// Fortschritt über die Bauphase: jede Aufnahme gegen die erste (Urgelände) und gegen die vorige.
/// `captures`: (Name, Datei) in zeitlicher Reihenfolge, die erste ist die Basis.
pub fn series(
    captures: &[(String, PathBuf)],
    region: Option<&Region>,
    p: &SurfaceParams,
) -> Result<Value> {
    p.validate()?;
    if captures.len() < 2 {
        bail!("mindestens zwei Aufnahmen nötig");
    }
    let o = p.options();
    let paths: Vec<&Path> = captures.iter().map(|(_, p)| p.as_path()).collect();
    let regions: Vec<Region> = region.cloned().into_iter().collect();
    let spec = common_spec(&paths, &regions, &o)?;
    let base = surface::load(paths[0], Some(spec), None, &o)?;
    let mut epochs = Vec::new();
    for (name, path) in &captures[1..] {
        epochs.push((name.clone(), surface::load(path, Some(spec), None, &o)?));
    }
    let steps = volume::series(&base, &epochs, region.map(|r| &r.polygon))?;
    Ok(json!({
        "base": captures[0].0,
        "region": region.map(|r| r.name.clone()),
        "steps": steps,
        "cell": p.cell,
    }))
}

#[cfg(test)]
mod tests {
    use super::super::testdata::{write_cloud, write_with, X0, Y0};
    use super::*;
    use fixpunkt::geom::Polygon;

    fn grube() -> Region {
        Region {
            name: "Grube".into(),
            polygon: Polygon::rect(X0 + 5.0, Y0 + 5.0, X0 + 35.0, Y0 + 35.0),
        }
    }

    #[test]
    fn volume_dem_stockpile() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.laz"), dir.path().join("b.laz"));
        write_cloud(&a, false);
        write_cloud(&b, true);
        let out = dir.path().join("job");
        let v = volume(
            &a,
            &b,
            &[grube()],
            &SurfaceParams::default(),
            25832,
            &out,
            None,
        )
        .unwrap();
        // 20 × 20 × 2 = 800 m³
        let cut = v["regions"][0]["cut_m3"].as_f64().unwrap();
        assert!((cut - 800.0).abs() < 8.0, "{v}");
        assert!(v["regions"][0].get("layers").is_none());
        // Schichten: bis 0,5 m und darunter → ein Viertel und drei Viertel
        let layers = (vec!["Oberboden".to_string(), "Lehm".to_string()], vec![0.5]);
        let l = volume(
            &a,
            &b,
            &[grube()],
            &SurfaceParams::default(),
            25832,
            &out,
            Some(&layers),
        )
        .unwrap();
        let l = &l["regions"][0]["layers"];
        assert_eq!(l[0]["name"], "Oberboden");
        let (top, rest) = (
            l[0]["cut_m3"].as_f64().unwrap(),
            l[1]["cut_m3"].as_f64().unwrap(),
        );
        assert!((top + rest - cut).abs() < 1e-6);
        assert!((top / cut - 0.25).abs() < 0.01, "{l}");
        assert_eq!(
            geotiff::read(&out.join("diff.tif")).unwrap().epsg,
            Some(25832)
        );

        let d = dem(
            &a,
            &SurfaceParams {
                cell: 1.0,
                ..Default::default()
            },
            25832,
            &out,
        )
        .unwrap();
        assert_eq!(d["cols"], 40);

        let s = stockpile(
            &b,
            &[grube()],
            Base::Height(299.0),
            &SurfaceParams::default(),
        )
        .unwrap();
        // 30 × 30 × 1 m über 299 minus Grube (20 × 20 × 1 m unter 299 zählt als Abtrag)
        assert!(
            (s["total"]["fill_m3"].as_f64().unwrap() - 500.0).abs() < 8.0,
            "{s}"
        );
        assert!(stockpile(&b, &[], Base::Plane, &SurfaceParams::default()).is_err());
    }

    #[test]
    fn series_accumulates() {
        let dir = tempfile::tempdir().unwrap();
        let files: Vec<(String, PathBuf)> = [0.0, 1.0, 2.0]
            .iter()
            .enumerate()
            .map(|(i, depth)| {
                let p = dir.path().join(format!("{i}.laz"));
                write_with(&p, |dx, dy| {
                    let inside = (10.0..30.0).contains(&dx) && (10.0..30.0).contains(&dy);
                    300.0 - if inside { *depth } else { 0.0 }
                });
                (format!("Flug {i}"), p)
            })
            .collect();
        let r = series(&files, Some(&grube()), &SurfaceParams::default()).unwrap();
        let steps = r["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 2);
        // je Flug 1 m tiefer auf 400 m²: 400 m³ seit dem vorigen, 800 m³ seit dem Urgelände
        assert!(
            (steps[1]["since_previous"]["cut_m3"].as_f64().unwrap() - 400.0).abs() < 5.0,
            "{r}"
        );
        assert!(
            (steps[1]["since_start"]["cut_m3"].as_f64().unwrap() - 800.0).abs() < 8.0,
            "{r}"
        );
        assert_eq!(steps[0]["label"], "Flug 1");
        assert_eq!(r["base"], "Flug 0");
        assert!(series(&files[..1], None, &SurfaceParams::default()).is_err());
    }
}
