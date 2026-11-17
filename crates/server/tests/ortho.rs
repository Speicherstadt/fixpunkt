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

/// Orthofoto als GeoTIFF exportieren, auf einen Bereich um den Bagger zugeschnitten (Pixel 330…380 / 350…380 =
/// Ost +33…38 m, Nord +2…5 m): Ausschnitt Ost +32…39, Nord +1…6 → 70 × 50 Pixel à 0,1 m
#[tokio::test]
async fn orthophoto_export_clipped_to_a_region() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Ortho-Export"}))
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
    let export = |regions: Vec<i64>| json!({"kind": "export", "params": {"captureId": cap, "formats": ["ortho"], "regionIds": regions}});
    // ohne Orthofoto schlägt der Auftrag fehl, mit Grund
    let j = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            export(vec![]),
        )
        .await
        .json();
    jobs::run_pending(&c.app).await.unwrap();
    let failed = c.get(&format!("/api/jobs/{}", j["id"])).await.json();
    assert_eq!(failed["status"], "failed");
    assert!(
        failed["error"].as_str().unwrap().contains("kein Orthofoto"),
        "{failed}"
    );

    c.upload(
        &format!("/api/captures/{cap}/ortho?file=o.tif"),
        ORTHO.to_vec(),
    )
    .await;
    let fc = json!({"type": "FeatureCollection",
        "crs": {"type": "name", "properties": {"name": "urn:ogc:def:crs:EPSG::25832"}},
        "features": [common::rect("Bagger", 32.0, 1.0, 39.0, 6.0)]});
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
    let j = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            export(vec![rid]),
        )
        .await
        .json();
    jobs::run_pending(&c.app).await.unwrap();
    let done = c.get(&format!("/api/jobs/{}", j["id"])).await.json();
    assert_eq!(done["status"], "done", "{done}");
    assert_eq!(done["result"]["files"], json!(["orthofoto.tif"]));

    let tif = c
        .get(&format!("/api/jobs/{}/files/orthofoto.tif", j["id"]))
        .await;
    assert_eq!(tif.status, StatusCode::OK);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("o.tif");
    std::fs::write(&path, &tif.body).unwrap();
    // unabhängig gelesen: Lage, EPSG, Größe, Bagger gelb an seiner Stelle
    let o = fixpunkt::ortho::read_downsampled(&path, 8192).unwrap();
    assert_eq!((o.width, o.height), (70, 50));
    assert_eq!(o.georef.epsg, Some(25832));
    assert!(
        (o.georef.min_x - (X0 + 32.0)).abs() < 1e-6 && (o.georef.max_y - (Y0 + 6.0)).abs() < 1e-6
    );
    assert!((o.georef.cell - 0.1).abs() < 1e-9);
    // Ost +35,5 / Nord +3,5 → Spalte 35, Zeile 25
    let px = |col: u32, row: u32| {
        let i = ((row * o.width + col) * 4) as usize;
        [o.rgba[i], o.rgba[i + 1], o.rgba[i + 2], o.rgba[i + 3]]
    };
    assert_eq!(px(35, 25), [230, 180, 30, 255]);
}
