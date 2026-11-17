//! Job-Warteschlange in SQLite, abgearbeitet im Prozess. Übersteht Neustarts: Aufträge, die beim
//! Beenden liefen, werden beim Start wieder eingereiht. Die Auftragsarten stehen in `spec.rs`.

pub mod spec;

pub use spec::capture_path;

use crate::auth::CurrentUser;
use crate::db::{self, now_ms, status, Job};
use crate::error::{code, ApiError, ApiResult};
use crate::events::{Event, Topic};
use crate::state::AppRef;
use anyhow::{Context, Result};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

pub const INSPECT: &str = "inspect";

/// Auftrag einreihen und einen Worker wecken
pub async fn enqueue(
    app: &AppRef,
    site_id: i64,
    kind: &str,
    params: &Value,
    user: Option<i64>,
) -> Result<i64> {
    let id = sqlx::query("INSERT INTO jobs(site_id, kind, params, status, created_by, created_at) VALUES(?, ?, ?, ?, ?, ?)")
        .bind(site_id)
        .bind(kind)
        .bind(params.to_string())
        .bind(status::QUEUED)
        .bind(user)
        .bind(now_ms())
        .execute(&app.db)
        .await?
        .last_insert_rowid();
    notify(app, id, site_id, status::QUEUED);
    app.jobs_wakeup.notify_one();
    Ok(id)
}

fn notify(app: &AppRef, id: i64, site_id: i64, s: &str) {
    app.events.send(Event::Job {
        id,
        site_id,
        status: s.into(),
    });
    app.events.changed(Topic::Jobs, Some(site_id));
}

/// Beim Start: unterbrochene Aufträge erneut einreihen
pub async fn requeue_interrupted(app: &AppRef) -> Result<u64> {
    Ok(
        sqlx::query("UPDATE jobs SET status = ?, started_at = NULL WHERE status = ?")
            .bind(status::QUEUED)
            .bind(status::RUNNING)
            .execute(&app.db)
            .await?
            .rows_affected(),
    )
}

