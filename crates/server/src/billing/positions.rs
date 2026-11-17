//! Positionen eines Aufmaßes: je Bereich – bei Schichten je Bereich und Schicht – die Menge im Zeitraum mit
//! Leistungsposition, Preis und Betrag. Stand je Aufnahme aus der neuesten Aushub-Berechnung gegen das Urgelände.

use super::convert::{in_unit, quantities, Quantities};
use super::layers::Layer;
use super::statement::Settings;
use crate::dashboard::{params, result};
use crate::db::{status, Capture, Job, RegionRow};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub region_id: i64,
    pub name: String,
    /// Schicht (Homogenbereich), wenn nach Schichten abgerechnet wird
    pub layer: Option<String>,
    pub lv_position: Option<String>,
    pub material: Option<String>,
    /// `cut` oder `fill`
    pub kind: String,
    /// Stand bis „von“ und bis „bis“ in m³ fest; `None`, wenn die Berechnung fehlt
    pub previous: Option<f64>,
    pub total: Option<f64>,
    /// Menge im Zeitraum in allen Einheiten
    pub period: Option<Quantities>,
    pub unit: Option<String>,
    pub unit_price: Option<f64>,
    /// Menge in der Preiseinheit und Betrag in € (netto)
    pub quantity: Option<f64>,
    pub amount: Option<f64>,
}

/// neueste fertige Aushub-Berechnung `base → cap`
fn volume_job(base: i64, cap: i64, jobs: &[Job]) -> Option<&Job> {
    jobs.iter()
        .filter(|j| j.kind == "volume" && j.status == status::DONE)
        .filter(|j| {
            let p = params(j);
            p["before"].as_i64() == Some(base) && p["after"].as_i64() == Some(cap)
        })
        .max_by_key(|j| j.id)
}

/// Stand eines Bereichs (oder einer Schicht darin) bis zur Aufnahme `cap` in m³ fest; das Urgelände hat 0
fn cumulative(
    region: &RegionRow,
    layer: Option<&str>,
    base: i64,
    cap: i64,
    jobs: &[Job],
) -> Option<f64> {
    if cap == base {
        return Some(0.0);
    }
    let r = result(volume_job(base, cap, jobs)?);
    let entry = r["regions"]
        .as_array()?
        .iter()
        .find(|x| x["name"] == region.name.as_str())?;
    match layer {
        Some(l) => entry["layers"]
            .as_array()?
            .iter()
            .find(|x| x["name"] == l)?["cut_m3"]
            .as_f64(),
        None if region.target_kind.as_deref() == Some("fill") => entry["fill_m3"].as_f64(),
        None => entry["cut_m3"].as_f64(),
    }
}

/// Aufnahmen ohne fertige Aushub-Berechnung gegen das Urgelände (außer dem Urgelände selbst)
pub fn missing(base: i64, caps: &[&Capture], jobs: &[Job]) -> Vec<String> {
    caps.iter()
        .filter(|c| c.id != base && volume_job(base, c.id, jobs).is_none())
        .map(|c| c.name.clone())
        .collect()
}

/// auf Cent runden, wie auf der Rechnung
pub fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Preis einer Position: der Schicht, sonst des Bereichs
fn price(r: &RegionRow, l: Option<&Layer>) -> (Option<String>, Option<f64>, Option<String>) {
    match l {
        Some(l) if l.unit_price.is_some() => (
            l.lv_position.clone().or_else(|| r.lv_position.clone()),
            l.unit_price,
            l.price_unit.clone(),
        ),
        Some(l) => (
            l.lv_position.clone().or_else(|| r.lv_position.clone()),
            r.unit_price,
            r.price_unit.clone(),
        ),
        None => (r.lv_position.clone(), r.unit_price, r.price_unit.clone()),
    }
}

struct Ctx<'a> {
    base: i64,
    from: i64,
    to: i64,
    jobs: &'a [Job],
    s: &'a Settings,
}

fn position(c: &Ctx, r: &RegionRow, l: Option<&Layer>) -> Position {
    let layer = l.map(|l| l.name.as_str());
    let previous = cumulative(r, layer, c.base, c.from, c.jobs);
    let total = cumulative(r, layer, c.base, c.to, c.jobs);
    // eine Schicht ist zugleich das Material
    let material = layer.map(String::from).or_else(|| r.material.clone());
    let period = previous
        .zip(total)
        .map(|(a, b)| quantities(b - a, c.s.props(material.as_deref()), c.s.truck_t));
    let (lv_position, unit_price, unit) = price(r, l);
    let quantity = period
        .as_ref()
        .zip(unit.as_deref())
        .and_then(|(q, u)| in_unit(q, u));
    Position {
        region_id: r.id,
        name: r.name.clone(),
        layer: layer.map(String::from),
        lv_position,
        material,
        kind: r.target_kind.clone().unwrap_or_else(|| "cut".into()),
        previous,
        total,
        period,
        unit,
        unit_price,
        quantity,
        amount: quantity.zip(unit_price).map(|(q, p)| round2(q * p)),
    }
}

