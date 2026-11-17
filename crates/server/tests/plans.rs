//! Pläne über die API: DXF mit Zonenkennung hochladen, Geometrie für die Ansicht, eigene DXF-Höhenlinien
//! aus dem Export als Plan wieder einlesen.

mod common;

use axum::http::{Method, StatusCode};
use common::{terrain_xyz, Client, X0, Y0};
use fixpunkt_server::jobs;
use serde_json::{json, Value};

/// Lageplan der Testgrube (e2e/data/lageplan.dxf): Ostwerte mit Zonenkennung 32
const PLAN: &[u8] = include_bytes!("../../../e2e/data/lageplan.dxf");

#[tokio::test]
async fn dxf_plans_for_the_view() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Pläne"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let r = c
        .upload(
            &format!("/api/sites/{sid}/plans?file=lageplan.dxf"),
            PLAN.to_vec(),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.json());
    let plan = r.json();
    assert_eq!(plan["name"], "lageplan");
    let info = &plan["info"];
    assert_eq!(
        info["layers"],
        json!(["Baufeld", "Baugrube", "Beschriftung", "Hoehen", "Kanal"])
    );
    assert_eq!(info["paths"], 4);
    assert_eq!(info["spots"], 2);
    assert_eq!(info["zoneOffset"], 32_000_000.0);
    assert_eq!(info["bbox"], json!([X0 + 2.0, Y0, X0 + 38.0, Y0 + 40.0]));

    let id = plan["id"].as_i64().unwrap();
    let g: Value = c.get(&format!("/api/plans/{id}/geometry")).await.json();
    let pit = g["paths"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["layer"] == "Baugrube")
        .unwrap();
    assert_eq!(pit["closed"], true);
    assert_eq!(pit["points"][0], json!([X0 + 10.0, Y0 + 10.0]));
    assert_eq!(g["spots"][0]["at"], json!([X0 + 20.0, Y0 + 20.0, 298.2]));
    assert_eq!(g["spots"][1]["at"], json!([X0 + 15.0, Y0 + 15.0, 298.25]));

    // kaputte Dateien und andere Formate
    let bad = c
        .upload(
            &format!("/api/sites/{sid}/plans?file=x.dxf"),
            b"0\nSECTION\nkaputt".to_vec(),
        )
        .await;
    assert_eq!(bad.json()["error"], "plans.invalid");
    let pdf = c
        .upload(
            &format!("/api/sites/{sid}/plans?file=x.pdf"),
            b"%PDF".to_vec(),
        )
        .await;
    assert_eq!(pdf.json()["error"], "plans.invalid");

    assert_eq!(
        c.get(&format!("/api/sites/{sid}/plans"))
            .await
            .json()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        c.delete(&format!("/api/plans/{id}")).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        c.get(&format!("/api/plans/{id}/geometry")).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn exported_contours_read_back_as_plan() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Rundweg"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let cap = c
        .upload(
            &format!("/api/sites/{sid}/captures?file=g.xyz"),
            terrain_xyz(true),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    jobs::run_pending(&c.app).await.unwrap();
    let job = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "export", "params": {"captureId": cap, "formats": ["dxf"], "contourInterval": 0.5}}),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    jobs::run_pending(&c.app).await.unwrap();
    let dxf = c.get(&format!("/api/jobs/{job}/files/dgm.dxf")).await;
    assert_eq!(dxf.status, StatusCode::OK);
    let r = c
        .upload(&format!("/api/sites/{sid}/plans?file=dgm.dxf"), dxf.body)
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.json());
    let info = &r.json()["info"];
    // Höhenlinien von 298,5 bis 300 m um die Grube: es gibt welche, und sie liegen im Gelände
    assert!(info["paths"].as_u64().unwrap() > 0, "{info}");
    assert_eq!(info["zoneOffset"], 0.0);
    let b = info["bbox"].as_array().unwrap();
    assert!(
        b[0].as_f64().unwrap() >= X0 - 1.0 && b[2].as_f64().unwrap() <= X0 + 41.0,
        "{info}"
    );
}
