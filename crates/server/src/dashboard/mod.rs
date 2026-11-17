//! Überblick einer Baustelle auf einen Blick: Aufnahmen, Aushub-Fortschritt gegen das Urgelände, letzter
//! Soll/Ist-Vergleich, Aufträge, verletzte Zwangspunkte, Dokumentation und Pläne. `GET /api/sites/{id}/dashboard`.

use crate::db::{self, status, Capture, Job, RegionRow};
use crate::error::{ApiError, ApiResult};
use crate::state::AppRef;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use serde_json::Value;

pub mod materials;
pub use materials::{materials, stock, stock_history, stock_series, targets, Target};

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

pub(crate) fn params(j: &Job) -> Value {
    serde_json::from_str(&j.params).unwrap_or(Value::Null)
}

pub(crate) fn result(j: &Job) -> Value {
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
    Ok(Json(data(&app, site).await?))
}

/// Alles für den Überblick als JSON (auch Grundlage des eigenen Berichts)
pub async fn data(app: &AppRef, site: i64) -> ApiResult<Value> {
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
            let g = crate::view::capture_view(app, c).await?;
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
    Ok(serde_json::json!({
        "captures": {
            "count": captures.len(),
            "ready": ready.len(),
            "last": last.map(|c| serde_json::json!({"id": c.id, "name": c.name, "at": c.captured_at})),
        },
        "progress": progress,
        "materials": materials(&targets),
        "stock": stock(&regions, &ready, &jobs),
        "stockHistory": stock_history(&regions, &ready, &jobs),
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
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn capture(id: i64, name: &str, at: i64) -> Capture {
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

    pub(crate) fn volume(id: i64, before: i64, after: i64, cut: f64, st: &str) -> Job {
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
