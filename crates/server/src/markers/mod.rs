//! Dokumentation einer Baustelle: Zwangspunkte (mit Sollhöhe), Fotos (Position aus EXIF), Notizen. Eine
//! Markierung hat höchstens ein Foto; Datei und Vorschaubild liegen unter `markers/<id>/`.

pub mod photo;

use crate::auth::CurrentUser;
use crate::captures::safe_file_name;
use crate::db::{self, now_ms};
use crate::error::{code, ApiError, ApiResult};
use crate::events::Topic;
use crate::state::AppRef;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use fixpunkt::crs::{utm_zone, Utm};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::path::PathBuf;

pub const KINDS: &[&str] = &["constraint", "photo", "note"];
/// Größtes Foto in Bytes
pub const MAX_PHOTO: usize = 50_000_000;

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Marker {
    pub id: i64,
    pub site_id: i64,
    pub kind: String,
    pub title: String,
    pub note: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    pub photo: Option<String>,
    pub taken_at: Option<i64>,
    pub created_by: Option<i64>,
    pub created_at: i64,
}

pub(crate) fn dir(app: &AppRef, id: i64) -> PathBuf {
    app.cfg.data_dir.join("markers").join(id.to_string())
}

async fn get(app: &AppRef, id: i64) -> ApiResult<Marker> {
    sqlx::query_as("SELECT * FROM markers WHERE id = ?")
        .bind(id)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(ApiError::not_found)
}

