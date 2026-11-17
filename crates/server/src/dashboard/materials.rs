//! Material im Überblick: Massenziele je Bereich (Abtrag/Auftrag) und Lagerbestand der Haufwerk-Bereiche,
//! je Material summiert und über die Befliegungen verfolgt.

use super::{params, result};
use crate::db::{status, Capture, Job, RegionRow};
use serde::Serialize;
use serde_json::Value;

/// Massenziel eines Bereichs mit Stand aus der neuesten Aushub-Berechnung gegen das Urgelände
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub region_id: i64,
    pub name: String,
    pub material: Option<String>,
    /// `cut` (Abtrag) oder `fill` (Auftrag)
    pub kind: String,
    pub target: f64,
    /// bisher bewegt; `None`, wenn keine Berechnung den Bereich enthält
    pub done: Option<f64>,
    pub percent: Option<f64>,
    pub open: Option<f64>,
}

/// Ziele aller Bereiche mit Soll-Menge; `latest` ist das Ergebnis der neuesten Aushub-Berechnung
pub fn targets(regions: &[RegionRow], latest: Option<&Value>) -> Vec<Target> {
    regions
        .iter()
        .filter(|r| r.target_kind.as_deref() != Some("stock"))
        .filter_map(|r| {
            let target = r.target_m3?;
            let kind = r.target_kind.clone().unwrap_or_else(|| "cut".into());
            let field = if kind == "fill" { "fill_m3" } else { "cut_m3" };
            let done = latest
                .and_then(|v| v["regions"].as_array())
                .and_then(|rs| rs.iter().find(|x| x["name"] == r.name.as_str()))
                .and_then(|x| x[field].as_f64());
            Some(Target {
                region_id: r.id,
                name: r.name.clone(),
                material: r.material.clone(),
                kind,
                target,
                done,
                percent: done.map(|d| d / target * 100.0),
                open: done.map(|d| (target - d).max(0.0)),
            })
        })
        .collect()
}

/// Bestand eines Lagers (Haufwerk-Bereich) je Aufnahme: (Aufnahmezeit, m³), zeitlich geordnet, je Aufnahme die
/// neueste Haufwerk-Berechnung
pub fn stock_series(region: &str, captures: &[Capture], jobs: &[Job]) -> Vec<(i64, f64)> {
    let mut done: Vec<&Job> = jobs
        .iter()
        .filter(|j| j.kind == "stockpile" && j.status == status::DONE)
        .collect();
    done.sort_by_key(|j| std::cmp::Reverse(j.id));
    let mut out: Vec<(i64, i64, f64)> = Vec::new();
    for j in done {
        let Some(cap) = params(j)["captureId"].as_i64() else {
            continue;
        };
        if out.iter().any(|o| o.1 == cap) {
            continue;
        }
        let Some(c) = captures.iter().find(|c| c.id == cap) else {
            continue;
        };
        let r = result(j);
        let Some(v) = r["regions"]
            .as_array()
            .and_then(|rs| rs.iter().find(|x| x["name"] == region))
            .and_then(|x| x["fill_m3"].as_f64())
        else {
            continue;
        };
        out.push((c.captured_at, cap, v));
    }
    out.sort_by_key(|o| (o.0, o.1));
    out.into_iter().map(|(at, _, v)| (at, v)).collect()
}

/// Bestand je Material über alle Lager: aktuell (je Lager die neueste Messung), vorher (die davor), Änderung
pub fn stock(regions: &[RegionRow], captures: &[Capture], jobs: &[Job]) -> Vec<Value> {
    let lagers: Vec<(&str, Vec<(i64, f64)>)> = regions
        .iter()
        .filter(|r| r.target_kind.as_deref() == Some("stock"))
        .map(|r| {
            (
                r.material.as_deref().unwrap_or("ohne Material"),
                stock_series(&r.name, captures, jobs),
            )
        })
        .collect();
    let mut names: Vec<&str> = lagers.iter().map(|l| l.0).collect();
    names.sort_unstable();
    names.dedup();
    names
        .into_iter()
        .map(|m| {
            let of: Vec<&Vec<(i64, f64)>> =
                lagers.iter().filter(|l| l.0 == m).map(|l| &l.1).collect();
            let measured: Vec<&Vec<(i64, f64)>> =
                of.iter().copied().filter(|s| !s.is_empty()).collect();
            let current: f64 = measured.iter().filter_map(|s| s.last()).map(|p| p.1).sum();
            let at = measured.iter().filter_map(|s| s.last()).map(|p| p.0).max();
            // Änderung nur, wenn jedes gemessene Lager eine Messung davor hat
            let before: Option<f64> = measured
                .iter()
                .map(|s| s.len().checked_sub(2).map(|i| s[i].1))
                .sum();
            serde_json::json!({
                "material": m,
                "stockpiles": of.len(),
                "measured": measured.len(),
                "current": (!measured.is_empty()).then_some(current),
                "previous": before.filter(|_| !measured.is_empty()),
                "change": before.filter(|_| !measured.is_empty()).map(|b| current - b),
                "at": at,
            })
        })
        .collect()
}

