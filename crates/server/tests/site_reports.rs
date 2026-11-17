//! Berichte einer Baustelle über die API: Dokumentation mit Foto, Plan/Ist des Lageplans – beide gegen die
//! Testgrube (Sohle bei x = 20 m auf 298,20 m, bei x = 15 m auf 298,15 m).

mod common;

use axum::http::{header, Method, StatusCode};
use common::{terrain_xyz, Client, X0, Y0};
use fixpunkt_server::jobs;
use serde_json::json;

const PHOTO: &[u8] = include_bytes!("../../../e2e/data/foto-gps.jpg");
const PLAN: &[u8] = include_bytes!("../../../e2e/data/lageplan.dxf");

fn pdf(bytes: &[u8]) -> (lopdf::Document, String) {
    let doc = lopdf::Document::load_mem(bytes).expect("gültiges PDF");
    let pages: Vec<u32> = doc.get_pages().keys().copied().collect();
    let text = doc.extract_text(&pages).expect("Text lesbar");
    (doc, text)
}

#[tokio::test]
async fn documentation_and_plan_reports() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(
            Method::POST,
            "/api/sites",
            json!({"name": "Kirchstrasse 12"}),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();

    // ohne Einträge und ohne Aufnahme: trotzdem ein Bericht
    let empty = c
        .get(&format!("/api/sites/{sid}/documentation/report"))
        .await;
    assert_eq!(empty.status, StatusCode::OK);
    assert!(pdf(&empty.body).1.contains("noch nichts dokumentiert"));

    c.upload(
        &format!("/api/sites/{sid}/captures?file=aushub.xyz&capturedAt=1"),
        terrain_xyz(true),
    )
    .await;
    jobs::run_pending(&c.app).await.unwrap();
    for (title, z) in [("Kanal", 298.15), ("Schacht", 298.19)] {
        c.json(
            Method::POST,
            &format!("/api/sites/{sid}/markers"),
            json!({"kind": "constraint", "title": title, "x": X0 + 20.0, "y": Y0 + 20.0, "z": z}),
        )
        .await;
    }
    c.json(
        Method::POST,
        &format!("/api/sites/{sid}/markers"),
        json!({"kind": "note", "title": "Bodenprobe BP 2", "note": "Labor Z0", "x": X0 + 5.0, "y": Y0 + 5.0}),
    )
    .await;
    let photo = c
        .upload(
            &format!("/api/sites/{sid}/photos?file=Schacht.jpg"),
            PHOTO.to_vec(),
        )
        .await;
    assert_eq!(photo.status, StatusCode::CREATED, "{}", photo.json());

    let r = c
        .get(&format!("/api/sites/{sid}/documentation/report"))
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.headers[header::CONTENT_TYPE], "application/pdf");
    assert_eq!(
        r.headers[header::CONTENT_DISPOSITION],
        format!("attachment; filename=\"Dokumentation-Kirchstrasse-12-{sid}.pdf\"")
    );
    let (doc, text) = pdf(&r.body);
    // Kanal 298,15 gegen Gelände 298,20: +5,0 cm verletzt; Schacht +1,0 cm eingehalten
    for s in [
        "Dokumentation",
        "Kanal",
        "5,0 cm",
        "verletzt",
        "1,0 cm",
        "eingehalten",
        "1 / 2",
        "Bodenprobe BP 2",
        "Labor Z0",
        "Schacht",
        "aushub.xyz",
        "Blickrichtung 135",
    ] {
        assert!(text.contains(s), "{s} fehlt in {text}");
    }
    // das Foto steckt als JPEG (DCTDecode) im PDF, so groß wie das Vorschaubild
    let jpegs: Vec<i64> = doc
        .objects
        .values()
        .filter_map(|o| o.as_stream().ok())
        .filter(|s| s.dict.get(b"Filter").and_then(|f| f.as_name()).ok() == Some(b"DCTDecode"))
        .map(|s| s.dict.get(b"Width").unwrap().as_i64().unwrap())
        .collect();
    assert_eq!(jpegs.len(), 1, "ein Foto");
    assert!(jpegs[0] > 0 && jpegs[0] <= 480);

    // Plan/Ist: Höhenpunkte 298,20 (x = 20) und 298,25 (x = 15, Gelände 298,15) → 0,0 und −10,0 cm
    let plan = c
        .upload(
            &format!("/api/sites/{sid}/plans?file=lageplan.dxf"),
            PLAN.to_vec(),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let r = c.get(&format!("/api/plans/{plan}/report")).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.headers[header::CONTENT_DISPOSITION],
        format!("attachment; filename=\"Plan-Ist-Kirchstrasse-12-{plan}.pdf\"")
    );
    let text = pdf(&r.body).1;
    for s in [
        "Plan/Ist", "lageplan", "1 / 2", "298,20 m", "298,25 m", "298,15 m", "10,0 cm", "5,0 cm",
    ] {
        assert!(text.contains(s), "{s} fehlt in {text}");
    }
    assert_eq!(
        c.get("/api/plans/999/report").await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        c.get("/api/sites/999/documentation/report").await.status,
        StatusCode::NOT_FOUND
    );
}

