//! Abruf der Baustellen-Berichte: `GET /api/sites/{id}/documentation/report` und `GET /api/plans/{id}/report`.
//! Sammelt Einträge, Gelände der letzten geprüften Aufnahme und Vorschaubilder; der Inhalt entsteht in `site.rs`.

use super::site::{documentation_doc, plan_doc, Entry, SiteContext, Spot};
use super::{file_name, pdf};
use crate::db::{self, now_ms, status, Capture};
use crate::error::{ApiError, ApiResult};
use crate::markers::{self, Marker};
use crate::state::AppRef;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use fixpunkt::grid::Grid;
use serde_json::Value;

/// Letzte geprüfte Aufnahme der Baustelle und ihr Höhenraster
async fn latest(app: &AppRef, site: i64) -> ApiResult<Option<(Capture, Grid)>> {
    let c: Option<Capture> = sqlx::query_as(
        "SELECT * FROM captures WHERE site_id = ? AND status = ? ORDER BY captured_at DESC, id DESC LIMIT 1",
    )
    .bind(site)
    .bind(status::READY)
    .fetch_optional(&app.db)
    .await?;
    let Some(c) = c else { return Ok(None) };
    let g = crate::view::capture_view(app, &c).await?;
    Ok(Some((c, g)))
}

/// Vorschaubild mit Größe; es ist schon ein RGB-JPEG (markers/photo.rs)
fn thumb(path: &std::path::Path) -> Option<(Vec<u8>, usize, usize)> {
    let bytes = std::fs::read(path).ok()?;
    let (w, h) = image::ImageReader::new(std::io::Cursor::new(&bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()?;
    Some((bytes, w as usize, h as usize))
}

fn pdf_response(bytes: Vec<u8>, name: String) -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        bytes,
    )
        .into_response()
}

async fn render(doc: super::doc::Doc) -> ApiResult<Vec<u8>> {
    Ok(tokio::task::spawn_blocking(move || pdf::render(&doc))
        .await
        .map_err(anyhow::Error::from)?)
}

pub async fn documentation(
    State(app): State<AppRef>,
    Path(site): Path<i64>,
) -> ApiResult<Response> {
    let s = db::get_site(&app.db, site)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let list: Vec<Marker> = sqlx::query_as(
        "SELECT * FROM markers WHERE site_id = ? ORDER BY kind, COALESCE(taken_at, created_at), id",
    )
    .bind(site)
    .fetch_all(&app.db)
    .await?;
    let last = latest(&app, site).await?;
    let entries: Vec<Entry> = list
        .into_iter()
        .map(|m| {
            let ist = match (&last, m.kind.as_str(), m.x, m.y) {
                (Some((_, g)), "constraint", Some(x), Some(y)) => g.sample(x, y),
                _ => None,
            };
            let photo = m
                .photo
                .as_ref()
                .and_then(|_| thumb(&markers::dir(&app, m.id).join("thumb.jpg")));
            let line = (m.kind == "line").then(|| {
                let pts = m.line();
                (markers::line_length(&pts), pts.len())
            });
            Entry {
                line,
                direction: m.direction,
                at: m.taken_at.unwrap_or(m.created_at),
                kind: m.kind,
                title: m.title,
                note: m.note,
                x: m.x,
                y: m.y,
                z: m.z,
                ist,
                photo,
            }
        })
        .collect();
    let ctx = SiteContext {
        site: s.name.clone(),
        epsg: s.epsg,
        now: now_ms(),
        against: last.map(|(c, _)| c.name),
    };
    let bytes = render(documentation_doc(&ctx, &entries)).await?;
    Ok(pdf_response(
        bytes,
        file_name("Dokumentation", &s.name, site),
    ))
}

pub async fn plan(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<Response> {
    let (site, name): (i64, String) =
        sqlx::query_as("SELECT site_id, name FROM plans WHERE id = ?")
            .bind(id)
            .fetch_optional(&app.db)
            .await?
            .ok_or_else(ApiError::not_found)?;
    let s = db::get_site(&app.db, site)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let geometry: Value = serde_json::from_slice(
        &tokio::fs::read(crate::plans::dir(&app, id).join("geometry.json")).await?,
    )?;
    let last = latest(&app, site).await?;
    let spots: Vec<Spot> = geometry["spots"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| {
            let at = [
                s["at"][0].as_f64()?,
                s["at"][1].as_f64()?,
                s["at"][2].as_f64()?,
            ];
            Some(Spot {
                layer: s["layer"].as_str().unwrap_or("").into(),
                at,
                ist: last.as_ref().and_then(|(_, g)| g.sample(at[0], at[1])),
            })
        })
        .collect();
    let ctx = SiteContext {
        site: s.name.clone(),
        epsg: s.epsg,
        now: now_ms(),
        against: last.map(|(c, _)| c.name),
    };
    let bytes = render(plan_doc(&ctx, &name, &spots)).await?;
    Ok(pdf_response(bytes, file_name("Plan-Ist", &s.name, id)))
}
