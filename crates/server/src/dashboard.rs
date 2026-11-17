//! Überblick einer Baustelle auf einen Blick: Aufnahmen, Aushub-Fortschritt gegen das Urgelände, letzter
//! Soll/Ist-Vergleich, Aufträge, verletzte Zwangspunkte, Dokumentation und Pläne. `GET /api/sites/{id}/dashboard`.

use crate::db::{self, status, Capture, Job, RegionRow};
use crate::error::{ApiError, ApiResult};
use crate::state::AppRef;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use serde_json::Value;

/// Zwangspunkt gilt bis zu dieser Abweichung als eingehalten (wie `CONSTRAINT_TOLERANCE` in der UI)
pub const CONSTRAINT_TOLERANCE: f64 = 0.03;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub job_id: i64,
    pub capture_id: i64,
    pub name: String,
    pub at: i64,
    pub cut: f64,
    pub fill: f64,
    pub net: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Violation {
    pub id: i64,
    pub title: String,
    /// Ist − Soll in m
    pub diff: f64,
}

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

fn params(j: &Job) -> Value {
    serde_json::from_str(&j.params).unwrap_or(Value::Null)
}

fn result(j: &Job) -> Value {
    j.result
        .as_deref()
        .and_then(|r| serde_json::from_str(r).ok())
        .unwrap_or(Value::Null)
}

/// Aushub-Fortschritt: fertige Aushub-Berechnungen gegen die älteste Aufnahme (Urgelände), je späterer Aufnahme
/// die neueste Berechnung, zeitlich geordnet
pub fn progress(captures: &[Capture], jobs: &[Job]) -> Vec<Progress> {
    let Some(base) = captures.iter().min_by_key(|c| (c.captured_at, c.id)) else {
        return Vec::new();
    };
    let mut out: Vec<Progress> = Vec::new();
    let mut done: Vec<&Job> = jobs
        .iter()
        .filter(|j| j.kind == "volume" && j.status == status::DONE)
        .collect();
    done.sort_by_key(|j| std::cmp::Reverse(j.id));
    for j in done {
        let p = params(j);
        let (Some(before), Some(after)) = (p["before"].as_i64(), p["after"].as_i64()) else {
            continue;
        };
        if before != base.id || out.iter().any(|o| o.capture_id == after) {
            continue;
        }
        let Some(c) = captures.iter().find(|c| c.id == after) else {
            continue;
        };
        let t = &result(j)["total"];
        let num = |k: &str| t[k].as_f64().unwrap_or(0.0);
        out.push(Progress {
            job_id: j.id,
            capture_id: after,
            name: c.name.clone(),
            at: c.captured_at,
            cut: num("cut_m3"),
            fill: num("fill_m3"),
            net: num("net_m3"),
        });
    }
    out.sort_by_key(|p| (p.at, p.capture_id));
    out
}

/// Letzter fertiger Soll/Ist-Vergleich: Anteil in Toleranz
pub fn latest_compare(jobs: &[Job]) -> Option<Value> {
    let j = jobs
        .iter()
        .filter(|j| j.kind == "compare" && j.status == status::DONE)
        .max_by_key(|j| j.id)?;
    let d = &result(j)["deviation"];
    Some(serde_json::json!({
        "jobId": j.id,
        "at": j.finished_at.unwrap_or(j.created_at),
        "within": d["within_tolerance_percent"].as_f64(),
        "tolerance": d["tolerance"].as_f64(),
    }))
}

/// Zwangspunkte gegen eine Höhenfunktion prüfen; ohne Höhe an der Stelle nicht geprüft
pub fn violations(
    points: &[(i64, String, f64, f64, f64)],
    height_at: impl Fn(f64, f64) -> Option<f64>,
) -> (usize, Vec<Violation>) {
    let mut checked = 0;
    let mut bad = Vec::new();
    for (id, title, x, y, z) in points {
        let Some(h) = height_at(*x, *y) else { continue };
        checked += 1;
        let diff = h - z;
        if diff.abs() > CONSTRAINT_TOLERANCE {
            bad.push(Violation {
                id: *id,
                title: title.clone(),
                diff,
            });
        }
    }
    (checked, bad)
}

