//! Fixpunkt-Server: API, Rechenaufträge und die eingebettete Web-UI in einer Programmdatei.

pub mod api;
pub mod auth;
pub mod captures;
pub mod compute;
pub mod config;
pub mod crypto;
pub mod dashboard;
pub mod db;
pub mod error;
pub mod events;
pub mod flightplan;
pub mod jobs;
pub mod layouts;
pub mod location;
pub mod markers;
pub mod odm;
pub mod ortho;
pub mod plans;
pub mod points;
pub mod report;
pub mod sites;
pub mod state;
pub mod ui;
pub mod uploads;
pub mod view;

use anyhow::{Context, Result};
use axum::Router;
use config::Config;
use state::{App, AppRef};
use std::sync::Arc;
use tower_http::compression::predicate::{NotForContentType, Predicate};
use tower_http::compression::{CompressionLayer, DefaultPredicate};

/// Datenverzeichnis anlegen, Datenbank öffnen, unterbrochene Aufträge einreihen, ersten Admin anlegen.
pub async fn init(cfg: Config) -> Result<AppRef> {
    for d in [
        cfg.data_dir.clone(),
        cfg.data_dir.join("captures"),
        cfg.data_dir.join("jobs"),
    ] {
        std::fs::create_dir_all(&d).with_context(|| format!("{} anlegen", d.display()))?;
    }
    let db = db::connect(&cfg.db_path())
        .await
        .context("Datenbank öffnen")?;
    let webauthn = cfg
        .public_url
        .as_deref()
        .map(auth::passkeys::webauthn)
        .transpose()
        .context("FIXPUNKT_PUBLIC_URL")?;
    let app = Arc::new(App {
        db,
        cfg,
        webauthn,
        ceremonies: Default::default(),
        events: Default::default(),
        jobs_wakeup: Default::default(),
        odm_wakeup: Default::default(),
    });
    let n = jobs::requeue_interrupted(&app).await?;
    if n > 0 {
        tracing::info!("{n} unterbrochene Aufträge wieder eingereiht");
    }
    let n = uploads::expire(&app).await?;
    if n > 0 {
        tracing::info!("{n} abgelaufene Teil-Uploads gelöscht");
    }
    auth::ensure_initial_user(&app).await?;
    Ok(app)
}

/// Alle Routen: `/api/…` und die UI für alles andere.
pub fn router(app: AppRef) -> Router {
    let ui = ui::handler(app.cfg.dev_ui.clone());
    Router::new()
        .nest("/api", api::router(app))
        .fallback(ui)
        .layer(
            // gzip/br für UI und JSON; Ereignisse (text/event-stream) und Downloads bleiben ungepuffert
            CompressionLayer::new().compress_when(
                DefaultPredicate::new()
                    .and(NotForContentType::const_new("font/"))
                    .and(NotForContentType::const_new("image/tiff"))
                    .and(NotForContentType::const_new("application/octet-stream")),
                // Raster der Ansicht (application/x-fixpunkt-grid) werden komprimiert: viele leere Zellen
            ),
        )
}

/// Server starten und bis Strg+C / SIGTERM laufen lassen.
pub async fn serve(cfg: Config) -> Result<()> {
    let listen = cfg.listen;
    let app = init(cfg).await?;
    jobs::spawn_workers(&app);
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("{listen} belegen"))?;
    tracing::info!("Fixpunkt läuft auf http://{}", listener.local_addr()?);
    axum::serve(listener, router(app))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    tracing::info!("beendet");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = term => {},
    }
}
