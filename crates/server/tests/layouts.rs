//! Eigener Überblick je Benutzer und Baustelle.

mod common;

use axum::http::{Method, StatusCode};
use common::Client;
use serde_json::json;

#[tokio::test]
async fn layouts_per_user_and_site() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(
            Method::POST,
            "/api/sites",
            json!({"name": "Eigener Überblick"}),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let url = format!("/api/me/layouts/{sid}");

    // Grundeinstellung: alle Abschnitte sichtbar, feste Reihenfolge
    let d = c.get(&url).await.json();
    assert_eq!(d["custom"], false);
    let ids: Vec<&str> = d["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "kpis",
            "progress",
            "targets",
            "stock",
            "history",
            "attention"
        ]
    );

    // ein Leser ordnet seinen eigenen Überblick
    c.json(
        Method::POST,
        "/api/users",
        json!({"name": "leser", "password": "lesen1234", "role": "viewer"}),
    )
    .await;
    let mut viewer = c.other();
    viewer.login("leser", "lesen1234").await;
    let saved = viewer
        .json(
            Method::PUT,
            &url,
            json!({"sections": [{"id": "stock", "visible": true}, {"id": "kpis", "visible": false}]}),
        )
        .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.json());
    let v = viewer.get(&url).await.json();
    assert_eq!(v["custom"], true);
    assert_eq!(v["sections"][0], json!({"id": "stock", "visible": true}));
    assert_eq!(v["sections"][1], json!({"id": "kpis", "visible": false}));
    assert_eq!(
        v["sections"].as_array().unwrap().len(),
        6,
        "fehlende hinten angehängt"
    );

    // der Admin sieht weiter seine Grundeinstellung
    assert_eq!(c.get(&url).await.json()["custom"], false);

    // zurücksetzen
    assert_eq!(viewer.delete(&url).await.status, StatusCode::NO_CONTENT);
    assert_eq!(viewer.get(&url).await.json()["custom"], false);

    assert_eq!(
        c.get("/api/me/layouts/999").await.status,
        StatusCode::NOT_FOUND
    );
    // mit der Baustelle verschwindet der gespeicherte Überblick
    viewer
        .json(Method::PUT, &url, json!({"sections": []}))
        .await;
    assert_eq!(
        c.delete(&format!("/api/sites/{sid}")).await.status,
        StatusCode::NO_CONTENT
    );
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_layouts")
        .fetch_one(&c.app.db)
        .await
        .unwrap();
    assert_eq!(left, 0);
}
