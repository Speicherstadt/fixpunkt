//! Fortsetzbarer Upload nach tus 1.0 (<https://tus.io/protocols/resumable-upload>): Kern, `creation` und
//! `termination`. Große Dateien (Aufnahmen, Drohnenfotos, Orthofotos) kommen in Stücken; reißt die Verbindung
//! ab, fragt der Client mit HEAD nach, wie viel angekommen ist, und schickt den Rest. Ist alles da, geht die
//! Datei an ihr Ziel (`Target`), genau wie bei einem Upload in einem Stück.
//!
//! Teil-Uploads liegen unter `uploads/<id>.part` mit `uploads/<id>.json`; nach [`EXPIRY_MS`] räumt der Server
//! sie beim Start ab.

use crate::auth::CurrentUser;
use crate::captures::safe_file_name;
use crate::db::now_ms;
use crate::error::{code, ApiError, ApiResult};
use crate::state::AppRef;
use crate::{captures, compute, odm, ortho};
use anyhow::{bail, Context, Result};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use base64::Engine;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

pub const TUS_VERSION: &str = "1.0.0";
/// Unfertige Uploads werden nach einer Woche verworfen
pub const EXPIRY_MS: i64 = 7 * 24 * 3600 * 1000;

/// Wohin die fertige Datei geht (aus `Upload-Metadata`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "lowercase")]
pub enum Target {
    /// neue Aufnahme einer Baustelle
    Capture {
        site_id: i64,
        name: String,
        captured_at: Option<i64>,
    },
    /// Foto für einen Photogrammetrie-Entwurf
    Image { job_id: i64 },
    /// Orthofoto einer Aufnahme
    Ortho { capture_id: i64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct State_ {
    user: i64,
    file: String,
    length: u64,
    offset: u64,
    created_at: i64,
    target: Target,
}

/// `Upload-Metadata`: `schluessel base64,schluessel2 base64` → Klartext
pub fn parse_metadata(header: &str) -> Result<HashMap<String, String>> {
    let b64 = base64::engine::general_purpose::STANDARD;
    header
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|pair| {
            let (k, v) = pair.split_once(' ').unwrap_or((pair, ""));
            let value = String::from_utf8(b64.decode(v.trim())?)?;
            Ok((k.to_string(), value))
        })
        .collect()
}

/// Ziel aus den Metadaten
pub fn target_of(meta: &HashMap<String, String>) -> Result<Target> {
    let num = |k: &str| -> Result<i64> {
        meta.get(k)
            .with_context(|| format!("{k} fehlt"))?
            .parse()
            .with_context(|| format!("{k} ist keine Zahl"))
    };
    Ok(match meta.get("target").map(String::as_str) {
        Some("capture") => Target::Capture {
            site_id: num("siteId")?,
            name: meta.get("name").cloned().unwrap_or_default(),
            captured_at: num("capturedAt").ok(),
        },
        Some("image") => Target::Image {
            job_id: num("jobId")?,
        },
        Some("ortho") => Target::Ortho {
            capture_id: num("captureId")?,
        },
        other => bail!("unbekanntes Ziel {other:?}"),
    })
}

fn dir(app: &AppRef) -> PathBuf {
    app.cfg.data_dir.join("uploads")
}

fn part(app: &AppRef, id: &str) -> PathBuf {
    dir(app).join(format!("{id}.part"))
}

fn state_path(app: &AppRef, id: &str) -> PathBuf {
    dir(app).join(format!("{id}.json"))
}

/// Nur Hex-IDs, die der Server selbst vergeben hat (kein Pfad über die URL)
fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit())
}

async fn load(app: &AppRef, id: &str, user: i64) -> ApiResult<State_> {
    if !valid_id(id) {
        return Err(ApiError::not_found());
    }
    let bytes = tokio::fs::read(state_path(app, id))
        .await
        .map_err(|_| ApiError::not_found())?;
    let s: State_ = serde_json::from_slice(&bytes).map_err(anyhow::Error::from)?;
    if s.user != user {
        return Err(ApiError::not_found());
    }
    Ok(s)
}

async fn save(app: &AppRef, id: &str, s: &State_) -> Result<()> {
    Ok(tokio::fs::write(state_path(app, id), serde_json::to_vec(s)?).await?)
}

fn tus_headers(h: &mut HeaderMap) {
    h.insert("Tus-Resumable", HeaderValue::from_static(TUS_VERSION));
}