/// Eigener Bericht: Bausteine in gewählter Reihenfolge, Titel und Firma; Aushub 800 m³ gegen ein Ziel von 1.000 m³
#[tokio::test]
async fn custom_report_from_parts() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(
            Method::POST,
            "/api/sites",
            json!({"name": "Kirchstrasse 12"}),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let fc = json!({"type": "FeatureCollection",
        "crs": {"type": "name", "properties": {"name": "urn:ogc:def:crs:EPSG::25832"}},
        "features": [common::rect("Grube", 10.0, 10.0, 30.0, 30.0)]});
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
    c.json(
        Method::PATCH,
        &format!("/api/regions/{rid}"),
        json!({"material": "Boden Klasse 4", "targetM3": 1000.0}),
    )
    .await;
    let mut ids = Vec::new();
    for (pit, at) in [(false, 1), (true, 2)] {
        let r = c
            .upload(
                &format!("/api/sites/{sid}/captures?file={at}.xyz&capturedAt={at}"),
                terrain_xyz(pit),
            )
            .await;
        ids.push(r.json()["id"].as_i64().unwrap());
    }
    jobs::run_pending(&c.app).await.unwrap();
    c.json(
        Method::POST,
        &format!("/api/sites/{sid}/jobs"),
        json!({"kind": "volume", "params": {"before": ids[0], "after": ids[1], "regionIds": [rid]}}),
    )
    .await;
    jobs::run_pending(&c.app).await.unwrap();
    c.upload(
        &format!("/api/sites/{sid}/photos?file=Schacht.jpg"),
        PHOTO.to_vec(),
    )
    .await;

    let r = c
        .get(&format!(
            "/api/sites/{sid}/report?parts=photos,kpis,targets&title=Wochenbericht%20KW%2047&company=Muster%20Tiefbau"
        ))
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.headers[header::CONTENT_DISPOSITION],
        format!("attachment; filename=\"Bericht-Kirchstrasse-12-{sid}.pdf\"")
    );
    let (doc, text) = pdf(&r.body);
    for s in [
        "Wochenbericht KW 47",
        "Muster Tiefbau",
        "Massenziele",
        "Boden Klasse 4",
        "1.000 m³",
        "Aufnahmen",
        "Schacht",
    ] {
        assert!(text.contains(s), "{s} fehlt in {text}");
    }
    // Aushub 800 m³: Anteil 80 %
    assert!(text.contains("80 %"), "{text}");
    // gewählte Reihenfolge: Fotos vor Massenzielen
    assert!(text.find("Fotos").unwrap() < text.find("Massenziele").unwrap());
    let jpegs = doc
        .objects
        .values()
        .filter_map(|o| o.as_stream().ok())
        .filter(|s| s.dict.get(b"Filter").and_then(|f| f.as_name()).ok() == Some(b"DCTDecode"))
        .count();
    assert_eq!(jpegs, 1);

    let bad = c
        .get(&format!("/api/sites/{sid}/report?parts=kpis,quatsch"))
        .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        c.get("/api/sites/999/report?parts=kpis").await.status,
        StatusCode::NOT_FOUND
    );
}