pub fn spawn_workers(app: &AppRef) {
    for _ in 0..app.cfg.max_jobs {
        let app = app.clone();
        tokio::spawn(async move {
            loop {
                match run_next(&app).await {
                    Ok(true) => {}
                    Ok(false) => {
                        tokio::select! {
                            _ = app.jobs_wakeup.notified() => {}
                            _ = tokio::time::sleep(Duration::from_secs(10)) => {}
                        }
                    }
                    Err(e) => {
                        tracing::error!("Job-Worker: {e:#}");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });
    }
}

/// Alle wartenden Aufträge nacheinander abarbeiten (Tests, Kommandozeile)
pub async fn run_pending(app: &AppRef) -> Result<usize> {
    let mut n = 0;
    while run_next(app).await? {
        n += 1;
    }
    Ok(n)
}

/// Nächsten Auftrag übernehmen und ausführen; `false`, wenn keiner wartet.
pub async fn run_next(app: &AppRef) -> Result<bool> {
    let job: Option<Job> = sqlx::query_as(
        "UPDATE jobs SET status = ?, started_at = ?
         WHERE id = (SELECT id FROM jobs WHERE status = ? ORDER BY id LIMIT 1) RETURNING *",
    )
    .bind(status::RUNNING)
    .bind(now_ms())
    .bind(status::QUEUED)
    .fetch_optional(&app.db)
    .await?;
    let Some(job) = job else { return Ok(false) };
    notify(app, job.id, job.site_id, status::RUNNING);
    tracing::info!("Auftrag {} ({}) gestartet", job.id, job.kind);
    let outcome = match spec::JobSpec::parse(&job.kind, &serde_json::from_str(&job.params)?) {
        Ok(spec) => spec::execute(app, job.site_id, spec, app.cfg.job_dir(job.id)).await,
        Err(e) => Err(e),
    };
    let (st, result, error) = match &outcome {
        Ok(v) => (status::DONE, Some(v.to_string()), None),
        Err(e) => (status::FAILED, None, Some(format!("{e:#}"))),
    };
    sqlx::query("UPDATE jobs SET status = ?, result = ?, error = ?, finished_at = ? WHERE id = ?")
        .bind(st)
        .bind(&result)
        .bind(&error)
        .bind(now_ms())
        .bind(job.id)
        .execute(&app.db)
        .await?;
    if job.kind == INSPECT {
        finish_inspect(app, &job, &outcome).await?;
    }
    match &error {
        Some(e) => tracing::warn!("Auftrag {} fehlgeschlagen: {e}", job.id),
        None => tracing::info!("Auftrag {} fertig", job.id),
    }
    notify(app, job.id, job.site_id, st);
    Ok(true)
}

async fn finish_inspect(app: &AppRef, job: &Job, outcome: &Result<Value>) -> Result<()> {
    let p: Value = serde_json::from_str(&job.params)?;
    let id = p["captureId"].as_i64().context("captureId")?;
    let (st, info, err) = match outcome {
        Ok(v) => (status::READY, Some(v.to_string()), None),
        Err(e) => (status::FAILED, None, Some(format!("{e:#}"))),
    };
    sqlx::query("UPDATE captures SET status = ?, info = ?, error = ? WHERE id = ?")
        .bind(st)
        .bind(info)
        .bind(err)
        .bind(id)
        .execute(&app.db)
        .await?;
    app.events.changed(Topic::Captures, Some(job.site_id));
    Ok(())
}

// ---- API ------------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct NewJob {
    kind: String,
    #[serde(default)]
    params: Value,
}

pub async fn create(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(site_id): Path<i64>,
    Json(j): Json<NewJob>,
) -> ApiResult<(StatusCode, Json<Job>)> {
    db::get_site(&app.db, site_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    spec::check(&app, site_id, &j.kind, &j.params).await?;
    let id = enqueue(&app, site_id, &j.kind, &j.params, Some(me.id)).await?;
    Ok((
        StatusCode::CREATED,
        Json(
            db::get_job(&app.db, id)
                .await?
                .ok_or_else(ApiError::not_found)?,
        ),
    ))
}

pub async fn list(
    State(app): State<AppRef>,
    Path(site_id): Path<i64>,
) -> ApiResult<Json<Vec<Job>>> {
    Ok(Json(
        sqlx::query_as("SELECT * FROM jobs WHERE site_id = ? AND kind != ? ORDER BY id DESC")
            .bind(site_id)
            .bind(INSPECT)
            .fetch_all(&app.db)
            .await?,
    ))
}

pub async fn get(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<Json<Job>> {
    Ok(Json(
        db::get_job(&app.db, id)
            .await?
            .ok_or_else(ApiError::not_found)?,
    ))
}

pub async fn delete(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<StatusCode> {
    let job = db::get_job(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    if job.status == status::RUNNING {
        return Err(ApiError::new(StatusCode::CONFLICT, code::INVALID_INPUT).with_detail("läuft"));
    }
    sqlx::query("DELETE FROM jobs WHERE id = ?")
        .bind(id)
        .execute(&app.db)
        .await?;
    let _ = tokio::fs::remove_dir_all(app.cfg.job_dir(id)).await;
    app.events.changed(Topic::Jobs, Some(job.site_id));
    Ok(StatusCode::NO_CONTENT)
}

/// Ergebnisdatei eines Auftrags – nur Namen, die im Ergebnis stehen (kein Pfad von außen).
pub async fn file(
    State(app): State<AppRef>,
    Path((id, name)): Path<(i64, String)>,
) -> ApiResult<Response> {
    let job = db::get_job(&app.db, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let result: Value = job
        .result
        .as_deref()
        .map(serde_json::from_str)
        .transpose()?
        .unwrap_or(Value::Null);
    let listed = result["files"]
        .as_array()
        .is_some_and(|f| f.iter().any(|n| n.as_str() == Some(name.as_str())));
    if !listed {
        return Err(ApiError::not_found());
    }
    let path = app.cfg.job_dir(id).join(&name);
    let f = tokio::fs::File::open(&path)
        .await
        .map_err(|_| ApiError::not_found())?;
    let disposition = format!("attachment; filename=\"auftrag-{id}-{name}\"");
    Ok((
        [
            (header::CONTENT_TYPE, content_type(&name).to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        Body::from_stream(tokio_util::io::ReaderStream::new(f)),
    )
        .into_response())
}

/// Inhaltstyp einer Ergebnisdatei nach Endung
pub fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("") {
        "tif" | "tiff" => "image/tiff",
        "dxf" => "image/vnd.dxf",
        "xml" => "application/xml",
        "csv" => "text/csv; charset=utf-8",
        "obj" => "model/obj",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::content_type;

    #[test]
    fn content_types() {
        assert_eq!(content_type("diff.tif"), "image/tiff");
        assert_eq!(content_type("dgm.xml"), "application/xml");
        assert_eq!(content_type("dgm.dxf"), "image/vnd.dxf");
        assert_eq!(content_type("x"), "application/octet-stream");
    }
}