/// `OPTIONS /api/uploads`: was der Server kann
pub async fn options(State(app): State<AppRef>) -> Response {
    let mut h = HeaderMap::new();
    tus_headers(&mut h);
    h.insert("Tus-Version", HeaderValue::from_static(TUS_VERSION));
    h.insert(
        "Tus-Extension",
        HeaderValue::from_static("creation,termination,expiration"),
    );
    h.insert("Tus-Max-Size", HeaderValue::from(app.cfg.max_upload));
    (StatusCode::NO_CONTENT, h).into_response()
}

/// Prüft, ob der Nutzer an das Ziel hochladen darf, bevor ein Byte kommt
async fn check_target(app: &AppRef, t: &Target, file: &str) -> ApiResult<()> {
    match t {
        Target::Capture { site_id, .. } => {
            crate::db::get_site(&app.db, *site_id)
                .await?
                .ok_or_else(ApiError::not_found)?;
            if !compute::is_supported(file) {
                return Err(ApiError::bad_request(code::UNSUPPORTED_FILE));
            }
        }
        Target::Image { job_id } => {
            odm::draft_job(app, *job_id).await?;
            if !odm::is_image(file) {
                return Err(ApiError::bad_request(code::ODM_UNSUPPORTED_IMAGE));
            }
        }
        Target::Ortho { capture_id } => {
            crate::db::get_capture(&app.db, *capture_id)
                .await?
                .ok_or_else(ApiError::not_found)?;
            let l = file.to_ascii_lowercase();
            if !(l.ends_with(".tif") || l.ends_with(".tiff")) {
                return Err(ApiError::bad_request(code::ORTHO_INVALID).with_detail("nur GeoTIFF"));
            }
        }
    }
    Ok(())
}

/// `POST /api/uploads` mit `Upload-Length` und `Upload-Metadata` (filename, target, …)
pub async fn create(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let header = |k: &str| headers.get(k).and_then(|v| v.to_str().ok());
    let length: u64 = header("Upload-Length")
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| ApiError::bad_request(code::INVALID_INPUT).with_detail("Upload-Length"))?;
    if length > app.cfg.max_upload {
        return Err(ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            code::FILE_TOO_LARGE,
        ));
    }
    let invalid =
        |e: anyhow::Error| ApiError::bad_request(code::INVALID_INPUT).with_detail(format!("{e:#}"));
    let meta = parse_metadata(header("Upload-Metadata").unwrap_or("")).map_err(invalid)?;
    let target = target_of(&meta).map_err(invalid)?;
    let file = meta
        .get("filename")
        .and_then(|f| safe_file_name(f))
        .ok_or_else(|| ApiError::bad_request(code::INVALID_INPUT).with_detail("filename"))?;
    check_target(&app, &target, &file).await?;
    let id = hex::encode(rand::random::<[u8; 16]>());
    tokio::fs::create_dir_all(dir(&app)).await?;
    tokio::fs::File::create(part(&app, &id)).await?;
    let s = State_ {
        user: me.id,
        file,
        length,
        offset: 0,
        created_at: now_ms(),
        target,
    };
    save(&app, &id, &s).await?;
    let mut h = HeaderMap::new();
    tus_headers(&mut h);
    h.insert(
        "Location",
        HeaderValue::from_str(&format!("/api/uploads/{id}")).map_err(anyhow::Error::from)?,
    );
    h.insert("Upload-Offset", HeaderValue::from(0u64));
    Ok((StatusCode::CREATED, h).into_response())
}

/// `HEAD /api/uploads/{id}`: wie viel schon da ist
pub async fn head(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let s = load(&app, &id, me.id).await?;
    let mut h = HeaderMap::new();
    tus_headers(&mut h);
    h.insert("Upload-Offset", HeaderValue::from(s.offset));
    h.insert("Upload-Length", HeaderValue::from(s.length));
    h.insert("Cache-Control", HeaderValue::from_static("no-store"));
    Ok((StatusCode::OK, h).into_response())
}

