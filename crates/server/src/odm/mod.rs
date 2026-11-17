//! Drohnenfotos → Punktwolke über NodeODM. Ablauf: Auftrag als Entwurf anlegen (`POST /sites/{id}/odm`),
//! Fotos einzeln hochladen (`PUT /jobs/{id}/images?file=…`), starten (`POST /jobs/{id}/start`). Der Worker
//! lädt die Fotos zu NodeODM, fragt den Fortschritt ab, holt die Punktwolke und legt sie als Aufnahme an
//! (`run.rs`); Zone und Höhenbezug richtet `cloud.rs` aus.

pub mod client;
pub mod cloud;
pub mod run;

use crate::auth::CurrentUser;
use crate::captures::{safe_file_name, write_body};
use crate::db::{self, now_ms, status, Job};
use crate::error::{code, ApiError, ApiResult};
use crate::events::Topic;
use crate::state::AppRef;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use client::NodeOdm;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

pub const KIND: &str = "odm";
/// Weniger Fotos rechnet ODM nicht sinnvoll
pub const MIN_IMAGES: usize = 3;
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "tif", "tiff", "png"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    /// schnell, gröbere Punktwolke – zum Ausprobieren
    Fast,
    #[default]
    Standard,
    /// dichter, braucht deutlich mehr Zeit und Speicher
    High,
}

/// Parameter des Auftrags (`jobs.params`)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Params {
    pub name: String,
    #[serde(default)]
    pub captured_at: Option<i64>,
    #[serde(default)]
    pub quality: Quality,
    /// Wird von jeder Höhe abgezogen, z. B. die Höhenanomalie (ellipsoidisch → DHHN2016)
    #[serde(default)]
    pub height_offset: f64,
}

/// ODM-Optionen je Qualität; `auto-boundary` schneidet Ränder ohne Überdeckung ab
pub fn options(q: Quality) -> Value {
    let (pc, features) = match q {
        Quality::Fast => ("low", "medium"),
        Quality::Standard => ("medium", "high"),
        Quality::High => ("high", "high"),
    };
    json!([
        {"name": "pc-quality", "value": pc},
        {"name": "feature-quality", "value": features},
        {"name": "auto-boundary", "value": true},
    ])
}

pub fn is_image(file: &str) -> bool {
    file.rsplit_once('.')
        .is_some_and(|(_, e)| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

pub fn images_dir(app: &AppRef, job_id: i64) -> PathBuf {
    app.cfg.job_dir(job_id).join("images")
}

/// Fotos eines Auftrags, nach Namen sortiert
pub fn list_images(dir: &std::path::Path) -> std::io::Result<Vec<PathBuf>> {
    let mut v: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.to_str().is_some_and(is_image))
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e),
    };
    v.sort();
    Ok(v)
}

fn client(app: &AppRef) -> ApiResult<NodeOdm> {
    let url = app
        .cfg
        .nodeodm_url
        .as_deref()
        .ok_or_else(|| ApiError::new(StatusCode::CONFLICT, code::ODM_NOT_CONFIGURED))?;
    Ok(NodeOdm::new(url, app.cfg.nodeodm_token.clone())?)
}

/// `GET /api/odm`: eingerichtet? erreichbar? Version und Warteschlange
pub async fn node_status(State(app): State<AppRef>) -> ApiResult<Json<Value>> {
    let Ok(odm) = client(&app) else {
        return Ok(Json(json!({ "configured": false })));
    };
    Ok(Json(match odm.info().await {
        Ok(i) => json!({
            "configured": true, "reachable": true, "version": i.version,
            "engineVersion": i.engine_version, "queue": i.task_queue_count,
        }),
        Err(e) => json!({ "configured": true, "reachable": false, "error": format!("{e:#}") }),
    }))
}

