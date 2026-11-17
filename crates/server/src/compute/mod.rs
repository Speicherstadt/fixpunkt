//! Rechenaufträge: ruft den Rechenkern (`crates/processing`) direkt als Bibliothek auf.
//! Reine Funktionen über Dateien – ohne Datenbank, damit sie für sich testbar sind.
//! Je Auftragsart ein Modul; hier das Gemeinsame (Rasteroptionen, Prüfen, Bereiche).

pub mod compare;
pub mod export;
pub mod volume;

#[cfg(test)]
pub(crate) mod testdata;

use anyhow::{bail, Context, Result};
use fixpunkt::geom::{BBox, Polygon, Region};
use fixpunkt::grid::{Aggregate, Grid, GridSpec};
use fixpunkt::pointcloud::{PointCloud, ReadOptions};
use fixpunkt::surface::{self, SurfaceOptions};
use fixpunkt::volume::Base;
use fixpunkt::{filter, geotiff};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

/// Dateiendungen, die der Rechenkern lesen kann
pub const SUPPORTED: &[&str] = &[
    "las", "laz", "xyz", "txt", "csv", "pts", "tif", "tiff", "asc",
];

pub fn is_supported(file_name: &str) -> bool {
    Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| SUPPORTED.contains(&e.to_ascii_lowercase().as_str()))
}

/// Einstellungen fürs Rastern, wie in der Kommandozeile
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SurfaceParams {
    pub cell: f64,
    pub ground_only: bool,
    pub classify_ground: bool,
}

impl Default for SurfaceParams {
    fn default() -> Self {
        SurfaceParams {
            cell: 0.25,
            ground_only: false,
            classify_ground: false,
        }
    }
}

impl SurfaceParams {
    pub fn validate(&self) -> Result<()> {
        if !(0.01..=10.0).contains(&self.cell) {
            bail!("Zellgröße muss zwischen 0,01 und 10 m liegen");
        }
        Ok(())
    }

    pub(crate) fn options(&self) -> SurfaceOptions {
        SurfaceOptions {
            cell: self.cell,
            ground_only: self.ground_only,
            classify_ground: self.classify_ground.then(filter::GroundParams::default),
            aggregate: Aggregate::Mean,
            fill: 4,
        }
    }
}

/// Überblick über eine hochgeladene Datei (wie `fixpunkt info`).
pub fn inspect(path: &Path) -> Result<Value> {
    if surface::is_raster(path) {
        let (g, epsg) = if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("asc"))
        {
            (Grid::read_asc(path)?, None)
        } else {
            let r = geotiff::read(path)?;
            (r.grid, r.epsg)
        };
        let st = g.z_stats();
        return Ok(json!({
            "type": "raster", "epsg": epsg, "cols": g.spec.cols, "rows": g.spec.rows, "cell": g.spec.cell,
            "bbox": bbox_json(&g.bbox()), "validCells": g.valid_cells(),
            "zMin": st.map(|s| s.0), "zMax": st.map(|s| s.1),
        }));
    }
    let c = PointCloud::read(path, ReadOptions::default())?;
    let b = c.bbox();
    let (lo, hi) = c.z_range().context("keine Punkte")?;
    let area = (b.width() * b.height()).max(1e-9);
    Ok(json!({
        "type": "pointcloud", "points": c.len(), "bbox": bbox_json(&b),
        "zMin": lo, "zMax": hi, "densityPerM2": c.len() as f64 / area,
    }))
}

fn bbox_json(b: &BBox) -> Value {
    json!({ "minX": b.min_x, "minY": b.min_y, "maxX": b.max_x, "maxY": b.max_y })
}

pub(crate) fn union_bbox(rs: &[Region]) -> Option<BBox> {
    let mut b = BBox::empty();
    for r in rs {
        let rb = r.polygon.bbox();
        b.extend(rb.min_x, rb.min_y);
        b.extend(rb.max_x, rb.max_y);
    }
    (!rs.is_empty()).then_some(b)
}

/// Zellen außerhalb aller Bereiche auf „keine Daten“ setzen (ohne Bereiche: nichts).
pub(crate) fn mask_to_regions(g: &mut Grid, regions: &[Region]) {
    if regions.is_empty() {
        return;
    }
    let mut mask = vec![false; g.spec.len()];
    for r in regions {
        for (m, k) in mask.iter_mut().zip(g.spec.mask(Some(&r.polygon))) {
            *m |= k;
        }
    }
    g.data.iter_mut().zip(mask).for_each(|(v, k)| {
        if !k {
            *v = f32::NAN
        }
    });
}

