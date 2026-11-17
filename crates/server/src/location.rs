//! Lage einer Baustelle für die Karte: Mitte über alle geprüften Aufnahmen, sonst über die Bereiche; als
//! Breite/Länge (WGS84 ≈ ETRS89, auf der Karte gleich). Baustellen ohne Daten haben keine Lage.

use crate::compute;
use crate::db::{status, Db};
use anyhow::Result;
use fixpunkt::crs::{utm_zone, Utm};
use fixpunkt::geom::BBox;
use serde_json::Value;
use std::collections::HashMap;

/// Ausdehnung aus der Prüfung einer Aufnahme (`info.bbox`)
pub fn bbox_of_info(info: &Value) -> Option<BBox> {
    let b = &info["bbox"];
    let v = |k: &str| b[k].as_f64().filter(|x| x.is_finite());
    Some(BBox {
        min_x: v("minX")?,
        min_y: v("minY")?,
        max_x: v("maxX")?,
        max_y: v("maxY")?,
    })
}

/// Mitte der gemeinsamen Ausdehnung
pub fn center(boxes: &[BBox]) -> Option<(f64, f64)> {
    let mut u = BBox::empty();
    for b in boxes {
        u.extend(b.min_x, b.min_y);
        u.extend(b.max_x, b.max_y);
    }
    (!u.is_empty()).then(|| ((u.min_x + u.max_x) / 2.0, (u.min_y + u.max_y) / 2.0))
}

/// UTM-Koordinate → [Breite, Länge] in Grad; `None` außerhalb von UTM oder bei unsinnigen Werten
pub fn lat_lon(epsg: i64, x: f64, y: f64) -> Option<[f64; 2]> {
    let zone = utm_zone(u32::try_from(epsg).ok()?)?;
    let (lat, lon) = Utm::new(zone).ok()?.inverse(x, y);
    ((-85.0..=85.0).contains(&lat) && (-180.0..=180.0).contains(&lon)).then_some([lat, lon])
}

/// Lage je Baustelle (Schlüssel: Baustellen-ID)
pub async fn site_centers(db: &Db) -> Result<HashMap<i64, [f64; 2]>> {
    let mut boxes: HashMap<i64, Vec<BBox>> = HashMap::new();
    let captures: Vec<(i64, String)> =
        sqlx::query_as("SELECT site_id, info FROM captures WHERE status = ? AND info IS NOT NULL")
            .bind(status::READY)
            .fetch_all(db)
            .await?;
    for (site, info) in captures {
        if let Some(b) = serde_json::from_str(&info)
            .ok()
            .as_ref()
            .and_then(bbox_of_info)
        {
            boxes.entry(site).or_default().push(b);
        }
    }
    let regions: Vec<(i64, String, String)> =
        sqlx::query_as("SELECT site_id, name, geojson FROM regions")
            .fetch_all(db)
            .await?;
    // Bereiche nur für Baustellen ohne geprüfte Aufnahme
    let with_captures: std::collections::HashSet<i64> = boxes.keys().copied().collect();
    for (site, name, gj) in regions {
        if with_captures.contains(&site) {
            continue;
        }
        if let Ok(r) = compute::parse_region(&name, &gj) {
            boxes.entry(site).or_default().push(r.polygon.bbox());
        }
    }
    let epsg: HashMap<i64, i64> = sqlx::query_as("SELECT id, epsg FROM sites")
        .fetch_all(db)
        .await?
        .into_iter()
        .collect();
    Ok(boxes
        .into_iter()
        .filter_map(|(site, b)| {
            let (x, y) = center(&b)?;
            Some((site, lat_lon(*epsg.get(&site)?, x, y)?))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn center_of_all_boxes() {
        let a = BBox {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 10.0,
            max_y: 10.0,
        };
        let b = BBox {
            min_x: 20.0,
            min_y: -10.0,
            max_x: 30.0,
            max_y: 0.0,
        };
        assert_eq!(center(&[a, b]), Some((15.0, 0.0)));
        assert_eq!(center(&[]), None);
    }

    #[test]
    fn utm32_to_lat_lon() {
        // Mittelmeridian 9° O, Ostwert 500 000 m liegt genau darauf; unabhängig über die Vorwärtsrechnung
        let (e, n) = Utm::new(32).unwrap().forward(48.137, 11.575); // München
        let [lat, lon] = lat_lon(25832, e, n).unwrap();
        assert!((lat - 48.137).abs() < 1e-9 && (lon - 11.575).abs() < 1e-9);
        let [_, lon] = lat_lon(25832, 500_000.0, 5_400_000.0).unwrap();
        assert!((lon - 9.0).abs() < 1e-9);
        assert_eq!(lat_lon(4326, 1.0, 2.0), None);
    }

    #[test]
    fn bbox_from_capture_info() {
        let info = json!({"bbox": {"minX": 1.0, "minY": 2.0, "maxX": 3.0, "maxY": 4.0}});
        let b = bbox_of_info(&info).unwrap();
        assert_eq!((b.min_x, b.max_y), (1.0, 4.0));
        assert!(bbox_of_info(&json!({"points": 3})).is_none());
    }
}
