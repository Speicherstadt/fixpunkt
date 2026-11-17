//! Baustellen und ihre Abrechnungsbereiche.

use crate::compute;
use crate::db::{self, now_ms, RegionRow, Site};
use crate::error::{code, ApiError, ApiResult};
use crate::events::Topic;
use crate::state::AppRef;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use fixpunkt::geom::Polygon;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SiteItem {
    #[serde(flatten)]
    #[sqlx(flatten)]
    site: Site,
    captures: i64,
    jobs: i64,
    last_capture_at: Option<i64>,
}

pub async fn list(State(app): State<AppRef>) -> ApiResult<Json<Vec<SiteItem>>> {
    Ok(Json(
        sqlx::query_as(
            "SELECT s.*,
               (SELECT COUNT(*) FROM captures c WHERE c.site_id = s.id) AS captures,
               (SELECT COUNT(*) FROM jobs j WHERE j.site_id = s.id AND j.kind != 'inspect') AS jobs,
               (SELECT MAX(captured_at) FROM captures c WHERE c.site_id = s.id) AS last_capture_at
             FROM sites s ORDER BY s.name COLLATE NOCASE",
        )
        .fetch_all(&app.db)
        .await?,
    ))
}

#[derive(Deserialize)]
pub struct SiteInput {
    name: Option<String>,
    description: Option<String>,
    epsg: Option<i64>,
}

fn clean_name(n: Option<String>) -> Option<String> {
    n.map(|n| n.trim().to_string()).filter(|n| !n.is_empty())
}

pub async fn create(
    State(app): State<AppRef>,
    Json(s): Json<SiteInput>,
) -> ApiResult<(StatusCode, Json<Site>)> {
    let name = clean_name(s.name).ok_or_else(|| ApiError::bad_request(code::NAME_REQUIRED))?;
    let epsg = s.epsg.unwrap_or(25832);
    if !(1024..=32767).contains(&epsg) {
        return Err(ApiError::bad_request(code::INVALID_INPUT).with_detail("epsg"));
    }
    let id =
        sqlx::query("INSERT INTO sites(name, description, epsg, created_at) VALUES(?, ?, ?, ?)")
            .bind(name)
            .bind(s.description.unwrap_or_default().trim())
            .bind(epsg)
            .bind(now_ms())
            .execute(&app.db)
            .await?
            .last_insert_rowid();
    app.events.changed(Topic::Sites, None);
    Ok((
        StatusCode::CREATED,
        Json(
            db::get_site(&app.db, id)
                .await?
                .ok_or_else(ApiError::not_found)?,
        ),
    ))
}

pub async fn get(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<Json<Site>> {
    Ok(Json(
        db::get_site(&app.db, id)
            .await?
            .ok_or_else(ApiError::not_found)?,
    ))
}

pub async fn update(
    State(app): State<AppRef>,
    Path(id): Path<i64>,
    Json(s): Json<SiteInput>,
) -> ApiResult<Json<Site>> {
    let old = db::get_site(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    sqlx::query("UPDATE sites SET name = ?, description = ? WHERE id = ?")
        .bind(clean_name(s.name).unwrap_or(old.name))
        .bind(
            s.description
                .map(|d| d.trim().to_string())
                .unwrap_or(old.description),
        )
        .bind(id)
        .execute(&app.db)
        .await?;
    app.events.changed(Topic::Sites, None);
    Ok(Json(
        db::get_site(&app.db, id)
            .await?
            .ok_or_else(ApiError::not_found)?,
    ))
}

pub async fn delete(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<StatusCode> {
    db::get_site(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let captures: Vec<i64> = sqlx::query_scalar("SELECT id FROM captures WHERE site_id = ?")
        .bind(id)
        .fetch_all(&app.db)
        .await?;
    let jobs: Vec<i64> = sqlx::query_scalar("SELECT id FROM jobs WHERE site_id = ?")
        .bind(id)
        .fetch_all(&app.db)
        .await?;
    sqlx::query("DELETE FROM sites WHERE id = ?")
        .bind(id)
        .execute(&app.db)
        .await?;
    for c in captures {
        let _ = tokio::fs::remove_dir_all(app.cfg.capture_dir(c)).await;
    }
    for j in jobs {
        let _ = tokio::fs::remove_dir_all(app.cfg.job_dir(j)).await;
    }
    app.events.changed(Topic::Sites, None);
    Ok(StatusCode::NO_CONTENT)
}

// ---- Bereiche -------------------------------------------------------------------------

pub async fn list_regions(
    State(app): State<AppRef>,
    Path(site_id): Path<i64>,
) -> ApiResult<Json<Vec<RegionRow>>> {
    Ok(Json(
        sqlx::query_as("SELECT * FROM regions WHERE site_id = ? ORDER BY name COLLATE NOCASE")
            .bind(site_id)
            .fetch_all(&app.db)
            .await?,
    ))
}

#[derive(Deserialize)]
pub struct RegionInput {
    /// Name, wenn das GeoJSON genau ein Polygon enthält (sonst die Namen der Features)
    name: Option<String>,
    /// GeoJSON als Objekt oder Text: metrisch (mit `crs`) oder Länge/Breite (wird nach UTM32 umgerechnet)
    geojson: Value,
}

/// Legt je Polygon im GeoJSON einen Bereich an.
pub async fn create_regions(
    State(app): State<AppRef>,
    Path(site_id): Path<i64>,
    Json(r): Json<RegionInput>,
) -> ApiResult<(StatusCode, Json<Vec<RegionRow>>)> {
    let site = db::get_site(&app.db, site_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let text = match &r.geojson {
        Value::String(s) => s.clone(),
        v => v.to_string(),
    };
    let mut regions = Polygon::regions_from_geojson_str(&text, fixpunkt::crs::Utm::default())
        .map_err(|e| ApiError::bad_request(code::INVALID_GEOJSON).with_detail(format!("{e:#}")))?;
    if let (Some(n), 1) = (clean_name(r.name), regions.len()) {
        regions[0].name = n;
    }
    let epsg = u16::try_from(site.epsg).unwrap_or(25832);
    let mut ids = Vec::new();
    for reg in &regions {
        let gj = compute::region_geojson(&reg.name, &reg.polygon, epsg).to_string();
        let id = sqlx::query("INSERT INTO regions(site_id, name, geojson, area_m2, created_at) VALUES(?, ?, ?, ?, ?)")
            .bind(site_id)
            .bind(&reg.name)
            .bind(gj)
            .bind(reg.polygon.area())
            .bind(now_ms())
            .execute(&app.db)
            .await?
            .last_insert_rowid();
        ids.push(id);
    }
    let mut out = Vec::new();
    for id in ids {
        out.push(
            sqlx::query_as("SELECT * FROM regions WHERE id = ?")
                .bind(id)
                .fetch_one(&app.db)
                .await?,
        );
    }
    app.events.changed(Topic::Regions, Some(site_id));
    Ok((StatusCode::CREATED, Json(out)))
}

pub async fn delete_region(
    State(app): State<AppRef>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    let site: Option<i64> = sqlx::query_scalar("SELECT site_id FROM regions WHERE id = ?")
        .bind(id)
        .fetch_optional(&app.db)
        .await?;
    let site = site.ok_or_else(ApiError::not_found)?;
    sqlx::query("DELETE FROM regions WHERE id = ?")
        .bind(id)
        .execute(&app.db)
        .await?;
    app.events.changed(Topic::Regions, Some(site));
    Ok(StatusCode::NO_CONTENT)
}