/// Positionen und ob Schichten angelegt sind, die Berechnungen aber noch keine Aufteilung haben
pub fn positions(
    regions: &[RegionRow],
    base: i64,
    from: i64,
    to: i64,
    jobs: &[Job],
    s: &Settings,
) -> (Vec<Position>, bool) {
    let c = Ctx {
        base,
        from,
        to,
        jobs,
        s,
    };
    let mut outdated = false;
    let mut out: Vec<Position> = Vec::new();
    for r in regions
        .iter()
        .filter(|r| r.target_kind.as_deref() != Some("stock"))
    {
        let cut = r.target_kind.as_deref() != Some("fill");
        let split = cut
            && !s.layers.is_empty()
            && s.layers.iter().all(|l| {
                [from, to]
                    .iter()
                    .all(|cap| cumulative(r, Some(&l.name), base, *cap, jobs).is_some())
            });
        if split {
            out.extend(s.layers.iter().map(|l| position(&c, r, Some(l))));
        } else {
            let p = position(&c, r, None);
            outdated |= cut && !s.layers.is_empty() && p.period.is_some();
            out.push(p);
        }
    }
    // nach Leistungsposition, ohne Position am Ende; Schichten eines Bereichs bleiben von oben nach unten
    out.sort_by(|a, b| {
        (
            a.lv_position.is_none(),
            &a.lv_position,
            a.name.to_lowercase(),
        )
            .cmp(&(
                b.lv_position.is_none(),
                &b.lv_position,
                b.name.to_lowercase(),
            ))
    });
    (out, outdated)
}

#[cfg(test)]
mod tests {
    use super::super::layers::tests::layer;
    use super::*;
    use crate::dashboard::tests::volume;
    use serde_json::json;

    fn grube(price: Option<(f64, &str)>) -> RegionRow {
        RegionRow {
            id: 1,
            site_id: 1,
            name: "Grube".into(),
            geojson: "{}".into(),
            area_m2: 1.0,
            created_at: 0,
            material: Some("Boden".into()),
            target_m3: None,
            target_kind: None,
            lv_position: Some("02.01.0010".into()),
            unit_price: price.map(|p| p.0),
            price_unit: price.map(|p| p.1.into()),
        }
    }

    fn job(id: i64, after: i64, cut: f64, layers: Option<[f64; 3]>) -> Job {
        let mut r = json!({"name": "Grube", "cut_m3": cut, "fill_m3": 0.0});
        if let Some(l) = layers {
            r["layers"] = json!([
                {"name": "Oberboden", "cut_m3": l[0]},
                {"name": "Lehm", "cut_m3": l[1]},
                {"name": "Fels", "cut_m3": l[2]},
            ]);
        }
        Job {
            result: Some(json!({"regions": [r], "total": {}}).to_string()),
            ..volume(id, 1, after, 0.0, status::DONE)
        }
    }

    fn settings() -> Settings {
        let mut fels = layer("Fels", None);
        fels.lv_position = Some("02.01.0030".into());
        fels.unit_price = Some(40.0);
        fels.price_unit = Some("m3".into());
        Settings {
            truck_t: None,
            materials: Vec::new(),
            layers: vec![
                layer("Oberboden", Some(0.3)),
                layer("Lehm", Some(1.5)),
                fels,
            ],
        }
    }

    #[test]
    fn positions_per_layer() {
        // Januar: Oberboden ganz, Lehm zur Hälfte; Februar: fertig bis 2 m
        let jobs = [
            job(10, 2, 360.0, Some([120.0, 240.0, 0.0])),
            job(11, 3, 800.0, Some([120.0, 480.0, 200.0])),
        ];
        let (ps, outdated) = positions(&[grube(Some((10.0, "m3")))], 1, 2, 3, &jobs, &settings());
        assert!(!outdated);
        let got: Vec<(Option<&str>, f64, Option<f64>)> = ps
            .iter()
            .map(|p| (p.layer.as_deref(), p.period.unwrap().bank_m3, p.amount))
            .collect();
        // Februar: Oberboden 0, Lehm 240 m³ × 10 € (Preis des Bereichs), Fels 200 m³ × 40 € (eigener Preis)
        assert_eq!(
            got,
            [
                (Some("Oberboden"), 0.0, Some(0.0)),
                (Some("Lehm"), 240.0, Some(2400.0)),
                (Some("Fels"), 200.0, Some(8000.0)),
            ]
        );
        assert_eq!(ps[2].lv_position.as_deref(), Some("02.01.0030"));
        assert_eq!(ps[1].material.as_deref(), Some("Lehm"));
        // Schichten zusammen = Bereich
        let sum: f64 = ps.iter().map(|p| p.period.unwrap().bank_m3).sum();
        assert_eq!(sum, 800.0 - 360.0);
    }

    #[test]
    fn older_computations_without_layers() {
        let jobs = [job(10, 2, 360.0, None)];
        let (ps, outdated) = positions(&[grube(None)], 1, 1, 2, &jobs, &settings());
        assert!(outdated);
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].layer, None);
        assert_eq!(ps[0].period.unwrap().bank_m3, 360.0);
        // ohne Schichten ist nichts veraltet
        let plain = Settings::default();
        assert!(!positions(&[grube(None)], 1, 1, 2, &jobs, &plain).1);
    }
}
