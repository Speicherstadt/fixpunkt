//! Orthofoto an einer Aufnahme: GeoTIFF hochladen (geschrieben mit Pillow, e2e/data/ortho.tif), Ecken im
//! System der Baustelle, Bild für die Ansicht.

mod common;

use axum::http::{Method, StatusCode};
use common::{terrain_xyz, Client, X0, Y0};
use fixpunkt_server::jobs;
use serde_json::json;

/// 400 × 400 Pixel à 0,1 m ab (X0 | Y0 + 40), RGBA; Bagger gelb bei Pixel 330…380 / 350…380, unterster Rand leer
const ORTHO: &[u8] = include_bytes!("../../../e2e/data/ortho.tif");

#[tokio::test]
async fn orthophoto_for_a_capture() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Ortho"}))
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

    let r = c
        .upload(
            &format!("/api/captures/{cap}/ortho?file=orthophoto.tif"),
            ORTHO.to_vec(),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json());
    let o = &r.json()["ortho"];
    assert_eq!(
        (o["width"].as_u64(), o["height"].as_u64()),
        (Some(400), Some(400))
    );
    assert_eq!(
        o["corners"],
        json!([[X0, Y0 + 40.0], [X0 + 40.0, Y0 + 40.0], [X0, Y0]])
    );
    assert_eq!(o["sourceEpsg"], 25832);

    let png = c.get(&format!("/api/captures/{cap}/ortho")).await;
    assert_eq!(png.headers["content-type"], "image/png");
    let img = image::load_from_memory(&png.body).unwrap().to_rgba8();
    assert_eq!(img.dimensions(), (400, 400));
    assert_eq!(img.get_pixel(355, 365).0, [230, 180, 30, 255], "Bagger");
    assert_eq!(img.get_pixel(10, 395).0[3], 0, "Rand ohne Daten");

    // nur GeoTIFF, und nur lesbare
    let bad = c
        .upload(
            &format!("/api/captures/{cap}/ortho?file=foto.jpg"),
            vec![1, 2, 3],
        )
        .await;
    assert_eq!(bad.json()["error"], "ortho.invalid");
    let broken = c
        .upload(
            &format!("/api/captures/{cap}/ortho?file=x.tif"),
            b"II*\0kaputt".to_vec(),
        )
        .await;
    assert_eq!(broken.json()["error"], "ortho.invalid");

    assert_eq!(
        c.delete(&format!("/api/captures/{cap}/ortho")).await.status,
        StatusCode::NO_CONTENT
    );
    let caps = c.get(&format!("/api/sites/{sid}/captures")).await.json();
    assert!(caps[0]["ortho"].is_null());
    assert_eq!(
        c.get(&format!("/api/captures/{cap}/ortho")).await.status,
        StatusCode::NOT_FOUND
    );
}
