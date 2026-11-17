//! Routen der API unter `/api`.

use crate::auth::{passkeys, tokens};
use crate::state::AppRef;
use crate::{
    auth, billing, captures, compute, dashboard, flightplan, jobs, layouts, markers, odm, ortho,
    plans, points, report, sites, uploads, view,
};
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
        .route("/me/tokens", get(tokens::list).post(tokens::create))
        .route("/me/tokens/{id}", delete(tokens::delete))
        .route(
            "/me/layouts/{site}",
            get(layouts::get).put(layouts::put).delete(layouts::reset),
        )
        .route("/me/passkeys", get(passkeys::list))
        .route("/me/passkeys/start", post(passkeys::register_start))
        .route("/me/passkeys/finish", post(passkeys::register_finish))
        .route("/me/passkeys/{id}", delete(passkeys::delete))
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
        .route(
            "/regions/{id}",
            delete(sites::delete_region).patch(sites::update_region),
        )
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
        .route("/captures/{id}/points", get(points::capture_points))
        .route(
            "/captures/{id}/ortho",
            get(ortho::image)
                .put(ortho::upload)
                .delete(ortho::delete)
                .layer(DefaultBodyLimit::disable()),
        )
        .route("/jobs/{id}/grid/{name}", get(view::job_grid))
        .route("/sites/{id}/jobs", get(jobs::list).post(jobs::create))
        .route("/jobs/{id}", get(jobs::get).delete(jobs::delete))
        .route("/jobs/{id}/files/{name}", get(jobs::file))
        .route("/jobs/{id}/report", get(report::job_report))
        .route("/odm", get(odm::node_status))
        .route("/sites/{id}/dashboard", get(dashboard::dashboard))
        .route("/regions/{id}/flightplan", get(flightplan::preview))
        .route("/regions/{id}/flightplan.kmz", get(flightplan::kmz))
        .route("/settings", get(settings))
        .route("/georef/fit", post(compute::georef::fit_handler))
        .route("/uploads", post(uploads::create).options(uploads::options))
        .route(
            "/uploads/{id}",
            axum::routing::head(uploads::head)
                .patch(uploads::patch)
                .delete(uploads::delete)
                .layer(DefaultBodyLimit::disable()),
        )
        .route(
            "/sites/{id}/markers",
            get(markers::list).post(markers::create),
        )
        .route(
            "/markers/{id}",
            axum::routing::patch(markers::update).delete(markers::delete),
        )
        .route(
            "/sites/{id}/photos",
            put(markers::upload_photo).layer(DefaultBodyLimit::max(markers::MAX_PHOTO)),
        )
        .route("/markers/{id}/photo", get(markers::photo))
        .route(
            "/sites/{id}/documentation.dxf",
            get(markers::dxf::documentation),
        )
        .route("/markers/{id}/thumb", get(markers::thumb))
        .route(
            "/sites/{id}/plans",
            get(plans::list)
                .put(plans::upload)
                .layer(DefaultBodyLimit::max(plans::MAX_PLAN)),
        )
        .route("/plans/{id}", delete(plans::delete))
        .route("/plans/{id}/geometry", get(plans::geometry))
        .route("/plans/{id}/report", get(report::site_routes::plan))
        .route("/sites/{id}/report", get(report::site_routes::custom))
        .route(
            "/sites/{id}/documentation/report",
            get(report::site_routes::documentation),
        )
        .route(
            "/sites/{id}/billing/settings",
            get(billing::get_settings).put(billing::put_settings),
        )
        .route("/sites/{id}/billing", get(billing::get_statement))
        .route(
            "/sites/{id}/billing/shares",
            get(billing::shares::list).post(billing::shares::create),
        )
        .route("/billing/shares/{id}", delete(billing::shares::revoke))
        .route("/billing/shares/{id}/pdf", get(billing::shares::pdf))
        .route("/sites/{id}/billing.pdf", get(report::site_routes::billing))
        .route(
            "/sites/{id}/deliveries",
            get(billing::deliveries::list).post(billing::deliveries::create),
        )
        .route(
            "/sites/{id}/deliveries.csv",
            put(billing::deliveries::import),
        )
        .route("/deliveries/{id}", delete(billing::deliveries::delete))
        .route("/sites/{id}/odm", post(odm::create))
        .route(
            "/jobs/{id}/images",
            put(odm::upload_image).layer(DefaultBodyLimit::disable()),
        )
        .route("/jobs/{id}/start", post(odm::start))
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
        .route("/auth/passkey/start", post(passkeys::login_start))
        .route("/auth/passkey/finish", post(passkeys::login_finish))
        // Freigabe eines Aufmaßes: ohne Anmeldung, nur mit dem Token aus dem Link
        .route("/public/shares/{token}", get(billing::shares::public_get))
        .route(
            "/public/shares/{token}/decision",
            post(billing::shares::public_decide),
        )
        .route(
            "/public/shares/{token}/pdf",
            get(billing::shares::public_pdf),
        )
        .merge(protected)
        .with_state(app)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

/// Einstellungen, die die UI braucht: Kacheln der Übersichtskarte
async fn settings(State(app): State<AppRef>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "mapTiles": app.cfg.map_tiles.as_ref().map(|(url, attribution)| {
            serde_json::json!({ "url": url, "attribution": attribution })
        }),
    }))
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