/// Verlauf je Material: zu jeder Befliegung mit einer Messung die Summe der Lager; ein Lager ohne neue Messung
/// zählt mit seinem letzten Wert, eins ohne bisherige Messung gar nicht
pub fn stock_history(regions: &[RegionRow], captures: &[Capture], jobs: &[Job]) -> Vec<Value> {
    let mut by_material: Vec<(&str, Vec<Vec<(i64, f64)>>)> = Vec::new();
    for r in regions
        .iter()
        .filter(|r| r.target_kind.as_deref() == Some("stock"))
    {
        let m = r.material.as_deref().unwrap_or("ohne Material");
        let s = stock_series(&r.name, captures, jobs);
        match by_material.iter_mut().find(|e| e.0 == m) {
            Some(e) => e.1.push(s),
            None => by_material.push((m, vec![s])),
        }
    }
    by_material.sort_by(|a, b| a.0.cmp(b.0));
    by_material
        .into_iter()
        .map(|(m, series)| {
            let mut times: Vec<i64> = series.iter().flatten().map(|p| p.0).collect();
            times.sort_unstable();
            times.dedup();
            let points: Vec<Value> = times
                .iter()
                .map(|t| {
                    let v: f64 = series
                        .iter()
                        .filter_map(|s| s.iter().rev().find(|p| p.0 <= *t).map(|p| p.1))
                        .sum();
                    serde_json::json!({"at": t, "value": v})
                })
                .collect();
            serde_json::json!({"material": m, "points": points})
        })
        .collect()
}