pub async fn list(
    State(app): State<AppRef>,
    Path(site): Path<i64>,
) -> ApiResult<Json<Vec<Marker>>> {
    Ok(Json(
        sqlx::query_as(
            "SELECT * FROM markers WHERE site_id = ? ORDER BY COALESCE(taken_at, created_at) DESC, id DESC",
        )
        .bind(site)
        .fetch_all(&app.db)
        .await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct NewMarker {
    kind: String,
    title: String,
    #[serde(default)]
    note: String,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
}

/// Prüft Art, Titel und Koordinaten; ein Zwangspunkt braucht Lage und Sollhöhe
pub fn validate(
    kind: &str,
    title: &str,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
) -> ApiResult<()> {
    if !KINDS.contains(&kind) {
        return Err(ApiError::bad_request(code::INVALID_INPUT).with_detail("Art"));
    }
    if title.trim().is_empty() {
        return Err(ApiError::bad_request(code::NAME_REQUIRED));
    }
    if [x, y, z].iter().flatten().any(|v| !v.is_finite()) || x.is_some() != y.is_some() {
        return Err(ApiError::bad_request(code::INVALID_INPUT).with_detail("Koordinaten"));
    }
    if kind == "constraint" && (x.is_none() || z.is_none()) {
        return Err(ApiError::bad_request(code::MARKER_NEEDS_POSITION));
    }
    Ok(())
}

pub async fn create(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(site): Path<i64>,
    Json(m): Json<NewMarker>,
) -> ApiResult<(StatusCode, Json<Marker>)> {
    db::get_site(&app.db, site)
        .await?
        .ok_or_else(ApiError::not_found)?;
    validate(&m.kind, &m.title, m.x, m.y, m.z)?;
    let id = sqlx::query(
        "INSERT INTO markers(site_id, kind, title, note, x, y, z, created_by, created_at) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(site)
    .bind(&m.kind)
    .bind(m.title.trim())
    .bind(m.note.trim())
    .bind(m.x)
    .bind(m.y)
    .bind(m.z)
    .bind(me.id)
    .bind(now_ms())
    .execute(&app.db)
    .await?
    .last_insert_rowid();
    app.events.changed(Topic::Markers, Some(site));
    Ok((StatusCode::CREATED, Json(get(&app, id).await?)))
}

#[derive(Debug, Deserialize)]
pub struct MarkerPatch {
    kind: Option<String>,
    title: Option<String>,
    note: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
}

pub async fn update(
    State(app): State<AppRef>,
    Path(id): Path<i64>,
    Json(p): Json<MarkerPatch>,
) -> ApiResult<Json<Marker>> {
    let m = get(&app, id).await?;
    let next = Marker {
        kind: p.kind.unwrap_or(m.kind),
        title: p.title.map(|t| t.trim().to_string()).unwrap_or(m.title),
        note: p.note.map(|t| t.trim().to_string()).unwrap_or(m.note),
        x: p.x.or(m.x),
        y: p.y.or(m.y),
        z: p.z.or(m.z),
        ..m
    };
    validate(&next.kind, &next.title, next.x, next.y, next.z)?;
    sqlx::query(
        "UPDATE markers SET kind = ?, title = ?, note = ?, x = ?, y = ?, z = ? WHERE id = ?",
    )
    .bind(&next.kind)
    .bind(&next.title)
    .bind(&next.note)
    .bind(next.x)
    .bind(next.y)
    .bind(next.z)
    .bind(id)
    .execute(&app.db)
    .await?;
    app.events.changed(Topic::Markers, Some(next.site_id));
    Ok(Json(get(&app, id).await?))
}

pub async fn delete(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<StatusCode> {
    let m = get(&app, id).await?;
    sqlx::query("DELETE FROM markers WHERE id = ?")
        .bind(id)
        .execute(&app.db)
        .await?;
    let _ = tokio::fs::remove_dir_all(dir(&app, id)).await;
    app.events.changed(Topic::Markers, Some(m.site_id));
    Ok(StatusCode::NO_CONTENT)
}

/// Breite/Länge → Lage im Bezugssystem der Baustelle (nur UTM)
pub fn site_position(epsg: i64, lat: f64, lon: f64) -> Option<(f64, f64)> {
    let zone = utm_zone(u32::try_from(epsg).ok()?)?;
    Some(Utm::new(zone).ok()?.forward(lat, lon))
}

#[derive(Deserialize)]
pub struct PhotoQuery {
    file: String,
}

/// `PUT /api/sites/{id}/photos?file=IMG_0001.jpg` mit dem Foto als Body: neue Markierung, Lage aus GPS
pub async fn upload_photo(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(site): Path<i64>,
    Query(q): Query<PhotoQuery>,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<Marker>)> {
    let s = db::get_site(&app.db, site)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let file = safe_file_name(&q.file).ok_or_else(|| ApiError::bad_request(code::INVALID_INPUT))?;
    let bytes = body.to_vec();
    let (thumb, exif) = tokio::task::spawn_blocking(move || {
        (
            photo::thumbnail(&bytes).ok(),
            photo::read_exif(&bytes),
            bytes,
        )
    })
    .await
    .map(|(t, e, b)| (t.map(|t| (t, b)), e))
    .map_err(anyhow::Error::from)?;
    let Some((thumb, bytes)) = thumb else {
        return Err(ApiError::bad_request(code::MARKER_NOT_AN_IMAGE));
    };
    let pos = exif
        .lat_lon
        .and_then(|(lat, lon)| site_position(s.epsg, lat, lon));
    let title = file
        .rsplit_once('.')
        .map_or(file.as_str(), |(stem, _)| stem);
    let id = sqlx::query(
        "INSERT INTO markers(site_id, kind, title, x, y, photo, taken_at, created_by, created_at) VALUES(?, 'photo', ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(site)
    .bind(title)
    .bind(pos.map(|p| p.0))
    .bind(pos.map(|p| p.1))
    .bind(&file)
    .bind(exif.taken_at)
    .bind(me.id)
    .bind(now_ms())
    .execute(&app.db)
    .await?
    .last_insert_rowid();
    let d = dir(&app, id);
    tokio::fs::create_dir_all(&d).await?;
    tokio::fs::write(d.join(&file), &bytes).await?;
    tokio::fs::write(d.join("thumb.jpg"), &thumb).await?;
    app.events.changed(Topic::Markers, Some(site));
    Ok((StatusCode::CREATED, Json(get(&app, id).await?)))
}

async fn send_file(path: PathBuf, content_type: &'static str) -> ApiResult<Response> {
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| ApiError::not_found())?;
    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "private, max-age=86400"),
        ],
        bytes,
    )
        .into_response())
}

pub async fn photo(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<Response> {
    let m = get(&app, id).await?;
    let file = m.photo.ok_or_else(ApiError::not_found)?;
    let ct = if file.to_ascii_lowercase().ends_with(".png") {
        "image/png"
    } else {
        "image/jpeg"
    };
    send_file(dir(&app, id).join(file), ct).await
}

pub async fn thumb(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<Response> {
    get(&app, id).await?;
    send_file(dir(&app, id).join("thumb.jpg"), "image/jpeg").await
}

/// Ordner der Markierungen einer Baustelle (zum Aufräumen beim Löschen der Baustelle)
pub async fn dirs_of_site(app: &AppRef, site: i64) -> ApiResult<Vec<PathBuf>> {
    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM markers WHERE site_id = ?")
        .bind(site)
        .fetch_all(&app.db)
        .await?;
    Ok(ids.into_iter().map(|id| dir(app, id)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_for_markers() {
        assert!(validate("note", "Bodenprobe", None, None, None).is_ok());
        assert!(validate("constraint", "Kanal", Some(1.0), Some(2.0), Some(298.1)).is_ok());
        let e = validate("constraint", "Kanal", Some(1.0), Some(2.0), None).unwrap_err();
        assert_eq!(e.code, code::MARKER_NEEDS_POSITION);
        assert_eq!(
            validate("note", " ", None, None, None).unwrap_err().code,
            code::NAME_REQUIRED
        );
        assert!(validate("foo", "x", None, None, None).is_err());
        assert!(validate("note", "x", Some(1.0), None, None).is_err());
        assert!(validate("note", "x", Some(f64::NAN), Some(1.0), None).is_err());
    }

    #[test]
    fn gps_to_site_position() {
        // unabhängig: Rückweg über die Umkehrung
        let (e, n) = site_position(25832, 48.780019664, 9.163685433).unwrap();
        let (lat, lon) = Utm::new(32).unwrap().inverse(e, n);
        assert!((lat - 48.780019664).abs() < 1e-9 && (lon - 9.163685433).abs() < 1e-9);
        assert!(
            (e - 512_025.0).abs() < 0.01 && (n - 5_403_015.0).abs() < 0.01,
            "{e} {n}"
        );
        assert_eq!(site_position(4326, 48.0, 9.0), None);
    }
}
