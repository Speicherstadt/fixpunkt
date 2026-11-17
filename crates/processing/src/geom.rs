//! Geometrie: Bounding-Box und Polygon (Abrechnungsgrenze / Geofence).
//!
//! Alle Koordinaten sind projiziert und metrisch (z. B. ETRS89/UTM32, EPSG:25832).

use crate::crs::Utm;
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::Path;

/// Achsenparallele Box in der Lage (x = Rechtswert, y = Hochwert).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BBox {
    pub fn empty() -> Self {
        BBox {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        }
    }

    pub fn is_empty(&self) -> bool {
        !(self.min_x <= self.max_x && self.min_y <= self.max_y)
    }

    pub fn extend(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    pub fn intersection(&self, o: &BBox) -> BBox {
        BBox {
            min_x: self.min_x.max(o.min_x),
            min_y: self.min_y.max(o.min_y),
            max_x: self.max_x.min(o.max_x),
            max_y: self.max_y.min(o.max_y),
        }
    }

    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }
}

/// Einfaches Polygon: eine Außenkontur, optional Löcher.
/// Ringe werden ohne doppelten Schlusspunkt gespeichert.
#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    pub exterior: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
}

impl Polygon {
    pub fn new(exterior: Vec<[f64; 2]>) -> Result<Self> {
        Self::with_holes(exterior, Vec::new())
    }

    pub fn with_holes(exterior: Vec<[f64; 2]>, holes: Vec<Vec<[f64; 2]>>) -> Result<Self> {
        let exterior = normalize_ring(exterior)?;
        let holes = holes
            .into_iter()
            .map(normalize_ring)
            .collect::<Result<_>>()?;
        Ok(Polygon { exterior, holes })
    }

    /// Rechteck – praktisch für Tests und schnelle Auswertungen.
    pub fn rect(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Polygon {
            exterior: vec![
                [min_x, min_y],
                [max_x, min_y],
                [max_x, max_y],
                [min_x, max_y],
            ],
            holes: Vec::new(),
        }
    }

    pub fn bbox(&self) -> BBox {
        let mut b = BBox::empty();
        for p in &self.exterior {
            b.extend(p[0], p[1]);
        }
        b
    }

    /// Horizontale Fläche in m² (Außenkontur minus Löcher).
    pub fn area(&self) -> f64 {
        ring_area(&self.exterior).abs() - self.holes.iter().map(|h| ring_area(h).abs()).sum::<f64>()
    }

    /// Umfang der Außenkontur in m.
    pub fn perimeter(&self) -> f64 {
        let n = self.exterior.len();
        (0..n)
            .map(|i| {
                let a = self.exterior[i];
                let b = self.exterior[(i + 1) % n];
                ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
            })
            .sum()
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        ring_contains(&self.exterior, x, y) && !self.holes.iter().any(|h| ring_contains(h, x, y))
    }

    /// Punkte entlang der Außenkontur, ungefähr alle `step` Meter (inkl. Eckpunkte).
    pub fn densify_exterior(&self, step: f64) -> Vec<[f64; 2]> {
        let n = self.exterior.len();
        let mut out = Vec::new();
        for i in 0..n {
            let a = self.exterior[i];
            let b = self.exterior[(i + 1) % n];
            let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
            let k = ((len / step).ceil() as usize).max(1);
            for s in 0..k {
                let t = s as f64 / k as f64;
                out.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
            }
        }
        out
    }

    /// Liest das erste Polygon aus einer GeoJSON-Datei (Polygon, Feature oder FeatureCollection).
    ///
    /// Metrische Koordinaten werden unverändert übernommen (müssen im System der Punktwolke sein).
    /// Längen-/Breitengrad (GeoJSON-Standard, ETRS89 ≈ WGS84) wird nach ETRS89/UTM32 umgerechnet.
    pub fn from_geojson_file(path: &Path) -> Result<Self> {
        Ok(Self::regions_from_geojson_file(path)?.remove(0).polygon)
    }

    pub fn from_geojson_str(text: &str) -> Result<Self> {
        Ok(Self::regions_from_geojson_str(text, Utm::default())?
            .remove(0)
            .polygon)
    }

    /// Alle Polygone einer GeoJSON-Datei als benannte Bereiche (Geofences).
    pub fn regions_from_geojson_file(path: &Path) -> Result<Vec<Region>> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("Grenze {} nicht lesbar", path.display()))?;
        Self::regions_from_geojson_str(&text, Utm::default())
    }

    /// Wie [`regions_from_geojson_file`](Self::regions_from_geojson_file); `utm` = Zielzone für Grad-Koordinaten.
    pub fn regions_from_geojson_str(text: &str, utm: Utm) -> Result<Vec<Region>> {
        let v: Value = serde_json::from_str(text).context("ungültiges GeoJSON")?;
        let metric = declares_metric_crs(&v);
        let mut found = Vec::new();
        collect_polygons(&v, None, &mut found);
        if found.is_empty() {
            bail!("kein Polygon im GeoJSON gefunden");
        }
        let mut out = Vec::new();
        for (i, (name, rings)) in found.into_iter().enumerate() {
            let mut parsed = rings.iter().map(parse_ring).collect::<Result<Vec<_>>>()?;
            if parsed.is_empty() {
                bail!("Polygon ohne Ringe");
            }
            if !metric && looks_geographic(parsed.iter().flatten()) {
                for ring in &mut parsed {
                    for p in ring.iter_mut() {
                        let (e, n) = utm.forward(p[1], p[0]);
                        *p = [e, n];
                    }
                }
            }
            let exterior = parsed.remove(0);
            let polygon = Polygon::with_holes(exterior, parsed)?;
            out.push(Region {
                name: name.unwrap_or_else(|| format!("Bereich {}", i + 1)),
                polygon,
            });
        }
        Ok(out)
    }
}

