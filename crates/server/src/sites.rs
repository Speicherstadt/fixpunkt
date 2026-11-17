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
    /// [Breite, Länge] der Baustelle für die Karte; ohne Daten leer
    #[sqlx(skip)]
    center: Option<[f64; 2]>,
}

pub async fn list(State(app): State<AppRef>) -> ApiResult<Json<Vec<SiteItem>>> {
    let mut sites: Vec<SiteItem> = sqlx::query_as(
        "SELECT s.*,
           (SELECT COUNT(*) FROM captures c WHERE c.site_id = s.id) AS captures,
           (SELECT COUNT(*) FROM jobs j WHERE j.site_id = s.id AND j.kind != 'inspect') AS jobs,
           (SELECT MAX(captured_at) FROM captures c WHERE c.site_id = s.id) AS last_capture_at
         FROM sites s ORDER BY s.name COLLATE NOCASE",
    )
    .fetch_all(&app.db)
    .await?;
    let centers = crate::location::site_centers(&app.db).await?;
    for s in &mut sites {
        s.center = centers.get(&s.site.id).copied();
    }
    Ok(Json(sites))
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
    let mut marker_dirs = crate::markers::dirs_of_site(&app, id).await?;
    marker_dirs.extend(crate::plans::dirs_of_site(&app, id).await?);
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
    for d in marker_dirs {
        let _ = tokio::fs::remove_dir_all(d).await;
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

/// Material, Massenziel und Preis eines Bereichs; was fehlt oder leer ist, wird gelöscht
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionPatch {
    name: Option<String>,
    material: Option<String>,
    target_m3: Option<f64>,
    target_kind: Option<String>,
    lv_position: Option<String>,
    unit_price: Option<f64>,
    price_unit: Option<String>,
}

fn trimmed(s: Option<String>) -> Option<String> {
    s.map(|m| m.trim().to_string()).filter(|m| !m.is_empty())
}

pub fn validate_target(p: &RegionPatch) -> ApiResult<()> {
    let bad = |d: &str| Err(ApiError::bad_request(code::INVALID_INPUT).with_detail(d.to_string()));
    if p.target_m3
        .is_some_and(|t| !t.is_finite() || t <= 0.0 || t > 1e8)
    {
        return bad("Soll-Menge");
    }
    if p.unit_price
        .is_some_and(|t| !t.is_finite() || t < 0.0 || t > 1e6)
    {
        return bad("Einheitspreis");
    }
    if !matches!(
        p.price_unit.as_deref(),
        None | Some("m3") | Some("m3_loose") | Some("t")
    ) {
        return bad("Preiseinheit: m3, m3_loose oder t");
    }
    match p.target_kind.as_deref() {
        None | Some("cut") | Some("fill") | Some("stock") => Ok(()),
        _ => bad("Art: cut, fill oder stock"),
    }
}

pub async fn update_region(
    State(app): State<AppRef>,
    Path(id): Path<i64>,
    Json(p): Json<RegionPatch>,
) -> ApiResult<Json<RegionRow>> {
    validate_target(&p)?;
    let r: RegionRow = sqlx::query_as("SELECT * FROM regions WHERE id = ?")
        .bind(id)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let name = clean_name(p.name).unwrap_or(r.name);
    let material = trimmed(p.material);
    // ein Lager wird nicht abgerechnet; ein Preis gilt nur mit Einheit (Standard: m³ fest)
    let stock = p.target_kind.as_deref() == Some("stock");
    let price = p.unit_price.filter(|_| !stock);
    let unit = price.map(|_| p.price_unit.unwrap_or_else(|| "m3".into()));
    let position = trimmed(p.lv_position).filter(|_| !stock);
    // Lager (Haufwerk) braucht keine Soll-Menge; Abtrag/Auftrag nur mit einer
    let (kind, target) = match p.target_kind.as_deref() {
        Some("stock") => (Some("stock".to_string()), None),
        _ => (
            p.target_m3
                .map(|_| p.target_kind.unwrap_or_else(|| "cut".into())),
            p.target_m3,
        ),
    };
    sqlx::query(
        "UPDATE regions SET name = ?, material = ?, target_m3 = ?, target_kind = ?, lv_position = ?,
           unit_price = ?, price_unit = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(material)
    .bind(target)
    .bind(kind)
    .bind(position)
    .bind(price)
    .bind(unit)
    .bind(id)
    .execute(&app.db)
    .await?;
    app.events.changed(Topic::Regions, Some(r.site_id));
    Ok(Json(
        sqlx::query_as("SELECT * FROM regions WHERE id = ?")
            .bind(id)
            .fetch_one(&app.db)
            .await?,
    ))
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
