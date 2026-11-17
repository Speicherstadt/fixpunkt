//! Höhenraster für die 2D-Ansicht im Browser: kompakt als Float32 (Zeile 0 = Norden, NaN = keine Daten),
//! Lage und Auflösung in den Headern `x-grid-*`. Die UI zeichnet und misst damit selbst.
//! Raster einer Aufnahme werden beim ersten Abruf berechnet und im Ordner der Aufnahme zwischengespeichert.

use crate::compute::SurfaceParams;
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::jobs::capture_path;
use crate::state::AppRef;
use anyhow::{Context, Result};
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use fixpunkt::geotiff;
use fixpunkt::grid::{Grid, GridSpec};
use fixpunkt::surface::{self, SurfaceOptions};
use std::path::Path as FsPath;

/// Obergrenze für die Ansicht: ~4 MB, flüssig im Browser
pub const MAX_CELLS: usize = 1_000_000;

/// Inhaltstyp des Binärformats (nicht octet-stream, damit gzip greift)
pub const CONTENT_TYPE: &str = "application/x-fixpunkt-grid";

/// Kleinste „runde“ Zellgröße ≥ `min`, mit der die Fläche höchstens `max_cells` Zellen hat
pub fn view_cell(width: f64, height: f64, min: f64, max_cells: usize) -> f64 {
    const STEPS: [f64; 12] = [
        0.1, 0.2, 0.25, 0.5, 1.0, 2.0, 2.5, 5.0, 10.0, 20.0, 25.0, 50.0,
    ];
    let need = (width * height / max_cells as f64).sqrt();
    STEPS
        .into_iter()
        .find(|&s| s >= need.max(min))
        .unwrap_or(100.0)
}

/// Raster so vergröbern, dass es höchstens `max_cells` Zellen hat
pub fn fit(g: Grid, max_cells: usize) -> Result<Grid> {
    if g.spec.len() <= max_cells {
        return Ok(g);
    }
    let cell = view_cell(
        g.spec.cols as f64 * g.spec.cell,
        g.spec.rows as f64 * g.spec.cell,
        g.spec.cell,
        max_cells,
    );
    Ok(g.resample(GridSpec::covering(&g.bbox(), cell)?))
}

/// Oberfläche einer Aufnahme für die Ansicht (ohne Bodenfilter: man sieht, was geflogen wurde)
pub fn surface_view(path: &FsPath, max_cells: usize) -> Result<Grid> {
    let base = SurfaceParams::default().options();
    if surface::is_raster(path) {
        return fit(surface::load(path, None, None, &base)?, max_cells);
    }
    let bbox = surface::extent(path, &base)?;
    let cell = view_cell(bbox.width(), bbox.height(), 0.25, max_cells);
    let o = SurfaceOptions { cell, ..base };
    fit(surface::load(path, None, None, &o)?, max_cells)
}

/// Binärform: f32 little endian
pub fn encode(g: &Grid) -> Vec<u8> {
    g.data.iter().flat_map(|v| v.to_le_bytes()).collect()
}

pub fn decode(spec: GridSpec, bytes: &[u8]) -> Result<Grid> {
    anyhow::ensure!(
        bytes.len() == spec.len() * 4,
        "Rasterdatei passt nicht zur Größe"
    );
    let data = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect();
    Ok(Grid { spec, data })
}

fn respond(g: &Grid) -> Response {
    let s = g.spec;
    (
        [
            (header::CONTENT_TYPE, CONTENT_TYPE.to_string()),
            (
                header::HeaderName::from_static("x-grid-min-x"),
                s.min_x.to_string(),
            ),
            (
                header::HeaderName::from_static("x-grid-max-y"),
                s.max_y.to_string(),
            ),
            (
                header::HeaderName::from_static("x-grid-cell"),
                s.cell.to_string(),
            ),
            (
                header::HeaderName::from_static("x-grid-cols"),
                s.cols.to_string(),
            ),
            (
                header::HeaderName::from_static("x-grid-rows"),
                s.rows.to_string(),
            ),
            (header::CACHE_CONTROL, "private, max-age=86400".to_string()),
        ],
        encode(g),
    )
        .into_response()
}

