//! Orthofoto einer Aufnahme: RGB-GeoTIFF hochladen (oder aus NodeODM), verkleinert als PNG ablegen, Ecken im
//! Bezugssystem der Baustelle. Die Ansicht legt das Bild über die drei Ecken affin an die richtige Stelle – so
//! passt auch ein Orthofoto aus einer anderen UTM-Zone.

use crate::captures::{safe_file_name, write_body};
use crate::db;
use crate::error::{code, ApiError, ApiResult};
use crate::events::Topic;
use crate::state::AppRef;
use anyhow::{Context, Result};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use fixpunkt::crs::{utm_zone, Utm};
use fixpunkt::ortho::{read_downsampled, Ortho};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path as FsPath;

/// Längste Seite des Bildes für die Ansicht (bei 100 m Baustelle ≈ 2,5 cm je Pixel)
pub const MAX_SIDE: u32 = 4096;
pub const SOURCE: &str = "ortho.tif";
pub const IMAGE: &str = "ortho.png";

/// Ecken links oben, rechts oben, links unten – im System des Orthofotos, dann in der Zone der Baustelle
pub fn corners(o: &Ortho, site_epsg: i64) -> Result<[[f64; 2]; 3]> {
    let g = o.georef;
    let (w, h) = (f64::from(o.width) * g.cell, f64::from(o.height) * g.cell);
    let raw = [
        [g.min_x, g.max_y],
        [g.min_x + w, g.max_y],
        [g.min_x, g.max_y - h],
    ];
    let to = utm_zone(u32::try_from(site_epsg)?).context("Baustelle nicht in UTM")?;
    let from = g.epsg.and_then(|e| utm_zone(u32::from(e))).unwrap_or(to);
    let (from, to) = (Utm::new(from)?, Utm::new(to)?);
    Ok(raw.map(|[x, y]| {
        let (x, y) = from.to_zone(&to, x, y);
        [x, y]
    }))
}

/// Orthofoto `src` aufbereiten: PNG nach `dir`, Beschreibung zurück
pub fn prepare(src: &FsPath, dir: &FsPath, site_epsg: i64) -> Result<Value> {
    let o = read_downsampled(src, MAX_SIDE)?;
    let img = image::RgbaImage::from_raw(o.width, o.height, o.rgba.clone())
        .context("Bildgröße passt nicht")?;
    img.save_with_format(dir.join(IMAGE), image::ImageFormat::Png)?;
    Ok(json!({
        "width": o.width,
        "height": o.height,
        "corners": corners(&o, site_epsg)?,
        "sourceEpsg": o.georef.epsg,
        "sourceSize": [o.source.0, o.source.1],
    }))
}

/// Orthofoto an eine Aufnahme hängen (Datei liegt schon als `captures/<id>/ortho.tif`)
pub async fn attach(app: &AppRef, capture_id: i64) -> Result<()> {
    let c = db::get_capture(&app.db, capture_id)
        .await?
        .context("Aufnahme fehlt")?;
    let site = db::get_site(&app.db, c.site_id)
        .await?
        .context("Baustelle fehlt")?;
    let dir = app.cfg.capture_dir(capture_id);
    let d = dir.clone();
    let meta =
        tokio::task::spawn_blocking(move || prepare(&d.join(SOURCE), &d, site.epsg)).await??;
    sqlx::query("UPDATE captures SET ortho = ? WHERE id = ?")
        .bind(meta.to_string())
        .bind(capture_id)
        .execute(&app.db)
        .await?;
    app.events.changed(Topic::Captures, Some(c.site_id));
    Ok(())
}

#[derive(Deserialize)]
pub struct UploadQuery {
    file: String,
}

/// `PUT /api/captures/{id}/ortho?file=orthophoto.tif` mit dem GeoTIFF als Body
pub async fn upload(
    State(app): State<AppRef>,
    Path(id): Path<i64>,
    Query(q): Query<UploadQuery>,
    body: Body,
) -> ApiResult<Json<db::Capture>> {
    db::get_capture(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let file = safe_file_name(&q.file).ok_or_else(|| ApiError::bad_request(code::INVALID_INPUT))?;
    let lower = file.to_ascii_lowercase();
    if !(lower.ends_with(".tif") || lower.ends_with(".tiff")) {
        return Err(ApiError::bad_request(code::ORTHO_INVALID).with_detail("nur GeoTIFF"));
    }
    let dir = app.cfg.capture_dir(id);
    write_body(&dir.join(SOURCE), body, app.cfg.max_upload).await?;
    if let Err(e) = attach(&app, id).await {
        let _ = tokio::fs::remove_file(dir.join(SOURCE)).await;
        return Err(ApiError::bad_request(code::ORTHO_INVALID).with_detail(format!("{e:#}")));
    }
    Ok(Json(
        db::get_capture(&app.db, id)
            .await?
            .ok_or_else(ApiError::not_found)?,
    ))
}

pub async fn image(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<Response> {
    db::get_capture(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let bytes = tokio::fs::read(app.cfg.capture_dir(id).join(IMAGE))
        .await
        .map_err(|_| ApiError::not_found())?;
    Ok((
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "private, max-age=86400"),
        ],
        bytes,
    )
        .into_response())
}

pub async fn delete(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<StatusCode> {
    let c = db::get_capture(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    sqlx::query("UPDATE captures SET ortho = NULL WHERE id = ?")
        .bind(id)
        .execute(&app.db)
        .await?;
    let dir = app.cfg.capture_dir(id);
    let _ = tokio::fs::remove_file(dir.join(SOURCE)).await;
    let _ = tokio::fs::remove_file(dir.join(IMAGE)).await;
    app.events.changed(Topic::Captures, Some(c.site_id));
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixpunkt::geotiff::Georef;

    fn ortho(epsg: Option<u16>, min_x: f64, max_y: f64) -> Ortho {
        Ortho {
            width: 400,
            height: 200,
            rgba: vec![0; 400 * 200 * 4],
            georef: Georef {
                min_x,
                max_y,
                cell: 0.1,
                epsg,
            },
            source: (4000, 2000),
        }
    }

    #[test]
    fn corners_in_the_site_zone() {
        let c = corners(&ortho(Some(25832), 512_000.0, 5_403_040.0), 25832).unwrap();
        assert_eq!(
            c,
            [
                [512_000.0, 5_403_040.0],
                [512_040.0, 5_403_040.0],
                [512_000.0, 5_403_020.0]
            ]
        );
        // ohne Bezugssystem: Zone der Baustelle angenommen
        assert_eq!(
            corners(&ortho(None, 1.0, 2.0), 25832).unwrap()[0],
            [1.0, 2.0]
        );
        // Zone 33 → 32: Ecken über Breite/Länge, unabhängig geprüft mit der Vorwärtsrechnung
        let (z32, z33) = (Utm::new(32).unwrap(), Utm::new(33).unwrap());
        let (e33, n33) = z33.forward(50.0, 12.001);
        let c = corners(&ortho(Some(32633), e33, n33), 25832).unwrap();
        let (e32, n32) = z32.forward(50.0, 12.001);
        assert!((c[0][0] - e32).abs() < 1e-6 && (c[0][1] - n32).abs() < 1e-6);
        // in Zone 32 ist das Bild leicht gedreht: die Oberkante steigt nicht genau waagrecht
        assert!((c[1][1] - c[0][1]).abs() > 0.01);
    }
}
