//! Flugplanung über die API: Vorschau je Bereich, Höhenunterschied aus der letzten Aufnahme, KMZ für Pilot 2.

mod common;

use axum::http::{header, Method, StatusCode};
use common::{rect, terrain_xyz, Client};
use fixpunkt_server::jobs;
use serde_json::json;

#[tokio::test]
async fn flight_plan_for_a_region() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Baugrube Süd"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let fc = json!({"type": "FeatureCollection",
        "crs": {"type": "name", "properties": {"name": "urn:ogc:def:crs:EPSG::25832"}},
        "features": [rect("Feld", 0.0, 0.0, 40.0, 40.0)]});
    let rid = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/regions"),
            json!({"geojson": fc}),
        )
        .await
        .json()[0]["id"]
        .as_i64()
        .unwrap();

    // ohne Aufnahme: Plan ohne Gelände. 2 cm GSD, M4E: 75,02 m Höhe, 31,68 m Bahnabstand
    // → ⌈40 / 31,68⌉ + 1 = 3 Bahnen à 40 + 2 · 15,824 m
    let r = c.get(&format!("/api/regions/{rid}/flightplan")).await;
    assert_eq!(r.status, StatusCode::OK, "{:?}", r.json());
    let v = r.json();
    assert_eq!(v["camera"], "DJI Matrice 4E");
    assert_eq!(v["terrain"], json!(null));
    let p = &v["plan"];
    assert!((p["altitude"].as_f64().unwrap() - 75.02).abs() < 0.01);
    assert_eq!(p["lines"].as_array().unwrap().len(), 3);
    assert!((p["length"].as_f64().unwrap() - (3.0 * 71.648 + 2.0 * 31.68)).abs() < 1e-6);
    assert_eq!(p["photos"], 3 * 5);

    // 1 cm, Ost–West: halbe Höhe (37,51 m), Bahnabstand 15,84 m → ⌈40 / 15,84⌉ + 1 = 4 Bahnen
    let fine = c
        .get(&format!(
            "/api/regions/{rid}/flightplan?gsd=1&direction=90&front=80&side=70"
        ))
        .await
        .json();
    assert!((fine["plan"]["altitude"].as_f64().unwrap() - 37.51).abs() < 0.01);
    assert_eq!(fine["plan"]["lines"].as_array().unwrap().len(), 4);
    let l0 = &fine["plan"]["lines"][0];
    assert!((l0["start"][1].as_f64().unwrap() - l0["end"][1].as_f64().unwrap()).abs() < 1e-6);

    for bad in ["gsd=50", "front=99", "drone=phantom"] {
        let r = c.get(&format!("/api/regions/{rid}/flightplan?{bad}")).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{bad}");
        assert_eq!(r.json()["error"], "api.invalidInput");
    }

    // mit Aufnahme: Grubensohle 298,1…298,3 m, Rand bis 300,4 m
    c.upload(
        &format!("/api/sites/{sid}/captures?file=aushub.xyz&capturedAt=1"),
        terrain_xyz(true),
    )
    .await;
    jobs::run_pending(&c.app).await.unwrap();
    let t = c
        .get(&format!("/api/regions/{rid}/flightplan"))
        .await
        .json()["terrain"]
        .clone();
    assert_eq!(t["capture"], "aushub.xyz");
    assert!((t["min"].as_f64().unwrap() - 298.1).abs() < 0.05, "{t}");
    assert!((t["max"].as_f64().unwrap() - 300.4).abs() < 0.05, "{t}");

    // KMZ: ZIP mit beiden WPML-Dateien, Name aus Baustelle und Bereich
    let k = c
        .get(&format!("/api/regions/{rid}/flightplan.kmz?drone=m3e"))
        .await;
    assert_eq!(k.status, StatusCode::OK);
    assert_eq!(
        k.headers[header::CONTENT_TYPE],
        "application/vnd.google-earth.kmz"
    );
    assert_eq!(
        k.headers[header::CONTENT_DISPOSITION],
        format!("attachment; filename=\"flugplan-Baugrube-Sued-Feld-{rid}.kmz\"")
    );
    assert!(k.body.starts_with(b"PK\x03\x04"));
    let has = |n: &[u8]| k.body.windows(n.len()).any(|w| w == n);
    assert!(has(b"wpmz/template.kml") && has(b"wpmz/waylines.wpml"));

    assert_eq!(
        c.get("/api/regions/999/flightplan").await.status,
        StatusCode::NOT_FOUND
    );
}
