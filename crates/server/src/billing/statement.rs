//! Aufmaß für einen Abrechnungszeitraum: Menge je Bereich zwischen zwei Befliegungen, in allen Einheiten, mit
//! Leistungsposition, Einheitspreis und Betrag; dazu der Abgleich mit Lieferscheinen im selben Zeitraum.
//!
//! Die Menge eines Zeitraums ist der Unterschied der Aushubmengen gegen das Urgelände (älteste Aufnahme):
//! Stand bis „bis“ minus Stand bis „von“. So addieren sich die Zeiträume zur Gesamtmenge.

use super::convert::{to_bank, Props};
use super::deliveries::Delivery;
use super::layers::Layer;
pub use super::positions::{missing, positions, round2, Position};
use crate::db::{status, Capture, Job, RegionRow};
use serde::Serialize;
use serde_json::{json, Value};

/// Einstellungen der Baustelle für die Umrechnung
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub truck_t: Option<f64>,
    pub materials: Vec<(String, Props)>,
    /// Schichten von oben nach unten; leer, wenn nicht nach Schichten abgerechnet wird
    pub layers: Vec<Layer>,
}

impl Settings {
    pub fn props(&self, material: Option<&str>) -> Props {
        material
            .and_then(|m| self.materials.iter().find(|(n, _)| n == m))
            .map(|(_, p)| *p)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub material: Option<String>,
    pub count: usize,
    /// Summe der Lieferscheine je Einheit (`t`, `m3`, `loads`)
    pub delivered: Vec<(String, f64)>,
    /// Lieferscheine umgerechnet in m³ fest; `None`, wenn ein Kennwert fehlt
    pub delivered_bank: Option<f64>,
    /// gemessen im Zeitraum, m³ fest
    pub measured_bank: Option<f64>,
    /// (geliefert − gemessen) ÷ gemessen
    pub deviation_percent: Option<f64>,
}

const DAY: i64 = 86_400_000;

/// Lieferscheine im Zeitraum: nach dem Tag von „von“ bis einschließlich dem Tag von „bis“
pub fn in_period(d: &Delivery, from_at: i64, to_at: i64) -> bool {
    let day = d.delivered_at.div_euclid(DAY);
    day > from_at.div_euclid(DAY) && day <= to_at.div_euclid(DAY)
}

/// Abgleich je Material: Lieferscheine im Zeitraum gegen die gemessene Menge
pub fn checks(ps: &[Position], ds: &[&Delivery], s: &Settings) -> Vec<Check> {
    let mut names: Vec<Option<&str>> = ds.iter().map(|d| d.material.as_deref()).collect();
    names.sort_unstable();
    names.dedup();
    names
        .into_iter()
        .map(|m| {
            let of: Vec<&&Delivery> = ds.iter().filter(|d| d.material.as_deref() == m).collect();
            let props = s.props(m);
            let delivered = ["t", "m3", "loads"]
                .iter()
                .filter_map(|u| {
                    let x: Vec<f64> = of
                        .iter()
                        .filter(|d| d.unit == *u)
                        .map(|d| d.quantity)
                        .collect();
                    (!x.is_empty()).then(|| (u.to_string(), x.iter().sum()))
                })
                .collect();
            let delivered_bank: Option<f64> = of
                .iter()
                .map(|d| to_bank(d.quantity, &d.unit, props, s.truck_t))
                .sum();
            let measured: Vec<Option<f64>> = ps
                .iter()
                .filter(|p| p.material.as_deref() == m)
                .map(|p| p.period.map(|q| q.bank_m3))
                .collect();
            let measured_bank: Option<f64> = if measured.is_empty() {
                None
            } else {
                measured.into_iter().sum()
            };
            let deviation_percent = delivered_bank
                .zip(measured_bank.filter(|v| v.abs() > 1e-9))
                .map(|(d, v)| (d - v) / v * 100.0);
            Check {
                material: m.map(Into::into),
                count: of.len(),
                delivered,
                delivered_bank,
                measured_bank,
                deviation_percent,
            }
        })
        .collect()
}

fn capture_json(c: &Capture) -> Value {
    json!({"id": c.id, "name": c.name, "at": c.captured_at})
}

/// Ganzes Aufmaß als JSON für UI und PDF; Fehler als Text, wenn der Zeitraum nicht passt
pub fn statement(
    captures: &[Capture],
    regions: &[RegionRow],
    jobs: &[Job],
    deliveries: &[Delivery],
    s: &Settings,
    from: i64,
    to: i64,
) -> Result<Value, &'static str> {
    let ready: Vec<&Capture> = captures
        .iter()
        .filter(|c| c.status == status::READY)
        .collect();
    let base = ready
        .iter()
        .min_by_key(|c| (c.captured_at, c.id))
        .ok_or("keine geprüfte Aufnahme")?;
    let find = |id| ready.iter().find(|c| c.id == id).copied();
    let (f, t) = find(from).zip(find(to)).ok_or("Aufnahme unbekannt")?;
    if (f.captured_at, f.id) >= (t.captured_at, t.id) {
        return Err("„von“ muss vor „bis“ liegen");
    }
    let (ps, outdated) = positions(regions, base.id, f.id, t.id, jobs, s);
    let ds: Vec<&Delivery> = deliveries
        .iter()
        .filter(|d| in_period(d, f.captured_at, t.captured_at))
        .collect();
    let amounts: Vec<f64> = ps.iter().filter_map(|p| p.amount).collect();
    Ok(json!({
        "base": capture_json(base),
        "from": capture_json(f),
        "to": capture_json(t),
        "missing": missing(base.id, &[f, t], jobs),
        "truckT": s.truck_t,
        "amount": (!amounts.is_empty()).then(|| round2(amounts.iter().sum())),
        // Schichten angelegt, aber die Berechnungen sind älter: Bereiche ohne Aufteilung
        "layersOutdated": outdated,
        "unpriced": ps.iter().filter(|p| p.unit_price.is_none()).count(),
        // Preis je t oder m³ lose, aber Dichte oder Auflockerung fehlt
        "unconverted": ps
            .iter()
            .filter(|p| p.unit_price.is_some() && p.period.is_some() && p.quantity.is_none())
            .count(),
        "checks": checks(&ps, &ds, s),
        "positions": ps,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::tests::{capture, volume};

    fn region(id: i64, name: &str, m: &str, pos: &str, price: Option<(f64, &str)>) -> RegionRow {
        RegionRow {
            id,
            site_id: 1,
            name: name.into(),
            geojson: "{}".into(),
            area_m2: 1.0,
            created_at: 0,
            material: Some(m.into()),
            target_m3: None,
            target_kind: None,
            lv_position: Some(pos.into()),
            unit_price: price.map(|p| p.0),
            price_unit: price.map(|p| p.1.into()),
        }
    }

    fn job(id: i64, before: i64, after: i64, regions: &[(&str, f64)]) -> Job {
        let rs: Vec<Value> = regions
            .iter()
            .map(|(n, v)| json!({"name": n, "cut_m3": v, "fill_m3": 0.0}))
            .collect();
        Job {
            result: Some(json!({"regions": rs, "total": {}}).to_string()),
            ..volume(id, before, after, 0.0, status::DONE)
        }
    }

    fn delivery(day: i64, m: &str, q: f64, unit: &str) -> Delivery {
        Delivery {
            id: day,
            site_id: 1,
            delivered_at: day * DAY,
            material: Some(m.into()),
            quantity: q,
            unit: unit.into(),
            reference: None,
            created_at: 0,
        }
    }

    fn settings() -> Settings {
        Settings {
            truck_t: Some(15.0),
            layers: Vec::new(),
            materials: vec![(
                "Lehm".into(),
                Props {
                    density: Some(2.0),
                    bulking: Some(1.25),
                },
            )],
        }
    }

    #[test]
    fn period_quantities_and_amounts() {
        // Urgelände Tag 1, Monat 1 Tag 31, Monat 2 Tag 59
        let caps = [
            capture(1, "Urgelände", DAY),
            capture(2, "Januar", 31 * DAY),
            capture(3, "Februar", 59 * DAY),
        ];
        let regions = [
            region(1, "Grube", "Lehm", "01.02.0010", Some((12.5, "t"))),
            region(2, "Rampe", "Lehm", "01.02.0020", Some((8.0, "m3"))),
            region(3, "Rand", "Fels", "01.02.0030", None),
        ];
        let jobs = [
            job(
                10,
                1,
                2,
                &[("Grube", 300.0), ("Rampe", 20.0), ("Rand", 5.0)],
            ),
            job(
                11,
                1,
                3,
                &[("Grube", 800.0), ("Rampe", 50.0), ("Rand", 5.0)],
            ),
        ];
        let v = statement(&caps, &regions, &jobs, &[], &settings(), 2, 3).unwrap();
        let p = &v["positions"];
        // Grube im Februar: 800 − 300 = 500 m³ fest = 1.000 t = 67 Fahrten; 1.000 t × 12,50 € = 12.500 €
        assert_eq!(p[0]["period"]["bankM3"], 500.0);
        assert_eq!(p[0]["period"]["looseM3"], 625.0);
        assert_eq!(p[0]["period"]["tonnes"], 1000.0);
        assert_eq!(p[0]["period"]["loads"], 67.0);
        assert_eq!(p[0]["quantity"], 1000.0);
        assert_eq!(p[0]["amount"], 12500.0);
        // Rampe: 30 m³ × 8 € = 240 €
        assert_eq!(p[1]["amount"], 240.0);
        // Rand ohne Preis, ohne Kennwerte: Menge in m³, keine Tonnen
        assert_eq!(p[2]["period"]["bankM3"], 0.0);
        assert_eq!(p[2]["period"]["tonnes"], Value::Null);
        assert_eq!(v["amount"], 12740.0);
        assert_eq!(v["unpriced"], 1);
        assert_eq!(v["unconverted"], 0);
        // ohne Kennwerte: Preis je t lässt sich nicht anwenden
        let bare = statement(&caps, &regions, &jobs, &[], &Settings::default(), 2, 3).unwrap();
        assert_eq!(bare["unconverted"], 1);
        assert_eq!(bare["amount"], 240.0);
        // ab Urgelände: der ganze Stand
        let all = statement(&caps, &regions, &jobs, &[], &settings(), 1, 3).unwrap();
        assert_eq!(all["positions"][0]["period"]["bankM3"], 800.0);
        assert_eq!(all["positions"][0]["previous"], 0.0);
        // Zeiträume addieren sich: Januar + Februar = gesamt
        let jan = statement(&caps, &regions, &jobs, &[], &settings(), 1, 2).unwrap();
        assert_eq!(
            jan["amount"].as_f64().unwrap() + v["amount"].as_f64().unwrap(),
            all["amount"].as_f64().unwrap()
        );
    }

    #[test]
    fn wrong_period_and_missing_computation() {
        let caps = [capture(1, "Urgelände", DAY), capture(2, "Januar", 31 * DAY)];
        let regions = [region(1, "Grube", "Lehm", "1", None)];
        assert!(statement(&caps, &regions, &[], &[], &settings(), 2, 1).is_err());
        assert!(statement(&caps, &regions, &[], &[], &settings(), 1, 9).is_err());
        assert!(statement(&[], &regions, &[], &[], &settings(), 1, 2).is_err());
        let v = statement(&caps, &regions, &[], &[], &settings(), 1, 2).unwrap();
        assert_eq!(v["missing"], json!(["Januar"]));
        assert_eq!(v["positions"][0]["period"], Value::Null);
        assert_eq!(v["amount"], Value::Null);
    }

    #[test]
    fn deliveries_against_the_measurement() {
        let caps = [
            capture(1, "Urgelände", DAY + 9 * 3_600_000),
            capture(2, "Januar", 31 * DAY + 9 * 3_600_000),
        ];
        let regions = [region(1, "Grube", "Lehm", "1", None)];
        let jobs = [job(10, 1, 2, &[("Grube", 500.0)])];
        let ds = [
            delivery(1, "Lehm", 999.0, "t"), // am Tag des Urgeländes: nicht im Zeitraum
            delivery(10, "Lehm", 600.0, "t"),
            delivery(20, "Lehm", 250.0, "m3"), // 250 m³ lose = 200 m³ fest
            delivery(31, "Lehm", 10.0, "loads"), // 10 × 15 t = 150 t = 75 m³ fest
            delivery(32, "Lehm", 999.0, "t"),  // nach dem Flug: nicht im Zeitraum
            delivery(15, "Fels", 5.0, "t"),
        ];
        let v = statement(&caps, &regions, &jobs, &ds, &settings(), 1, 2).unwrap();
        let c = &v["checks"];
        assert_eq!(c[0]["material"], "Fels");
        // Fels: nicht gemessen, keine Dichte → nichts zu vergleichen
        assert_eq!(c[0]["deliveredBank"], Value::Null);
        assert_eq!(c[0]["measuredBank"], Value::Null);
        // Lehm: 600 t = 300 m³ + 200 + 75 = 575 m³ fest gegen 500 gemessen = +15 %
        assert_eq!(c[1]["count"], 3);
        assert_eq!(
            c[1]["delivered"],
            json!([["t", 600.0], ["m3", 250.0], ["loads", 10.0]])
        );
        assert_eq!(c[1]["deliveredBank"], 575.0);
        assert_eq!(c[1]["measuredBank"], 500.0);
        assert!((c[1]["deviationPercent"].as_f64().unwrap() - 15.0).abs() < 1e-9);
    }
}
