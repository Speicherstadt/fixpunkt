//! Aufnahmen hochladen (gestreamt, ohne die Datei im Speicher zu halten), auflisten, löschen.

use crate::auth::CurrentUser;
use crate::compute;
use crate::db::{self, now_ms, status, Capture};
use crate::error::{code, ApiError, ApiResult};
use crate::events::Topic;
use crate::jobs;
use crate::state::AppRef;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::json;
use tokio::io::AsyncWriteExt;

/// Nur der Dateiname, ohne Pfad und ohne Zeichen, die im Dateisystem stören
pub fn safe_file_name(name: &str) -> Option<String> {
    let base = name.rsplit(['/', '\\']).next()?.trim();
    let clean: String = base
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let clean = clean.trim_start_matches('.').to_string();
    (!clean.is_empty() && clean.len() <= 200).then_some(clean)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadQuery {
    file: String,
    name: Option<String>,
    captured_at: Option<i64>,
}

/// `PUT /api/sites/{id}/captures?file=nachher.laz&name=Nach%20Aushub` mit der Datei als Body.
pub async fn upload(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(site_id): Path<i64>,
    Query(q): Query<UploadQuery>,
    body: Body,
) -> ApiResult<(StatusCode, Json<Capture>)> {
    db::get_site(&app.db, site_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let file_name =
        safe_file_name(&q.file).ok_or_else(|| ApiError::bad_request(code::INVALID_INPUT))?;
    if !compute::is_supported(&file_name) {
        return Err(ApiError::bad_request(code::UNSUPPORTED_FILE));
    }
    let name = q
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| file_name.clone());
    let id = insert(&app, site_id, name.trim(), &file_name, q.captured_at).await?;
    let dir = app.cfg.capture_dir(id);
    match write_body(&dir.join(&file_name), body, app.cfg.max_upload).await {
        Ok(size) => finish(&app, id, site_id, size, Some(me.id)).await?,
        Err(e) => {
            sqlx::query("DELETE FROM captures WHERE id = ?")
                .bind(id)
                .execute(&app.db)
                .await?;
            let _ = tokio::fs::remove_dir_all(&dir).await;
            return Err(e);
        }
    }
    Ok((
        StatusCode::CREATED,
        Json(
            db::get_capture(&app.db, id)
                .await?
                .ok_or_else(ApiError::not_found)?,
        ),
    ))
}

/// Neue Aufnahme ohne Datei; die Datei gehört nach `capture_dir(id)/file_name`, dann [`finish`]
pub async fn insert(
    app: &AppRef,
    site_id: i64,
    name: &str,
    file_name: &str,
    captured_at: Option<i64>,
) -> anyhow::Result<i64> {
    let now = now_ms();
    Ok(sqlx::query(
        "INSERT INTO captures(site_id, name, file_name, file_size, captured_at, status, created_at) VALUES(?, ?, ?, 0, ?, ?, ?)",
    )
    .bind(site_id)
    .bind(name)
    .bind(file_name)
    .bind(captured_at.unwrap_or(now))
    .bind(status::UPLOADED)
    .bind(now)
    .execute(&app.db)
    .await?
    .last_insert_rowid())
}

/// Datei liegt bereit: Größe eintragen und die Prüfung einreihen
pub async fn finish(
    app: &AppRef,
    id: i64,
    site_id: i64,
    size: u64,
    user: Option<i64>,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE captures SET file_size = ? WHERE id = ?")
        .bind(size as i64)
        .bind(id)
        .execute(&app.db)
        .await?;
    jobs::enqueue(
        app,
        site_id,
        jobs::INSPECT,
        &json!({ "captureId": id }),
        user,
    )
    .await?;
    app.events.changed(Topic::Captures, Some(site_id));
    Ok(())
}

pub(crate) async fn write_body(path: &std::path::Path, body: Body, limit: u64) -> ApiResult<u64> {
    if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir).await?;
    }
    let mut f = tokio::fs::File::create(path).await?;
    let mut size = 0u64;
    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk
            .map_err(|e| ApiError::bad_request(code::INVALID_INPUT).with_detail(e.to_string()))?;
        size += chunk.len() as u64;
        if size > limit {
            return Err(ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                code::FILE_TOO_LARGE,
            ));
        }
        f.write_all(&chunk).await?;
    }
    f.flush().await?;
    if size == 0 {
        return Err(ApiError::bad_request(code::INVALID_INPUT).with_detail("leere Datei"));
    }
    Ok(size)
}

pub async fn list(
    State(app): State<AppRef>,
    Path(site_id): Path<i64>,
) -> ApiResult<Json<Vec<Capture>>> {
    Ok(Json(
        sqlx::query_as("SELECT * FROM captures WHERE site_id = ? ORDER BY captured_at, id")
            .bind(site_id)
            .fetch_all(&app.db)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturePatch {
    name: Option<String>,
    captured_at: Option<i64>,
}

pub async fn update(
    State(app): State<AppRef>,
    Path(id): Path<i64>,
    Json(p): Json<CapturePatch>,
) -> ApiResult<Json<Capture>> {
    let c = db::get_capture(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let name = p
        .name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or(c.name);
    sqlx::query("UPDATE captures SET name = ?, captured_at = ? WHERE id = ?")
        .bind(name)
        .bind(p.captured_at.unwrap_or(c.captured_at))
        .bind(id)
        .execute(&app.db)
        .await?;
    app.events.changed(Topic::Captures, Some(c.site_id));
    Ok(Json(
        db::get_capture(&app.db, id)
            .await?
            .ok_or_else(ApiError::not_found)?,
    ))
}

pub async fn delete(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<StatusCode> {
    let c = db::get_capture(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    sqlx::query("DELETE FROM captures WHERE id = ?")
        .bind(id)
        .execute(&app.db)
        .await?;
    let _ = tokio::fs::remove_dir_all(app.cfg.capture_dir(id)).await;
    app.events.changed(Topic::Captures, Some(c.site_id));
    Ok(StatusCode::NO_CONTENT)
}

pub async fn download(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<Response> {
    let c = db::get_capture(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let f = tokio::fs::File::open(jobs::capture_path(&app, &c))
        .await
        .map_err(|_| ApiError::not_found())?;
    let disposition = format!("attachment; filename=\"{}\"", c.file_name);
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        Body::from_stream(tokio_util::io::ReaderStream::new(f)),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::safe_file_name;

    #[test]
    fn file_names_are_sanitized() {
        assert_eq!(
            safe_file_name("nachher.laz").as_deref(),
            Some("nachher.laz")
        );
        assert_eq!(
            safe_file_name("../../etc/passwd").as_deref(),
            Some("passwd")
        );
        assert_eq!(
            safe_file_name("C:\\Flug 3\\dsm (1).tif").as_deref(),
            Some("dsm__1_.tif")
        );
        assert_eq!(safe_file_name(".hidden.las").as_deref(), Some("hidden.las"));
        assert_eq!(safe_file_name(""), None);
        assert_eq!(safe_file_name("/"), None);
    }
}