fn looks_geographic<'a>(mut pts: impl Iterator<Item = &'a [f64; 2]>) -> bool {
    pts.all(|p| p[0].abs() <= 180.0 && p[1].abs() <= 90.0)
}

/// `true`, wenn das (alte) GeoJSON-Feld `crs` ein anderes System als Länge/Breite nennt,
/// z. B. `urn:ogc:def:crs:EPSG::25832` aus einem QGIS-Export.
fn declares_metric_crs(v: &Value) -> bool {
    let Some(name) = v.pointer("/crs/properties/name").and_then(Value::as_str) else {
        return false;
    };
    let n = name.to_ascii_uppercase();
    !(n.contains("CRS84")
        || n.ends_with(":4326")
        || n.ends_with(":4258")
        || n.ends_with("::4326")
        || n.ends_with("::4258"))
}

/// Benannter Abrechnungsbereich
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub name: String,
    pub polygon: Polygon,
}

fn feature_name(v: &Value) -> Option<String> {
    let p = v.get("properties")?;
    ["name", "Name", "NAME", "bezeichnung", "Bezeichnung", "id"]
        .iter()
        .find_map(|k| p.get(*k))
        .map(|n| {
            n.as_str()
                .map(str::to_string)
                .unwrap_or_else(|| n.to_string())
        })
}

fn collect_polygons<'a>(
    v: &'a Value,
    name: Option<String>,
    out: &mut Vec<(Option<String>, &'a Vec<Value>)>,
) {
    match v.get("type").and_then(Value::as_str) {
        Some("Polygon") => {
            if let Some(c) = v.get("coordinates").and_then(Value::as_array) {
                out.push((name, c));
            }
        }
        Some("MultiPolygon") => {
            if let Some(ps) = v.get("coordinates").and_then(Value::as_array) {
                let many = ps.len() > 1;
                for (i, c) in ps.iter().filter_map(Value::as_array).enumerate() {
                    let n = if many {
                        name.clone().map(|n| format!("{n} ({})", i + 1))
                    } else {
                        name.clone()
                    };
                    out.push((n, c));
                }
            }
        }
        Some("Feature") => {
            if let Some(g) = v.get("geometry") {
                collect_polygons(g, feature_name(v).or(name), out);
            }
        }
        Some("FeatureCollection") => {
            for f in v
                .get("features")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                collect_polygons(f, None, out);
            }
        }
        Some("GeometryCollection") => {
            for g in v
                .get("geometries")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                collect_polygons(g, name.clone(), out);
            }
        }
        _ => {}
    }
}

/// Liest eine Linie (LineString) aus GeoJSON – z. B. Profilachse oder Leitungsverlauf.
pub fn line_from_geojson_str(text: &str, utm: Utm) -> Result<Vec<[f64; 2]>> {
    fn find(v: &Value) -> Option<&Value> {
        match v.get("type")?.as_str()? {
            "LineString" => v.get("coordinates"),
            "MultiLineString" => v.get("coordinates")?.get(0),
            "Feature" => find(v.get("geometry")?),
            "FeatureCollection" => v.get("features")?.as_array()?.iter().find_map(find),
            _ => None,
        }
    }
    let v: Value = serde_json::from_str(text).context("ungültiges GeoJSON")?;
    let mut pts = parse_ring(find(&v).context("keine Linie im GeoJSON gefunden")?)?;
    if pts.len() < 2 {
        bail!("Linie braucht mindestens 2 Punkte");
    }
    if !declares_metric_crs(&v) && looks_geographic(pts.iter()) {
        for p in pts.iter_mut() {
            let (e, n) = utm.forward(p[1], p[0]);
            *p = [e, n];
        }
    }
    Ok(pts)
}

/// Linie aus Datei: GeoJSON oder Text mit `x y` je Zeile.
pub fn line_from_file(path: &Path) -> Result<Vec<[f64; 2]>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("{} nicht lesbar", path.display()))?;
    if text.trim_start().starts_with('{') {
        return line_from_geojson_str(&text, Utm::default());
    }
    let pts: Vec<[f64; 2]> = text
        .lines()
        .filter_map(|l| {
            let v: Vec<f64> = l
                .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
                .filter(|s| !s.is_empty())
                .map_while(|s| s.parse().ok())
                .collect();
            (v.len() >= 2).then(|| [v[0], v[1]])
        })
        .collect();
    if pts.len() < 2 {
        bail!(
            "{}: Linie braucht mindestens 2 Punkte (x y je Zeile)",
            path.display()
        );
    }
    Ok(pts)
}