/// `POST /api/sites/{id}/odm`: Auftrag als Entwurf, Fotos folgen
pub async fn create(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(site_id): Path<i64>,
    Json(p): Json<Params>,
) -> ApiResult<(StatusCode, Json<Job>)> {
    client(&app)?;
    db::get_site(&app.db, site_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let name = p.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request(code::NAME_REQUIRED));
    }
    if !p.height_offset.is_finite() || p.height_offset.abs() > 200.0 {
        return Err(ApiError::bad_request(code::INVALID_INPUT).with_detail("Höhenversatz"));
    }
    let params = Params {
        name: name.to_string(),
        ..p
    };
    let id = sqlx::query(
        "INSERT INTO jobs(site_id, kind, params, status, created_by, created_at) VALUES(?, ?, ?, ?, ?, ?)",
    )
    .bind(site_id)
    .bind(KIND)
    .bind(serde_json::to_string(&params)?)
    .bind(status::DRAFT)
    .bind(me.id)
    .bind(now_ms())
    .execute(&app.db)
    .await?
    .last_insert_rowid();
    app.events.changed(Topic::Jobs, Some(site_id));
    let job = db::get_job(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    Ok((StatusCode::CREATED, Json(job)))
}

async fn draft(app: &AppRef, id: i64) -> ApiResult<Job> {
    let job = db::get_job(&app.db, id)
        .await?
        .filter(|j| j.kind == KIND)
        .ok_or_else(ApiError::not_found)?;
    if job.status != status::DRAFT {
        return Err(ApiError::new(StatusCode::CONFLICT, code::ODM_NOT_DRAFT));
    }
    Ok(job)
}

#[derive(Deserialize)]
pub struct ImageQuery {
    file: String,
}

/// `PUT /api/jobs/{id}/images?file=DJI_0001.JPG` mit dem Foto als Body
pub async fn upload_image(
    State(app): State<AppRef>,
    Path(id): Path<i64>,
    Query(q): Query<ImageQuery>,
    body: Body,
) -> ApiResult<Json<Value>> {
    draft(&app, id).await?;
    let file = safe_file_name(&q.file).ok_or_else(|| ApiError::bad_request(code::INVALID_INPUT))?;
    if !is_image(&file) {
        return Err(ApiError::bad_request(code::ODM_UNSUPPORTED_IMAGE));
    }
    let dir = images_dir(&app, id);
    write_body(&dir.join(&file), body, app.cfg.max_upload).await?;
    let images = list_images(&dir)?.len();
    Ok(Json(json!({ "images": images })))
}

/// `POST /api/jobs/{id}/start`: Entwurf einreihen
pub async fn start(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<Json<Job>> {
    let job = draft(&app, id).await?;
    if list_images(&images_dir(&app, id))?.len() < MIN_IMAGES {
        return Err(ApiError::bad_request(code::ODM_TOO_FEW_IMAGES));
    }
    sqlx::query("UPDATE jobs SET status = ?, progress = 0 WHERE id = ?")
        .bind(status::QUEUED)
        .bind(id)
        .execute(&app.db)
        .await?;
    app.events.changed(Topic::Jobs, Some(job.site_id));
    app.odm_wakeup.notify_one();
    Ok(Json(
        db::get_job(&app.db, id)
            .await?
            .ok_or_else(ApiError::not_found)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_presets() {
        let fast = options(Quality::Fast);
        assert_eq!(fast[0], json!({"name": "pc-quality", "value": "low"}));
        assert_eq!(options(Quality::High)[0]["value"], "high");
        assert_eq!(options(Quality::Standard)[1]["value"], "high");
        let p: Params = serde_json::from_value(json!({"name": "Flug 1"})).unwrap();
        assert_eq!(p.quality, Quality::Standard);
        assert_eq!(p.height_offset, 0.0);
    }

    #[test]
    fn image_files() {
        assert!(is_image("DJI_0001.JPG"));
        assert!(is_image("a.tiff"));
        assert!(!is_image("wolke.laz"));
        assert!(!is_image("jpg"));
    }

    #[test]
    fn images_listed_sorted_and_filtered() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_images(&dir.path().join("fehlt")).unwrap().is_empty());
        for f in ["b.jpg", "a.JPG", "notiz.txt"] {
            std::fs::write(dir.path().join(f), b"x").unwrap();
        }
        let names: Vec<String> = list_images(dir.path())
            .unwrap()
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a.JPG", "b.jpg"]);
    }
}
