//! Pläne (DXF) je Baustelle: Hintergrund der Ansicht und Höhen für Plan/Ist. Beim Hochladen wird die DXF einmal
//! gelesen (`dxf.rs`) und als JSON abgelegt, das die UI zeichnet; das Original bleibt daneben liegen.

pub mod dxf;

use crate::captures::safe_file_name;
use crate::db::{self, now_ms};
use crate::error::{code, ApiError, ApiResult};
use crate::events::Topic;
use crate::state::AppRef;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Größte DXF in Bytes
pub const MAX_PLAN: usize = 200_000_000;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub id: i64,
    pub site_id: i64,
    pub name: String,
    pub file_name: String,
    #[serde(serialize_with = "db::raw_json_required")]
    pub info: String,
    pub created_at: i64,
}

pub(crate) fn dir(app: &AppRef, id: i64) -> PathBuf {
    app.cfg.data_dir.join("plans").join(id.to_string())
}

/// Kurzinfo für die Liste
pub fn info(g: &dxf::Geometry) -> Value {
    json!({
        "layers": g.layers,
        "paths": g.paths.len(),
        "spots": g.spots.len(),
        "labels": g.labels.len(),
        "zoneOffset": g.zone_offset,
        "bbox": g.bbox,
    })
}

pub async fn list(State(app): State<AppRef>, Path(site): Path<i64>) -> ApiResult<Json<Vec<Plan>>> {
    Ok(Json(
        sqlx::query_as("SELECT * FROM plans WHERE site_id = ? ORDER BY created_at DESC, id DESC")
            .bind(site)
            .fetch_all(&app.db)
            .await?,
    ))
}

#[derive(Deserialize)]
pub struct UploadQuery {
    file: String,
    name: Option<String>,
}

/// `PUT /api/sites/{id}/plans?file=lageplan.dxf` mit der DXF als Body
pub async fn upload(
    State(app): State<AppRef>,
    Path(site): Path<i64>,
    Query(q): Query<UploadQuery>,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<Plan>)> {
    db::get_site(&app.db, site)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let file = safe_file_name(&q.file).ok_or_else(|| ApiError::bad_request(code::INVALID_INPUT))?;
    if !file.to_ascii_lowercase().ends_with(".dxf") {
        return Err(ApiError::bad_request(code::PLAN_INVALID).with_detail("nur DXF"));
    }
    let bytes = body.to_vec();
    let (geometry, bytes) = tokio::task::spawn_blocking(move || (dxf::read(&bytes), bytes))
        .await
        .map_err(anyhow::Error::from)?;
    let g = geometry
        .map_err(|e| ApiError::bad_request(code::PLAN_INVALID).with_detail(format!("{e:#}")))?;
    let name = q.name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| {
        file.trim_end_matches(".dxf")
            .trim_end_matches(".DXF")
            .to_string()
    });
    let id = sqlx::query(
        "INSERT INTO plans(site_id, name, file_name, info, created_at) VALUES(?, ?, ?, ?, ?)",
    )
    .bind(site)
    .bind(name.trim())
    .bind(&file)
    .bind(info(&g).to_string())
    .bind(now_ms())
    .execute(&app.db)
    .await?
    .last_insert_rowid();
    let d = dir(&app, id);
    tokio::fs::create_dir_all(&d).await?;
    tokio::fs::write(d.join(&file), &bytes).await?;
    tokio::fs::write(d.join("geometry.json"), serde_json::to_vec(&g)?).await?;
    app.events.changed(Topic::Plans, Some(site));
    let plan = sqlx::query_as("SELECT * FROM plans WHERE id = ?")
        .bind(id)
        .fetch_one(&app.db)
        .await?;
    Ok((StatusCode::CREATED, Json(plan)))
}

async fn get(app: &AppRef, id: i64) -> ApiResult<Plan> {
    sqlx::query_as("SELECT * FROM plans WHERE id = ?")
        .bind(id)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(ApiError::not_found)
}

/// Aufbereitete Geometrie (Linienzüge, Texte, Höhen) für die Ansicht
pub async fn geometry(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<Response> {
    get(&app, id).await?;
    let bytes = tokio::fs::read(dir(&app, id).join("geometry.json"))
        .await
        .map_err(|_| ApiError::not_found())?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "private, max-age=86400"),
        ],
        bytes,
    )
        .into_response())
}

pub async fn delete(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<StatusCode> {
    let p = get(&app, id).await?;
    sqlx::query("DELETE FROM plans WHERE id = ?")
        .bind(id)
        .execute(&app.db)
        .await?;
    let _ = tokio::fs::remove_dir_all(dir(&app, id)).await;
    app.events.changed(Topic::Plans, Some(p.site_id));
    Ok(StatusCode::NO_CONTENT)
}

/// Ordner der Pläne einer Baustelle (zum Aufräumen beim Löschen der Baustelle)
pub async fn dirs_of_site(app: &AppRef, site: i64) -> ApiResult<Vec<PathBuf>> {
    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM plans WHERE site_id = ?")
        .bind(site)
        .fetch_all(&app.db)
        .await?;
    Ok(ids.into_iter().map(|id| dir(app, id)).collect())
}
