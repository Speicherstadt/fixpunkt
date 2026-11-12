//! Geometrie: Bounding-Box und Polygon (Abrechnungsgrenze / Geofence).
//!
//! Alle Koordinaten sind projiziert und metrisch (z. B. ETRS89/UTM32, EPSG:25832).

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
        let holes = holes.into_iter().map(normalize_ring).collect::<Result<_>>()?;
        Ok(Polygon { exterior, holes })
    }

    /// Rechteck – praktisch für Tests und schnelle Auswertungen.
    pub fn rect(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Polygon {
            exterior: vec![[min_x, min_y], [max_x, min_y], [max_x, max_y], [min_x, max_y]],
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
    /// Achtung: Koordinaten werden **nicht** umprojiziert. Die Datei muss im selben
    /// metrischen System wie die Punktwolke vorliegen (z. B. in QGIS als EPSG:25832 exportieren).
    pub fn from_geojson_file(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("Grenze {} nicht lesbar", path.display()))?;
        Self::from_geojson_str(&text)
    }

    pub fn from_geojson_str(text: &str) -> Result<Self> {
        let v: Value = serde_json::from_str(text).context("ungültiges GeoJSON")?;
        let geom = find_polygon_geometry(&v).context("kein Polygon im GeoJSON gefunden")?;
        let rings = match geom.get("type").and_then(Value::as_str) {
            Some("Polygon") => geom.get("coordinates"),
            Some("MultiPolygon") => geom.get("coordinates").and_then(|c| c.get(0)),
            _ => None,
        }
        .and_then(Value::as_array)
        .context("Polygon ohne Koordinaten")?;

        let mut parsed = rings.iter().map(parse_ring).collect::<Result<Vec<_>>>()?;
        if parsed.is_empty() {
            bail!("Polygon ohne Ringe");
        }
        let exterior = parsed.remove(0);
        let poly = Polygon::with_holes(exterior, parsed)?;
        let b = poly.bbox();
        if b.max_x.abs() <= 180.0 && b.max_y.abs() <= 90.0 {
            bail!(
                "Grenze sieht nach Längen-/Breitengrad aus. Bitte im metrischen System der \
                 Punktwolke speichern (z. B. EPSG:25832)."
            );
        }
        Ok(poly)
    }
}

fn find_polygon_geometry(v: &Value) -> Option<&Value> {
    match v.get("type")?.as_str()? {
        "Polygon" | "MultiPolygon" => Some(v),
        "Feature" => find_polygon_geometry(v.get("geometry")?),
        "FeatureCollection" => v.get("features")?.as_array()?.iter().find_map(find_polygon_geometry),
        "GeometryCollection" => v.get("geometries")?.as_array()?.iter().find_map(find_polygon_geometry),
        _ => None,
    }
}

fn parse_ring(v: &Value) -> Result<Vec<[f64; 2]>> {
    v.as_array()
        .context("Ring ist kein Array")?
        .iter()
        .map(|p| {
            let x = p.get(0).and_then(Value::as_f64).context("Koordinate ohne x")?;
            let y = p.get(1).and_then(Value::as_f64).context("Koordinate ohne y")?;
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
    fn geojson_rejects_wgs84() {
        let gj = r#"{"type":"Polygon","coordinates":[[[9.1,48.7],[9.2,48.7],[9.2,48.8],[9.1,48.7]]]}"#;
        assert!(Polygon::from_geojson_str(gj).is_err());
    }
}