/// Zwischenspeicher im Ordner der Aufnahme: `ansicht.json` (Lage) und `ansicht.f32` (Werte)
fn cached(dir: &FsPath, make: impl FnOnce() -> Result<Grid>) -> Result<Grid> {
    let (meta, data) = (dir.join("ansicht.json"), dir.join("ansicht.f32"));
    if let (Ok(m), Ok(d)) = (std::fs::read_to_string(&meta), std::fs::read(&data)) {
        let v: serde_json::Value = serde_json::from_str(&m)?;
        let num = |k: &str| v[k].as_f64().context("Ansicht: Lage fehlt");
        let spec = GridSpec {
            min_x: num("minX")?,
            max_y: num("maxY")?,
            cell: num("cell")?,
            cols: num("cols")? as usize,
            rows: num("rows")? as usize,
        };
        if let Ok(g) = decode(spec, &d) {
            return Ok(g);
        }
    }
    let g = make()?;
    let s = g.spec;
    std::fs::write(&data, encode(&g))?;
    std::fs::write(
        &meta,
        serde_json::json!({"minX": s.min_x, "maxY": s.max_y, "cell": s.cell, "cols": s.cols, "rows": s.rows}).to_string(),
    )?;
    Ok(g)
}

/// Höhenraster einer Aufnahme wie in der Ansicht (zwischengespeichert)
pub async fn capture_view(app: &AppRef, c: &db::Capture) -> Result<Grid> {
    let (path, dir) = (capture_path(app, c), app.cfg.capture_dir(c.id));
    tokio::task::spawn_blocking(move || cached(&dir, || surface_view(&path, MAX_CELLS))).await?
}

pub async fn capture_grid(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<Response> {
    let c = db::get_capture(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    Ok(respond(&capture_view(&app, &c).await?))
}

/// Raster eines Ergebnisses (Differenz, Abweichung) als Überlagerung
pub async fn job_grid(
    State(app): State<AppRef>,
    Path((id, name)): Path<(i64, String)>,
) -> ApiResult<Response> {
    let job = db::get_job(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let result: serde_json::Value = job
        .result
        .as_deref()
        .map(serde_json::from_str)
        .transpose()?
        .unwrap_or_default();
    let listed = result["files"]
        .as_array()
        .is_some_and(|f| f.iter().any(|n| n.as_str() == Some(name.as_str())));
    if !listed || !name.ends_with(".tif") {
        return Err(ApiError::not_found());
    }
    let path = app.cfg.job_dir(id).join(&name);
    let g = tokio::task::spawn_blocking(move || fit(geotiff::read(&path)?.grid, MAX_CELLS))
        .await
        .map_err(anyhow::Error::from)??;
    Ok(respond(&g))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute::testdata::write_cloud;

    #[test]
    fn cell_sizes_stay_round_and_bounded() {
        assert_eq!(view_cell(40.0, 40.0, 0.25, MAX_CELLS), 0.25);
        // 1 km²: mindestens 1 m
        assert_eq!(view_cell(1000.0, 1000.0, 0.25, MAX_CELLS), 1.0);
        assert_eq!(view_cell(1500.0, 1000.0, 0.25, MAX_CELLS), 2.0);
        assert_eq!(view_cell(10.0, 10.0, 0.5, MAX_CELLS), 0.5);
    }

    #[test]
    fn view_of_cloud_is_cached_and_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.laz");
        write_cloud(&p, true);
        let g = cached(dir.path(), || surface_view(&p, MAX_CELLS)).unwrap();
        assert_eq!((g.spec.cols, g.spec.rows, g.spec.cell), (160, 160, 0.25));
        assert!(dir.path().join("ansicht.f32").exists());
        // zweiter Abruf aus dem Zwischenspeicher, ohne die Wolke zu lesen
        let again = cached(dir.path(), || anyhow::bail!("darf nicht neu rechnen")).unwrap();
        assert_eq!(again.spec, g.spec);
        assert_eq!(again.data.len(), g.data.len());
        // vergröbern auf höchstens 1.000 Zellen
        let small = fit(g, 1000).unwrap();
        assert!(small.spec.len() <= 1000, "{}", small.spec.len());
        assert!(decode(small.spec, &encode(&small)[..8]).is_err());
    }
}