/// Ausdehnung für eine Aufnahme: die Bereiche, sonst die ganze Aufnahme.
pub(crate) fn surface_box(input: &Path, rs: &[Region], o: &SurfaceOptions) -> Result<BBox> {
    match union_bbox(rs) {
        Some(b) => Ok(b),
        None => surface::extent(input, o),
    }
}

/// Gemeinsames Raster: Bereiche, sonst Überlappung der Aufnahmen.
pub(crate) fn common_spec(inputs: &[&Path], rs: &[Region], o: &SurfaceOptions) -> Result<GridSpec> {
    if let Some(b) = union_bbox(rs) {
        return GridSpec::covering(&b, o.cell);
    }
    let mut bb: Option<BBox> = None;
    for p in inputs {
        let e = surface::extent(p, o)?;
        bb = Some(bb.map_or(e, |b| b.intersection(&e)));
    }
    GridSpec::covering(&bb.context("keine Eingabe")?, o.cell)
}

/// Basis eines Haufwerks aus JSON: `"plane"`, `"lowest"` oder eine Zahl
pub fn parse_base(v: &Value) -> Result<Base> {
    match v {
        Value::String(s) if s == "plane" => Ok(Base::Plane),
        Value::String(s) if s == "lowest" => Ok(Base::Lowest),
        Value::Number(n) => Ok(Base::Height(n.as_f64().context("Höhe")?)),
        _ => bail!("base: \"plane\", \"lowest\" oder eine Höhe"),
    }
}

/// Polygon als GeoJSON-Feature im metrischen System (mit `crs`, damit es nicht umgerechnet wird).
pub fn region_geojson(name: &str, p: &Polygon, epsg: u16) -> Value {
    let ring = |r: &Vec<[f64; 2]>| {
        let mut v: Vec<[f64; 2]> = r.clone();
        if let Some(first) = r.first() {
            v.push(*first);
        }
        v
    };
    let mut rings = vec![ring(&p.exterior)];
    rings.extend(p.holes.iter().map(ring));
    json!({
        "type": "Feature",
        "crs": { "type": "name", "properties": { "name": format!("urn:ogc:def:crs:EPSG::{epsg}") } },
        "properties": { "name": name },
        "geometry": { "type": "Polygon", "coordinates": rings },
    })
}

/// Bereiche aus gespeichertem GeoJSON
pub fn parse_region(name: &str, geojson: &str) -> Result<Region> {
    let mut rs = Polygon::regions_from_geojson_str(geojson, fixpunkt::crs::Utm::default())?;
    let mut r = rs.drain(..).next().context("kein Polygon")?;
    r.name = name.to_string();
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::testdata::{write_cloud, X0, Y0};
    use super::*;

    #[test]
    fn inspect_regions_and_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let b = dir.path().join("b.laz");
        write_cloud(&b, true);
        let info = inspect(&b).unwrap();
        assert_eq!(info["points"], 40_000);
        assert_eq!(info["type"], "pointcloud");

        let poly = Polygon::rect(X0 + 5.0, Y0 + 5.0, X0 + 35.0, Y0 + 35.0);
        let gj = region_geojson("Grube", &poly, 25832).to_string();
        let region = parse_region("Grube", &gj).unwrap();
        assert_eq!(
            region.polygon, poly,
            "metrisches GeoJSON darf nicht umgerechnet werden"
        );

        let mut g = surface::load(&b, None, None, &SurfaceParams::default().options()).unwrap();
        let all = g.valid_cells();
        mask_to_regions(&mut g, &[region]);
        assert!(g.valid_cells() < all && g.valid_cells() > 0);

        assert!(SurfaceParams {
            cell: 0.0,
            ..Default::default()
        }
        .validate()
        .is_err());
        assert!(is_supported("x.LAZ") && is_supported("dgm.tif") && !is_supported("bild.jpg"));
        assert!(matches!(parse_base(&json!("plane")).unwrap(), Base::Plane));
        assert!(matches!(parse_base(&json!(312.5)).unwrap(), Base::Height(h) if h == 312.5));
        assert!(parse_base(&json!(true)).is_err());
    }
}