/// Summe je Material über die Ziele (ohne Material: nicht summiert)
pub fn materials(ts: &[Target]) -> Vec<Value> {
    let mut names: Vec<&str> = ts.iter().filter_map(|t| t.material.as_deref()).collect();
    names.sort_unstable();
    names.dedup();
    names
        .into_iter()
        .map(|m| {
            let of: Vec<&Target> = ts
                .iter()
                .filter(|t| t.material.as_deref() == Some(m))
                .collect();
            let target: f64 = of.iter().map(|t| t.target).sum();
            let done: f64 = of.iter().filter_map(|t| t.done).sum();
            serde_json::json!({"material": m, "target": target, "done": done})
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::tests::{capture, volume};
    use super::*;
    use serde_json::json;

    fn region(
        id: i64,
        name: &str,
        material: Option<&str>,
        target: Option<(f64, &str)>,
    ) -> RegionRow {
        RegionRow {
            id,
            site_id: 1,
            name: name.into(),
            geojson: "{}".into(),
            area_m2: 1.0,
            created_at: 0,
            material: material.map(Into::into),
            target_m3: target.map(|t| t.0),
            target_kind: target.map(|t| t.1.into()),
        }
    }

    #[test]
    fn targets_from_the_latest_volume() {
        let latest = json!({"regions": [
            {"name": "Grube", "cut_m3": 800.0, "fill_m3": 2.0},
            {"name": "Damm", "cut_m3": 1.0, "fill_m3": 150.0},
        ]});
        let regions = [
            region(1, "Grube", Some("Boden 3–5"), Some((1000.0, "cut"))),
            region(2, "Damm", Some("Boden 3–5"), Some((100.0, "fill"))),
            region(3, "Rand", None, None),
            region(4, "Neu", Some("Fels"), Some((50.0, "cut"))),
        ];
        let t = targets(&regions, Some(&latest));
        assert_eq!(t.len(), 3, "Rand hat kein Ziel");
        // Grube: 800 von 1000 m³ Abtrag = 80 %, 200 offen
        assert_eq!(
            (t[0].done, t[0].percent, t[0].open),
            (Some(800.0), Some(80.0), Some(200.0))
        );
        // Damm: Auftrag zählt, mehr als geplant → 150 %, nichts offen
        assert_eq!((t[1].percent, t[1].open), (Some(150.0), Some(0.0)));
        // Neu: in keiner Berechnung
        assert_eq!(t[2].done, None);
        let m = materials(&t);
        assert_eq!(
            m[0],
            json!({"material": "Boden 3–5", "target": 1100.0, "done": 950.0})
        );
        assert_eq!(
            m[1],
            json!({"material": "Fels", "target": 50.0, "done": 0.0})
        );
        assert!(targets(&regions, None).iter().all(|t| t.done.is_none()));
    }

    fn stockpile_job(id: i64, capture: i64, values: &[(&str, f64)]) -> Job {
        let regions: Vec<Value> = values
            .iter()
            .map(|(n, v)| json!({"name": n, "fill_m3": v, "cut_m3": 0.0}))
            .collect();
        Job {
            kind: "stockpile".into(),
            params: json!({"captureId": capture, "regionIds": []}).to_string(),
            result: Some(json!({"regions": regions}).to_string()),
            ..volume(id, 0, 0, 0.0, status::DONE)
        }
    }

    #[test]
    fn stock_per_material() {
        let caps = [
            capture(1, "Urgelände", 100),
            capture(2, "Woche 1", 200),
            capture(3, "Woche 2", 300),
        ];
        let stock_region = |id, name: &str, m: &str| RegionRow {
            target_kind: Some("stock".into()),
            ..region(id, name, Some(m), None)
        };
        let regions = [
            stock_region(1, "Halde Nord", "Oberboden"),
            stock_region(2, "Halde Süd", "Boden Klasse 4"),
            stock_region(3, "Halde West", "Oberboden"),
            region(4, "Grube", Some("Boden Klasse 4"), Some((1000.0, "cut"))),
        ];
        let jobs = [
            stockpile_job(10, 2, &[("Halde Nord", 380.0), ("Halde Süd", 1600.0)]),
            stockpile_job(11, 3, &[("Halde Nord", 999.0)]),
            // neuere Rechnung für Woche 2 gewinnt
            stockpile_job(12, 3, &[("Halde Nord", 420.0), ("Halde Süd", 1150.0)]),
        ];
        assert_eq!(
            stock_series("Halde Nord", &caps, &jobs),
            [(200, 380.0), (300, 420.0)]
        );
        let s = stock(&regions, &caps, &jobs);
        assert_eq!(
            s[0],
            json!({"material": "Boden Klasse 4", "stockpiles": 1, "measured": 1, "current": 1150.0,
                   "previous": 1600.0, "change": -450.0, "at": 300})
        );
        // Halde West ist noch nicht gemessen: zählt als Lager, nicht im Bestand
        assert_eq!(
            s[1],
            json!({"material": "Oberboden", "stockpiles": 2, "measured": 1, "current": 420.0,
                   "previous": 380.0, "change": 40.0, "at": 300})
        );
        // ein Lager ist kein Massenziel
        assert_eq!(targets(&regions, None).len(), 1);
        // Verlauf: je Befliegung die Summe
        let h = stock_history(&regions, &caps, &jobs);
        assert_eq!(
            h[0],
            json!({"material": "Boden Klasse 4", "points": [{"at": 200, "value": 1600.0}, {"at": 300, "value": 1150.0}]})
        );
        assert_eq!(h[1]["points"][1], json!({"at": 300, "value": 420.0}));
        // nur eine Messung: keine Änderung
        let one = stock(&regions[..1], &caps, &jobs[..1]);
        assert_eq!(
            (&one[0]["current"], &one[0]["change"]),
            (&json!(380.0), &json!(null))
        );
    }

    #[test]
    fn history_carries_the_last_value() {
        // zwei Oberboden-Lager, A in Woche 1 gemessen (100), B erst in Woche 2 (50): Woche 2 = 100 + 50
        let caps = [capture(2, "Woche 1", 200), capture(3, "Woche 2", 300)];
        let lager = |id, name: &str| RegionRow {
            target_kind: Some("stock".into()),
            ..region(id, name, Some("Oberboden"), None)
        };
        let jobs = [
            stockpile_job(1, 2, &[("A", 100.0)]),
            stockpile_job(2, 3, &[("B", 50.0)]),
        ];
        let h = stock_history(&[lager(1, "A"), lager(2, "B")], &caps, &jobs);
        assert_eq!(
            h[0]["points"],
            json!([{"at": 200, "value": 100.0}, {"at": 300, "value": 150.0}])
        );
    }
}