fn parse_ring(v: &Value) -> Result<Vec<[f64; 2]>> {
    v.as_array()
        .context("Ring ist kein Array")?
        .iter()
        .map(|p| {
            let x = p
                .get(0)
                .and_then(Value::as_f64)
                .context("Koordinate ohne x")?;
            let y = p
                .get(1)
                .and_then(Value::as_f64)
                .context("Koordinate ohne y")?;
            Ok([x, y])
        })
        .collect()
}

fn normalize_ring(mut ring: Vec<[f64; 2]>) -> Result<Vec<[f64; 2]>> {
    if ring.len() >= 2 && ring.first() == ring.last() {
        ring.pop();
    }
    if ring.len() < 3 {
        bail!("Polygonring braucht mindestens 3 Punkte");
    }
    Ok(ring)
}

/// Vorzeichenbehaftete Fläche (Gaußsche Trapezformel).
fn ring_area(ring: &[[f64; 2]]) -> f64 {
    let n = ring.len();
    // relativ zum ersten Punkt rechnen, sonst Auslöschung bei UTM-Koordinaten
    let [ox, oy] = ring[0];
    let mut s = 0.0;
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        s += (a[0] - ox) * (b[1] - oy) - (b[0] - ox) * (a[1] - oy);
    }
    s / 2.0
}

/// Punkt-in-Polygon (Strahlverfahren).
fn ring_contains(ring: &[[f64; 2]], x: f64, y: f64) -> bool {
    let n = ring.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let [xi, yi] = ring[i];
        let [xj, yj] = ring[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_and_contains() {
        let p = Polygon::rect(500_000.0, 5_400_000.0, 500_010.0, 5_400_020.0);
        assert!((p.area() - 200.0).abs() < 1e-9);
        assert!((p.perimeter() - 60.0).abs() < 1e-9);
        assert!(p.contains(500_005.0, 5_400_010.0));
        assert!(!p.contains(500_011.0, 5_400_010.0));
    }

    #[test]
    fn hole_is_excluded() {
        let p = Polygon::with_holes(
            vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
            vec![vec![[4.0, 4.0], [6.0, 4.0], [6.0, 6.0], [4.0, 6.0]]],
        )
        .unwrap();
        assert!((p.area() - 96.0).abs() < 1e-9);
        assert!(!p.contains(5.0, 5.0));
        assert!(p.contains(1.0, 1.0));
    }

    #[test]
    fn geojson_feature_collection() {
        let gj = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},
            "geometry":{"type":"Polygon","coordinates":[[[500000,5400000],[500010,5400000],
            [500010,5400010],[500000,5400010],[500000,5400000]]]}}]}"#;
        let p = Polygon::from_geojson_str(gj).unwrap();
        assert_eq!(p.exterior.len(), 4);
        assert!((p.area() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn geojson_wgs84_is_projected_to_utm32() {
        let gj = r#"{"type":"Polygon","coordinates":[[[9.0,48.0],[9.001,48.0],[9.001,48.001],[9.0,48.001],[9.0,48.0]]]}"#;
        let p = Polygon::from_geojson_str(gj).unwrap();
        let b = p.bbox();
        assert!((b.min_x - 500_000.0).abs() < 1e-6, "{}", b.min_x);
        assert!(
            b.min_y > 5_316_000.0 && b.min_y < 5_317_000.0,
            "{}",
            b.min_y
        );
        // 0,001° ≈ 74,6 m × 111,2 m
        assert!((p.area() - 74.6 * 111.2).abs() < 30.0, "{}", p.area());
    }

    #[test]
    fn multiple_named_regions_and_lines() {
        let gj = r#"{"type":"FeatureCollection","crs":{"type":"name","properties":{"name":"urn:ogc:def:crs:EPSG::25832"}},"features":[
            {"type":"Feature","properties":{"name":"Grube Nord"},"geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,10],[0,10]]]}},
            {"type":"Feature","properties":{},"geometry":{"type":"LineString","coordinates":[[0,0],[5,5]]}},
            {"type":"Feature","properties":{"id":7},"geometry":{"type":"MultiPolygon","coordinates":[[[[20,0],[30,0],[30,10]]]]}}]}"#;
        let r = Polygon::regions_from_geojson_str(gj, Utm::default()).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].name, "Grube Nord");
        assert_eq!(r[1].name, "7");
        assert!((r[0].polygon.area() - 100.0).abs() < 1e-9);
        let l = line_from_geojson_str(gj, Utm::default()).unwrap();
        assert_eq!(l, vec![[0.0, 0.0], [5.0, 5.0]]);
    }
}
