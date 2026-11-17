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

#[tokio::test]
async fn lines_and_dxf_export() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Leitungsbau"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let url = format!("/api/sites/{sid}/markers");
    // Leitung 3-4-5: 5 m, dann 6 m nach Norden; mittlerer Punkt ohne Gelände
    let line = c
        .json(
            Method::POST,
            &url,
            json!({"kind": "line", "title": "Kanal DN 300", "note": "Bestand",
                   "points": [[512000.0, 5403000.0, 299.5], [512003.0, 5403004.0], [512003.0, 5403010.0, 299.7]]}),
        )
        .await;
    assert_eq!(line.status, StatusCode::CREATED, "{}", line.json());
    let l = line.json();
    assert_eq!(l["kind"], "line");
    assert_eq!(l["points"][1], json!([512003.0, 5403004.0]));
    // für Liste und Karte liegt die Linie an ihrem Anfang
    assert_eq!(
        (l["x"].as_f64(), l["y"].as_f64()),
        (Some(512000.0), Some(5403000.0))
    );

    for bad in [
        json!({"kind": "line", "title": "kurz", "points": [[1.0, 2.0]]}),
        json!({"kind": "line", "title": "ohne"}),
        json!({"kind": "note", "title": "Notiz", "points": [[1.0, 2.0], [3.0, 4.0]]}),
    ] {
        let r = c.json(Method::POST, &url, bad.clone()).await;
        assert_eq!(r.json()["error"], "api.invalidInput", "{bad}");
    }
    // eine Linie wird kein Punkt
    let id = l["id"].as_i64().unwrap();
    let r = c
        .json(
            Method::PATCH,
            &format!("/api/markers/{id}"),
            json!({"kind": "note"}),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let r = c
        .json(
            Method::PATCH,
            &format!("/api/markers/{id}"),
            json!({"title": "Kanal DN 400"}),
        )
        .await;
    assert_eq!(r.json()["points"].as_array().unwrap().len(), 3);

    c.json(
        Method::POST,
        &url,
        json!({"kind": "constraint", "title": "Schacht", "x": 512003.0, "y": 5403010.0, "z": 297.85}),
    )
    .await;
    let dxf = c.get(&format!("/api/sites/{sid}/documentation.dxf")).await;
    assert_eq!(dxf.status, StatusCode::OK);
    assert_eq!(
        dxf.headers[axum::http::header::CONTENT_DISPOSITION],
        format!("attachment; filename=\"Dokumentation-Leitungsbau-{sid}.dxf\"")
    );
    let text = String::from_utf8(dxf.body).unwrap();
    assert!(text.starts_with("0\nSECTION") && text.ends_with("0\nEOF\n"));
    // Polylinie mit drei Stützpunkten; der mittlere übernimmt die Höhe davor (299,5)
    assert_eq!(text.matches("\nVERTEX\n").count(), 3);
    assert!(
        text.contains("VERTEX\n8\nFP_LEITUNGEN\n10\n512003.0000\n20\n5403004.0000\n30\n299.5000\n"),
        "{text}"
    );
    assert!(text.contains("Kanal DN 400") && text.contains("Schacht 297.85"));
    assert!(text.contains("FP_LEITUNGEN") && text.contains("FP_ZWANGSPUNKTE"));
}
