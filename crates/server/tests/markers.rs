//! Dokumentation über die API: Zwangspunkte, Notizen, Fotos mit Position aus EXIF.

mod common;

use axum::http::{Method, StatusCode};
use common::Client;
use serde_json::json;

/// Foto mit GPS an der Stelle E 512 025, N 5 403 015 (siehe markers/photo.rs)
const PHOTO: &[u8] = include_bytes!("../../../e2e/data/foto-gps.jpg");

#[tokio::test]
async fn constraint_points_notes_and_photos() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Doku"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let url = format!("/api/sites/{sid}/markers");

    // Zwangspunkt braucht Lage und Sollhöhe
    let r = c
        .json(
            Method::POST,
            &url,
            json!({"kind": "constraint", "title": "Kanal", "x": 512010.0, "y": 5403010.0}),
        )
        .await;
    assert_eq!(r.json()["error"], "markers.needsPosition");
    let k = c
        .json(
            Method::POST,
            &url,
            json!({"kind": "constraint", "title": "Kanalsohle Bestand", "note": "DN 300, nicht anfahren",
                   "x": 512010.0, "y": 5403010.0, "z": 298.05}),
        )
        .await;
    assert_eq!(k.status, StatusCode::CREATED, "{}", k.json());
    let note = c
        .json(
            Method::POST,
            &url,
            json!({"kind": "note", "title": "Bodenprobe genommen"}),
        )
        .await
        .json();
    assert!(note["x"].is_null());

    // Foto: Lage aus GPS in UTM32, Zeit aus EXIF, Vorschaubild
    let p = c
        .upload(
            &format!("/api/sites/{sid}/photos?file=Schacht.jpg"),
            PHOTO.to_vec(),
        )
        .await;
    assert_eq!(p.status, StatusCode::CREATED, "{}", p.json());
    let photo = p.json();
    assert_eq!(photo["kind"], "photo");
    assert_eq!(photo["title"], "Schacht");
    assert!(
        (photo["x"].as_f64().unwrap() - 512_025.0).abs() < 0.01,
        "{photo}"
    );
    assert!(
        (photo["y"].as_f64().unwrap() - 5_403_015.0).abs() < 0.01,
        "{photo}"
    );
    assert_eq!(photo["takenAt"], 1_794_907_800_000i64);
    let pid = photo["id"].as_i64().unwrap();
    let full = c.get(&format!("/api/markers/{pid}/photo")).await;
    assert_eq!(full.body, PHOTO);
    let thumb = c.get(&format!("/api/markers/{pid}/thumb")).await;
    assert_eq!(&thumb.body[..2], &[0xff, 0xd8], "JPEG");
    let bad = c
        .upload(
            &format!("/api/sites/{sid}/photos?file=x.jpg"),
            b"kein Bild".to_vec(),
        )
        .await;
    assert_eq!(bad.json()["error"], "markers.notAnImage");

    // Liste: alle drei
    let list = c.get(&url).await.json();
    assert_eq!(list.as_array().unwrap().len(), 3);

    // bearbeiten und löschen (samt Dateien)
    let nid = note["id"].as_i64().unwrap();
    let r = c
        .json(
            Method::PATCH,
            &format!("/api/markers/{nid}"),
            json!({"note": "Labor Müller", "x": 512001.0, "y": 5403001.0}),
        )
        .await
        .json();
    assert_eq!(
        (r["note"].as_str(), r["x"].as_f64()),
        (Some("Labor Müller"), Some(512001.0))
    );
    let dir = c.app.cfg.data_dir.join("markers").join(pid.to_string());
    assert!(dir.join("thumb.jpg").exists());
    assert_eq!(
        c.delete(&format!("/api/markers/{pid}")).await.status,
        StatusCode::NO_CONTENT
    );
    assert!(!dir.exists());
    assert_eq!(c.get(&url).await.json().as_array().unwrap().len(), 2);

    // Baustelle löschen räumt mit auf
    c.delete(&format!("/api/sites/{sid}")).await;
    assert_eq!(
        c.get(&format!("/api/markers/{nid}/thumb")).await.status,
        StatusCode::NOT_FOUND
    );
}