pub async fn dashboard(State(app): State<AppRef>, Path(site): Path<i64>) -> ApiResult<Json<Value>> {
    db::get_site(&app.db, site)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let captures: Vec<Capture> =
        sqlx::query_as("SELECT * FROM captures WHERE site_id = ? ORDER BY captured_at, id")
            .bind(site)
            .fetch_all(&app.db)
            .await?;
    let jobs: Vec<Job> =
        sqlx::query_as("SELECT * FROM jobs WHERE site_id = ? AND kind != 'inspect'")
            .bind(site)
            .fetch_all(&app.db)
            .await?;
    let ready: Vec<Capture> = captures
        .iter()
        .filter(|c| c.status == status::READY)
        .cloned()
        .collect();
    let last = ready.last();

    let constraints: Vec<(i64, String, f64, f64, f64)> = sqlx::query_as(
        "SELECT id, title, x, y, z FROM markers WHERE site_id = ? AND kind = 'constraint'
           AND x IS NOT NULL AND y IS NOT NULL AND z IS NOT NULL",
    )
    .bind(site)
    .fetch_all(&app.db)
    .await?;
    let (checked, violated) = match (last, constraints.is_empty()) {
        (Some(c), false) => {
            let g = crate::view::capture_view(&app, c).await?;
            violations(&constraints, |x, y| g.sample(x, y))
        }
        _ => (0, Vec::new()),
    };
    let count = |kind: &str| jobs.iter().filter(|j| j.status == kind).count();
    let (photos, notes): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(kind = 'photo'), 0), COALESCE(SUM(kind = 'note'), 0) FROM markers WHERE site_id = ?",
    )
    .bind(site)
    .fetch_one(&app.db)
    .await?;
    let regions: Vec<RegionRow> =
        sqlx::query_as("SELECT * FROM regions WHERE site_id = ? ORDER BY name COLLATE NOCASE")
            .bind(site)
            .fetch_all(&app.db)
            .await?;
    let progress = progress(&ready, &jobs);
    let latest = progress
        .last()
        .and_then(|p| jobs.iter().find(|j| j.id == p.job_id))
        .map(result);
    let targets = targets(&regions, latest.as_ref());
    let plans: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plans WHERE site_id = ?")
        .bind(site)
        .fetch_one(&app.db)
        .await?;
    Ok(Json(serde_json::json!({
        "captures": {
            "count": captures.len(),
            "ready": ready.len(),
            "last": last.map(|c| serde_json::json!({"id": c.id, "name": c.name, "at": c.captured_at})),
        },
        "progress": progress,
        "materials": materials(&targets),
        "stock": stock(&regions, &ready, &jobs),
        "targets": targets,
        "latestCompare": latest_compare(&jobs),
        "jobs": {
            "queued": count(status::QUEUED) + count(status::DRAFT),
            "running": count(status::RUNNING),
            "failed": count(status::FAILED),
        },
        "constraints": {
            "total": constraints.len(),
            "checked": checked,
            "violated": violated,
            "against": last.map(|c| c.name.clone()),
        },
        "documentation": {"photos": photos, "notes": notes},
        "plans": plans,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn capture(id: i64, name: &str, at: i64) -> Capture {
        Capture {
            id,
            site_id: 1,
            name: name.into(),
            file_name: format!("{id}.laz"),
            file_size: 1,
            captured_at: at,
            status: status::READY.into(),
            info: None,
            error: None,
            created_at: 0,
            ortho: None,
        }
    }

    fn volume(id: i64, before: i64, after: i64, cut: f64, st: &str) -> Job {
        Job {
            id,
            site_id: 1,
            kind: "volume".into(),
            params: json!({"before": before, "after": after}).to_string(),
            status: st.into(),
            result: Some(
                json!({"total": {"cut_m3": cut, "fill_m3": 1.0, "net_m3": 1.0 - cut}}).to_string(),
            ),
            error: None,
            created_by: None,
            created_at: 0,
            started_at: None,
            finished_at: None,
            progress: None,
        }
    }

    #[test]
    fn progress_against_the_original_terrain() {
        let caps = [
            capture(1, "Urgelände", 100),
            capture(2, "Woche 1", 200),
            capture(3, "Woche 2", 300),
        ];
        let jobs = [
            volume(10, 1, 3, 800.0, status::DONE),
            volume(11, 1, 2, 300.0, status::DONE),
            volume(12, 1, 2, 400.0, status::DONE), // neuere Rechnung für Woche 1 gewinnt
            volume(13, 2, 3, 500.0, status::DONE), // nicht gegen das Urgelände
            volume(14, 1, 3, 9.0, status::FAILED),
        ];
        let p = progress(&caps, &jobs);
        assert_eq!(
            p.iter()
                .map(|x| (x.name.as_str(), x.cut))
                .collect::<Vec<_>>(),
            [("Woche 1", 400.0), ("Woche 2", 800.0)]
        );
        assert_eq!(p[1].net, -799.0);
        assert!(progress(&[], &jobs).is_empty());
    }

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
        // nur eine Messung: keine Änderung
        let one = stock(&regions[..1], &caps, &jobs[..1]);
        assert_eq!(
            (&one[0]["current"], &one[0]["change"]),
            (&json!(380.0), &json!(null))
        );
    }

    #[test]
    fn constraint_violations() {
        let pts = [
            (1, "Kanal".to_string(), 0.0, 0.0, 298.0),
            (2, "Schacht".to_string(), 5.0, 0.0, 298.0),
            (3, "draußen".to_string(), 99.0, 0.0, 298.0),
        ];
        // Gelände: 298,02 bei x = 0, 298,10 bei x = 5, außerhalb nichts
        let (checked, bad) = violations(&pts, |x, _| {
            (x < 50.0).then_some(298.0 + 0.02 * if x > 1.0 { 5.0 } else { 1.0 })
        });
        assert_eq!(checked, 2);
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].title, "Schacht");
        assert!((bad[0].diff - 0.1).abs() < 1e-9);
    }
}