/// `PATCH /api/uploads/{id}` mit `Upload-Offset`: Stück anhängen; ist die Datei komplett, ans Ziel geben
pub async fn patch(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Body,
) -> ApiResult<Response> {
    let mut s = load(&app, &id, me.id).await?;
    let ct = headers
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if ct != "application/offset+octet-stream" {
        return Err(ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            code::INVALID_INPUT,
        ));
    }
    let offset: Option<u64> = headers
        .get("Upload-Offset")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());
    if offset != Some(s.offset) {
        return Err(
            ApiError::new(StatusCode::CONFLICT, code::INVALID_INPUT).with_detail("Upload-Offset")
        );
    }
    let mut f = tokio::fs::OpenOptions::new()
        .append(true)
        .open(part(&app, &id))
        .await?;
    let mut stream = body.into_data_stream();
    let mut failed = None;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                if s.offset + bytes.len() as u64 > s.length {
                    failed =
                        Some(ApiError::bad_request(code::INVALID_INPUT).with_detail("zu lang"));
                    break;
                }
                f.write_all(&bytes).await?;
                s.offset += bytes.len() as u64;
            }
            // Verbindung abgerissen: was angekommen ist, bleibt; der Client fragt mit HEAD nach
            Err(e) => {
                failed =
                    Some(ApiError::bad_request(code::INVALID_INPUT).with_detail(e.to_string()));
                break;
            }
        }
    }
    f.flush().await?;
    drop(f);
    save(&app, &id, &s).await?;
    if let Some(e) = failed {
        return Err(e);
    }
    if s.offset == s.length {
        finish(&app, &id, &s).await?;
    }
    let mut h = HeaderMap::new();
    tus_headers(&mut h);
    h.insert("Upload-Offset", HeaderValue::from(s.offset));
    Ok((StatusCode::NO_CONTENT, h).into_response())
}

/// Komplette Datei an ihr Ziel; der Teil-Upload verschwindet
async fn finish(app: &AppRef, id: &str, s: &State_) -> ApiResult<()> {
    let src = part(app, id);
    let result = match &s.target {
        Target::Capture {
            site_id,
            name,
            captured_at,
        } => {
            let name = if name.trim().is_empty() {
                s.file.clone()
            } else {
                name.trim().to_string()
            };
            captures::create_from_file(
                app,
                *site_id,
                &name,
                &s.file,
                *captured_at,
                Some(s.user),
                &src,
            )
            .await
            .map(|_| ())
        }
        Target::Image { job_id } => odm::accept_image(app, *job_id, &s.file, &src).await,
        Target::Ortho { capture_id } => ortho::accept(app, *capture_id, &src).await,
    };
    // die Datei ist ans Ziel verschoben oder unbrauchbar: der Teil-Upload ist erledigt
    let _ = tokio::fs::remove_file(&src).await;
    let _ = tokio::fs::remove_file(state_path(app, id)).await;
    result
}

/// `DELETE /api/uploads/{id}`: abbrechen
pub async fn delete(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    load(&app, &id, me.id).await?;
    let _ = tokio::fs::remove_file(part(&app, &id)).await;
    let _ = tokio::fs::remove_file(state_path(&app, &id)).await;
    let mut h = HeaderMap::new();
    tus_headers(&mut h);
    Ok((StatusCode::NO_CONTENT, h).into_response())
}

/// Beim Start: unfertige Uploads älter als [`EXPIRY_MS`] löschen; gibt die Zahl zurück
pub async fn expire(app: &AppRef) -> Result<usize> {
    let mut n = 0;
    let Ok(mut rd) = tokio::fs::read_dir(dir(app)).await else {
        return Ok(0);
    };
    while let Some(e) = rd.next_entry().await? {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let old = match tokio::fs::read(&p)
            .await
            .ok()
            .and_then(|b| serde_json::from_slice::<State_>(&b).ok())
        {
            Some(s) => now_ms() - s.created_at > EXPIRY_MS,
            None => true,
        };
        if old {
            let _ = tokio::fs::remove_file(p.with_extension("part")).await;
            tokio::fs::remove_file(&p).await?;
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_and_targets() {
        // „filename flug.laz,target capture,siteId 7,name Aushub Süd“ in Base64 wie tus-js-client
        let m = parse_metadata(
            "filename Zmx1Zy5sYXo=,target Y2FwdHVyZQ==,siteId Nw==,name QXVzaHViIFPDvGQ=",
        )
        .unwrap();
        assert_eq!(m["filename"], "flug.laz");
        assert_eq!(m["name"], "Aushub Süd");
        assert_eq!(
            target_of(&m).unwrap(),
            Target::Capture {
                site_id: 7,
                name: "Aushub Süd".into(),
                captured_at: None
            }
        );
        let img = parse_metadata("target aW1hZ2U=,jobId NDI=").unwrap();
        assert_eq!(target_of(&img).unwrap(), Target::Image { job_id: 42 });
        assert!(target_of(&parse_metadata("target Zm9v").unwrap()).is_err());
        assert!(parse_metadata("filename !!!").is_err());
        assert!(parse_metadata("").unwrap().is_empty());
    }

    #[test]
    fn ids_cannot_escape() {
        assert!(valid_id("0123456789abcdef0123456789abcdef"));
        assert!(!valid_id("../../fixpunkt.db"));
        assert!(!valid_id("0123456789abcdef0123456789abcdeg"));
    }
}
