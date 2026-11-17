//! Exporte für CAD und Vermessung: GeoTIFF, DXF mit Höhenlinien, LandXML, CSV, OBJ.

use super::{mask_to_regions, surface_box, SurfaceParams};
use anyhow::{bail, Result};
use fixpunkt::export::{self, Dxf, Mesh};
use fixpunkt::geom::Region;
use fixpunkt::geotiff;
use fixpunkt::grid::{Grid, GridSpec};
use fixpunkt::surface;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// Höhenraster als GeoTIFF
    Tif,
    /// Höhenlinien und Grenzen (R12)
    Dxf,
    /// TIN-Oberfläche (LandXML 1.2)
    Landxml,
    /// Rasterpunkte x;y;z
    Csv,
    /// Dreiecksnetz, relativ zur linken unteren Ecke
    Obj,
}

/// Mesh und CSV werden ausgedünnt, damit Dateien für CAD-Programme handlich bleiben
const MAX_POINTS: usize = 250_000;

/// Jede wie vielte Zelle, damit höchstens `MAX_POINTS` Punkte entstehen
pub fn stride_for(cells: usize) -> usize {
    ((cells as f64 / MAX_POINTS as f64).sqrt().ceil() as usize).max(1)
}

fn file_name(f: Format) -> &'static str {
    match f {
        Format::Tif => "dgm.tif",
        Format::Dxf => "dgm.dxf",
        Format::Landxml => "dgm.xml",
        Format::Csv => "dgm.csv",
        Format::Obj => "dgm.obj",
    }
}

/// Schreibt die gewünschten Formate nach `out_dir`; `interval` ist der Höhenlinienabstand in m.
#[allow(clippy::too_many_arguments)]
pub fn export(
    input: &Path,
    formats: &[Format],
    regions: &[Region],
    interval: f64,
    name: &str,
    p: &SurfaceParams,
    epsg: u16,
    out_dir: &Path,
) -> Result<Value> {
    p.validate()?;
    if formats.is_empty() {
        bail!("kein Format gewählt");
    }
    if !(0.05..=50.0).contains(&interval) {
        bail!("Höhenlinienabstand muss zwischen 0,05 und 50 m liegen");
    }
    let o = p.options();
    let spec = GridSpec::covering(&surface_box(input, regions, &o)?, o.cell)?;
    let mut g = surface::load(input, Some(spec), None, &o)?;
    mask_to_regions(&mut g, regions);
    std::fs::create_dir_all(out_dir)?;
    let stride = stride_for(g.valid_cells());
    let mut files = Vec::new();
    let mut contours = 0;
    for &f in formats {
        let path = out_dir.join(file_name(f));
        match f {
            Format::Tif => geotiff::write(&g, &path, Some(epsg), None)?,
            Format::Csv => export::write_grid_csv(&g, &path, stride)?,
            Format::Landxml => {
                Mesh::from_grid(&g, stride, None).write_landxml(&path, name, Some(epsg as u32))?
            }
            Format::Obj => {
                let z0 = g.z_stats().map(|s| s.0.floor()).unwrap_or(0.0);
                Mesh::from_grid(&g, stride, None)
                    .write_obj(&path, [spec.min_x, spec.min_y(), z0])?
            }
            Format::Dxf => contours = write_dxf(&g, regions, interval, &path)?,
        }
        files.push(file_name(f));
    }
    let st = g.z_stats();
    Ok(json!({
        "files": files, "cells": g.valid_cells(), "stride": stride, "contours": contours,
        "zMin": st.map(|s| s.0), "zMax": st.map(|s| s.1), "cell": p.cell,
    }))
}

fn write_dxf(g: &Grid, regions: &[Region], interval: f64, path: &Path) -> Result<usize> {
    let mut d = Dxf::new();
    let cs = export::contours(g, interval)?;
    d.contours(&cs, interval * 5.0);
    for r in regions {
        let pts: Vec<[f64; 3]> = r
            .polygon
            .exterior
            .iter()
            .map(|q| [q[0], q[1], g.sample(q[0], q[1]).unwrap_or(0.0)])
            .collect();
        d.polyline("GRENZE", &pts, true);
    }
    d.write(path)?;
    Ok(cs.len())
}

#[cfg(test)]
mod tests {
    use super::super::testdata::{write_cloud, X0, Y0};
    use super::*;
    use fixpunkt::geom::Polygon;

    #[test]
    fn writes_every_format() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("grube.laz");
        write_cloud(&input, true);
        let out = dir.path().join("job");
        let grube = Region {
            name: "Grube".into(),
            polygon: Polygon::rect(X0 + 5.0, Y0 + 5.0, X0 + 35.0, Y0 + 35.0),
        };
        let all = [
            Format::Tif,
            Format::Dxf,
            Format::Landxml,
            Format::Csv,
            Format::Obj,
        ];
        let r = export(
            &input,
            &all,
            &[grube],
            0.5,
            "Grube",
            &SurfaceParams::default(),
            25832,
            &out,
        )
        .unwrap();
        assert_eq!(r["files"].as_array().unwrap().len(), 5);
        // Grube 2 m tief, Linien alle 0,5 m: 300,0 / 299,5 / 299,0 / 298,5 / 298,0 – mindestens die drei inneren
        assert!(r["contours"].as_u64().unwrap() >= 3, "{r}");
        let dxf = std::fs::read_to_string(out.join("dgm.dxf")).unwrap();
        assert!(dxf.contains("HOEHENLINIEN") && dxf.contains("GRENZE"));
        let xml = std::fs::read_to_string(out.join("dgm.xml")).unwrap();
        assert!(xml.contains("<Surface name=\"Grube\"") && xml.contains("epsgCode=\"25832\""));
        assert_eq!(
            geotiff::read(&out.join("dgm.tif")).unwrap().epsg,
            Some(25832)
        );
        assert!(export(
            &input,
            &[],
            &[],
            0.5,
            "x",
            &SurfaceParams::default(),
            25832,
            &out
        )
        .is_err());
        assert!(export(
            &input,
            &all,
            &[],
            0.0,
            "x",
            &SurfaceParams::default(),
            25832,
            &out
        )
        .is_err());
    }

    #[test]
    fn stride_keeps_files_handy() {
        assert_eq!(stride_for(10_000), 1);
        assert_eq!(stride_for(250_000), 1);
        assert_eq!(stride_for(1_000_000), 2);
        assert_eq!(stride_for(4_000_000), 4);
    }
}
