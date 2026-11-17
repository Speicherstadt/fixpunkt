//! Routen der API unter `/api`.

use crate::state::AppRef;
use crate::{auth, captures, jobs, report, sites, view};
use axum::extract::{DefaultBodyLimit, State};
use axum::middleware;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use futures::Stream;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

pub fn router(app: AppRef) -> Router {
    let protected = Router::new()
        .route("/events", get(events))
        .route("/auth/password", post(auth::change_password))
        .route("/users", get(auth::list_users).post(auth::add_user))
        .route("/users/{id}", delete(auth::delete_user))
        .route("/sites", get(sites::list).post(sites::create))
        .route(
            "/sites/{id}",
            get(sites::get).patch(sites::update).delete(sites::delete),
        )
        .route(
            "/sites/{id}/regions",
            get(sites::list_regions).post(sites::create_regions),
        )
        .route("/regions/{id}", delete(sites::delete_region))
        .route("/sites/{id}/captures", get(captures::list))
        .route(
            "/sites/{id}/captures",
            put(captures::upload).layer(DefaultBodyLimit::disable()),
        )
        .route(
            "/captures/{id}",
            axum::routing::patch(captures::update).delete(captures::delete),
        )
        .route("/captures/{id}/file", get(captures::download))
        .route("/captures/{id}/grid", get(view::capture_grid))
        .route("/jobs/{id}/grid/{name}", get(view::job_grid))
        .route("/sites/{id}/jobs", get(jobs::list).post(jobs::create))
        .route("/jobs/{id}", get(jobs::get).delete(jobs::delete))
        .route("/jobs/{id}/files/{name}", get(jobs::file))
        .route("/jobs/{id}/report", get(report::job_report))
        .route_layer(middleware::from_fn_with_state(
            app.clone(),
            auth::require_auth,
        ));

    Router::new()
        .route("/health", get(health))
        .route("/auth/state", get(auth::state))
        .route("/auth/setup", post(auth::setup))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .merge(protected)
        .with_state(app)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

async fn events(
    State(app): State<AppRef>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let stream = BroadcastStream::new(app.events.subscribe()).map(|e| {
        Ok(match e {
            Ok(ev) => SseEvent::default().json_data(ev).unwrap_or_default(),
            // zu langsam gelesen: die UI lädt alles neu
            Err(_) => SseEvent::default().event("lagged").data(""),
        })
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
