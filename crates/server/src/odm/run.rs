//! Photogrammetrie-Auftrag im Worker: Fotos zu NodeODM, Fortschritt abfragen, Punktwolke holen und als
//! Aufnahme anlegen. Der Stand liegt in `jobs/<id>/nodeodm.json`: nach einem Neustart fragt der Auftrag
//! dieselbe NodeODM-Aufgabe weiter ab, statt die Fotos neu hochzuladen.

use super::client::{NodeOdm, TaskStatus};
use super::{cloud, images_dir, list_images, options, Params, MIN_IMAGES};
use crate::captures;
use crate::db::{self, Job};
use crate::events::Topic;
use crate::state::AppRef;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Fotos je Upload-Anfrage
const BATCH: usize = 20;
/// Anteile am Fortschritt: Hochladen bis 10 %, Rechnen in NodeODM bis 95 %, dann Punktwolke holen
const UPLOAD_SHARE: f64 = 10.0;
const PROCESS_SHARE: f64 = 85.0;
/// Ergebnisdatei von ODM: georeferenzierte Punktwolke
const ASSET: &str = "georeferenced_model.laz";

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    uuid: Option<String>,
    /// alle Fotos hochgeladen und die Aufgabe gestartet
    committed: bool,
}

fn state_path(dir: &Path) -> PathBuf {
    dir.join("nodeodm.json")
}

fn load(dir: &Path) -> State {
    std::fs::read(state_path(dir))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save(dir: &Path, s: &State) -> Result<()> {
    Ok(std::fs::write(state_path(dir), serde_json::to_vec(s)?)?)
}

/// Fortschritt speichern und melden (nur bei ganzen Prozentschritten)
async fn progress(app: &AppRef, job: &Job, last: &mut f64, p: f64) -> Result<()> {
    if (p - *last).abs() < 1.0 && p < 100.0 {
        return Ok(());
    }
    *last = p;
    sqlx::query("UPDATE jobs SET progress = ? WHERE id = ?")
        .bind(p)
        .bind(job.id)
        .execute(&app.db)
        .await?;
    app.events.changed(Topic::Jobs, Some(job.site_id));
    Ok(())
}

/// Neue Aufgabe in NodeODM: anlegen, Fotos in Paketen hochladen, starten
async fn submit(
    app: &AppRef,
    job: &Job,
    odm: &NodeOdm,
    p: &Params,
    images: &[PathBuf],
    last: &mut f64,
) -> Result<String> {
    let name = format!("Fixpunkt {} – {}", job.id, p.name);
    let uuid = odm.init(&name, &options(p.quality)).await?;
    let dir = app.cfg.job_dir(job.id);
    save(
        &dir,
        &State {
            uuid: Some(uuid.clone()),
            committed: false,
        },
    )?;
    for (i, batch) in images.chunks(BATCH).enumerate() {
        odm.upload(&uuid, batch).await?;
        let done = ((i + 1) * BATCH).min(images.len()) as f64 / images.len() as f64;
        progress(app, job, last, UPLOAD_SHARE * done).await?;
    }
    odm.commit(&uuid).await?;
    save(
        &dir,
        &State {
            uuid: Some(uuid.clone()),
            committed: true,
        },
    )?;
    Ok(uuid)
}

/// Warten, bis NodeODM fertig ist; Fehler mit den letzten Zeilen der Ausgabe
async fn wait(app: &AppRef, job: &Job, odm: &NodeOdm, uuid: &str, last: &mut f64) -> Result<i64> {
    loop {
        let info = odm.task_info(uuid).await?;
        progress(
            app,
            job,
            last,
            UPLOAD_SHARE + PROCESS_SHARE * info.progress / 100.0,
        )
        .await?;
        match info.status {
            TaskStatus::Completed => return Ok(info.processing_ms),
            TaskStatus::Failed | TaskStatus::Canceled => {
                let tail = odm.output_tail(uuid, 5).await.unwrap_or_default();
                let reason = info.error.unwrap_or_else(|| "abgebrochen".into());
                odm.remove(uuid).await;
                bail!("NodeODM: {reason}\n{}", tail.join("\n"));
            }
            TaskStatus::Queued | TaskStatus::Running => {
                tokio::time::sleep(app.cfg.nodeodm_poll).await;
            }
        }
    }
}

pub async fn run(app: &AppRef, job: &Job) -> Result<Value> {
    let p: Params = serde_json::from_str(&job.params)?;
    let url = app
        .cfg
        .nodeodm_url
        .as_deref()
        .context("NodeODM ist nicht eingerichtet (FIXPUNKT_NODEODM_URL)")?;
    let odm = NodeOdm::new(url, app.cfg.nodeodm_token.clone())?;
    let site = db::get_site(&app.db, job.site_id)
        .await?
        .context("Baustelle gelöscht")?;
    let dir = app.cfg.job_dir(job.id);
    let images = list_images(&images_dir(app, job.id))?;
    if images.len() < MIN_IMAGES {
        bail!(
            "zu wenige Fotos ({}), mindestens {MIN_IMAGES}",
            images.len()
        );
    }
    let mut last = -1.0;
    let state = load(&dir);
    let uuid = match state {
        State {
            uuid: Some(u),
            committed: true,
        } => u,
        State { uuid, .. } => {
            // halb hochgeladen: neu anfangen
            if let Some(old) = uuid {
                odm.remove(&old).await;
            }
            submit(app, job, &odm, &p, &images, &mut last).await?
        }
    };
    let processing_ms = wait(app, job, &odm, &uuid, &mut last).await?;

    let raw = dir.join("odm.laz");
    odm.download(&uuid, ASSET, &raw).await?;
    progress(app, job, &mut last, 97.0).await?;
    let file_name = format!("drohne-{}.laz", job.id);
    let staged = dir.join(&file_name);
    let (src, dest, offset) = (raw.clone(), staged.clone(), p.height_offset);
    let epsg = u32::try_from(site.epsg).unwrap_or(25832);
    let prepared =
        tokio::task::spawn_blocking(move || cloud::prepare(&src, &dest, epsg, offset)).await??;

    let capture_id = captures::insert(app, job.site_id, &p.name, &file_name, p.captured_at).await?;
    let target = app.cfg.capture_dir(capture_id);
    tokio::fs::create_dir_all(&target).await?;
    tokio::fs::rename(&staged, target.join(&file_name)).await?;
    let size = tokio::fs::metadata(target.join(&file_name)).await?.len();
    captures::finish(app, capture_id, job.site_id, size, job.created_by).await?;
    // NodeODM-Speicher freigeben; die Fotos bleiben im Auftrag, bis er gelöscht wird
    odm.remove(&uuid).await;
    progress(app, job, &mut last, 100.0).await?;
    Ok(json!({
        "captureId": capture_id,
        "images": images.len(),
        "processingMs": processing_ms,
        "sourceEpsg": prepared.source_epsg,
        "converted": prepared.converted,
        "heightOffset": p.height_offset,
    }))
}
