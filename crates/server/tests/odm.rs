//! Drohnenfotos über NodeODM: echter Router und Worker gegen einen NodeODM-Nachbau im Test (HTTP auf
//! 127.0.0.1). Der Nachbau liefert das Testgelände als Punktwolke in UTM-Zone 33 mit ellipsoidischen
//! Höhen; Fixpunkt muss sie in Zone 32 und auf Normalhöhen bringen.

mod common;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{Method, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use common::{terrain_xyz, Client, X0, Y0};
use fixpunkt::crs::Utm;
use fixpunkt::pointcloud::{Point3, PointCloud};
use fixpunkt_server::jobs;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

/// Höhenanomalie, die der Nachbau auf die Höhen aufschlägt und Fixpunkt wieder abzieht
const N: f64 = 47.25;

#[derive(Default)]
struct Stub {
    init: String,
    images: usize,
    polls: usize,
    fail: bool,
    removed: Vec<String>,
    model: Vec<u8>,
}

type Shared = Arc<Mutex<Stub>>;

/// Testgelände (Grube) als LAZ in Zone 33 mit Höhen + N, wie es ODM östlich von 12° liefern würde
fn odm_model() -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let xyz = dir.path().join("g.xyz");
    std::fs::write(&xyz, terrain_xyz(true)).unwrap();
    let mut c = PointCloud::read(&xyz, Default::default()).unwrap();
    let (z32, z33) = (Utm::new(32).unwrap(), Utm::new(33).unwrap());
    c.map(|p| {
        let (x, y) = z32.to_zone(&z33, p.x, p.y);
        Point3 { x, y, z: p.z + N }
    });
    let out = dir.path().join("m.laz");
    c.write_las_georef(&out, None, Some(32633)).unwrap();
    std::fs::read(out).unwrap()
}

async fn nodeodm(stub: Shared) -> String {
    let app = Router::new()
        .route(
            "/info",
            get(|| async { Json(json!({"version": "2.5.2", "taskQueueCount": 1, "engineVersion": "3.5.6"})) }),
        )
        .route(
            "/task/new/init",
            post(|State(s): State<Shared>, body: Bytes| async move {
                s.lock().unwrap().init = String::from_utf8_lossy(&body).into_owned();
                Json(json!({"uuid": "aufgabe-1"}))
            }),
        )
        .route(
            "/task/new/upload/{uuid}",
            post(|State(s): State<Shared>, body: Bytes| async move {
                let n = String::from_utf8_lossy(&body).matches("filename=").count();
                s.lock().unwrap().images += n;
                Json(json!({"success": true}))
            }),
        )
        .route(
            "/task/new/commit/{uuid}",
            post(|Path(u): Path<String>| async move { Json(json!({"uuid": u})) }),
        )
        .route(
            "/task/{uuid}/info",
            get(|State(s): State<Shared>| async move {
                let mut s = s.lock().unwrap();
                s.polls += 1;
                Json(match (s.fail, s.polls) {
                    (true, _) => json!({"status": {"code": 30, "errorMessage": "Not enough images"}, "progress": 12}),
                    (false, 1) => json!({"status": {"code": 20}, "progress": 50}),
                    _ => json!({"status": {"code": 40}, "progress": 100, "processingTime": 61000}),
                })
            }),
        )
        .route(
            "/task/{uuid}/output",
            get(|| async { Json(json!(["[INFO] Running ODM", "[ERROR] Not enough images in dataset"])) }),
        )
        .route(
            "/task/{uuid}/download/{asset}",
            get(|State(s): State<Shared>, Path((_, asset)): Path<(String, String)>| async move {
                if asset == "georeferenced_model.laz" {
                    (StatusCode::OK, s.lock().unwrap().model.clone())
                } else {
                    (StatusCode::NOT_FOUND, Vec::new())
                }
            }),
        )
        .route(
            "/task/remove",
            post(|State(s): State<Shared>, body: Bytes| async move {
                let b = String::from_utf8_lossy(&body).into_owned();
                s.lock().unwrap().removed.push(b);
                Json(json!({"success": true}))
            }),
        )
        .with_state(stub);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

async fn setup(fail: bool) -> (Client, Shared, i64) {
    let stub: Shared = Arc::new(Mutex::new(Stub {
        fail,
        model: odm_model(),
        ..Default::default()
    }));
    let url = nodeodm(stub.clone()).await;
    let mut c = Client::with(|cfg| cfg.nodeodm_url = Some(url)).await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Grube Ost"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    (c, stub, sid)
}

/// Entwurf anlegen und `n` Fotos hochladen; gibt die Auftragsnummer
async fn draft(c: &mut Client, sid: i64, n: usize) -> i64 {
    let r = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/odm"),
            json!({"name": "Flug Ost", "capturedAt": 1_790_000_000_000i64, "quality": "fast", "heightOffset": N}),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.json());
    assert_eq!(r.json()["status"], "draft");
    let id = r.json()["id"].as_i64().unwrap();
    for i in 0..n {
        let r = c
            .upload(
                &format!("/api/jobs/{id}/images?file=DJI_{i:04}.JPG"),
                vec![0xff, 0xd8, 0xff, i as u8],
            )
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.json());
        assert_eq!(r.json()["images"], i + 1);
    }
    id
}

#[tokio::test]
async fn photos_become_a_capture_in_the_site_zone() {
    let (mut c, stub, sid) = setup(false).await;
    let status = c.get("/api/odm").await.json();
    assert_eq!(status["reachable"], true);
    assert_eq!(status["version"], "2.5.2");

    let id = draft(&mut c, sid, 3).await;
    // Entwürfe übergeht der Worker
    assert_eq!(jobs::run_pending(&c.app).await.unwrap(), 0);
    let r = c
        .json(Method::POST, &format!("/api/jobs/{id}/start"), json!({}))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json());
    // nach dem Start keine Fotos mehr
    let late = c
        .upload(&format!("/api/jobs/{id}/images?file=spaet.jpg"), vec![1])
        .await;
    assert_eq!(late.json()["error"], "odm.notDraft");

    // Rechen-Worker lassen den Auftrag liegen (eigene Warteschlange, blockiert keine Rechnungen)
    assert!(!jobs::run_next(&c.app, jobs::Lane::Compute).await.unwrap());
    // Photogrammetrie, dann die Prüfung der neuen Aufnahme
    assert_eq!(jobs::run_pending(&c.app).await.unwrap(), 2);
    let job = c.get(&format!("/api/jobs/{id}")).await.json();
    assert_eq!(job["status"], "done", "{job}");
    assert_eq!(job["progress"], 100.0);
    let r = &job["result"];
    assert_eq!(r["images"], 3);
    assert_eq!(r["sourceEpsg"], 32633);
    assert_eq!(r["converted"], true);
    {
        let s = stub.lock().unwrap();
        assert_eq!(s.images, 3);
        assert!(
            s.init.contains("pc-quality") && s.init.contains("low"),
            "{}",
            s.init
        );
        assert!(s.init.contains("Flug Ost"));
        assert!(
            s.removed.iter().any(|b| b.contains("aufgabe-1")),
            "NodeODM aufgeräumt"
        );
    }

    // Aufnahme: Name und Datum aus dem Auftrag, Lage zurück in Zone 32, Höhen ohne N
    let cid = r["captureId"].as_i64().unwrap();
    let cap = c.get(&format!("/api/sites/{sid}/captures")).await.json()[0].clone();
    assert_eq!(cap["id"], cid);
    assert_eq!(cap["name"], "Flug Ost");
    assert_eq!(cap["capturedAt"], 1_790_000_000_000i64);
    assert_eq!(cap["status"], "ready", "{cap}");
    let info = &cap["info"];
    assert_eq!(info["points"], 40_000);
    let bbox = &info["bbox"];
    // Gelände 0,1 … 39,9 m ab X0/Y0 (Punkte in Zellmitten), mm-genau zurückgerechnet
    assert!(
        (bbox["minX"].as_f64().unwrap() - (X0 + 0.1)).abs() < 0.002,
        "{bbox}"
    );
    assert!(
        (bbox["minY"].as_f64().unwrap() - (Y0 + 0.1)).abs() < 0.002,
        "{bbox}"
    );
    // Grube 2 m unter 300 m, Rand bis 300,4 m
    assert!(
        (info["zMin"].as_f64().unwrap() - 298.1).abs() < 0.01,
        "{info}"
    );
    assert!(
        (info["zMax"].as_f64().unwrap() - 300.399).abs() < 0.01,
        "{info}"
    );
}

#[tokio::test]
async fn failed_processing_reports_nodeodm_output() {
    let (mut c, stub, sid) = setup(true).await;
    let id = draft(&mut c, sid, 3).await;
    c.json(Method::POST, &format!("/api/jobs/{id}/start"), json!({}))
        .await;
    jobs::run_pending(&c.app).await.unwrap();
    let job = c.get(&format!("/api/jobs/{id}")).await.json();
    assert_eq!(job["status"], "failed");
    let err = job["error"].as_str().unwrap();
    assert!(err.contains("Not enough images"), "{err}");
    assert!(err.contains("[ERROR]"), "letzte Zeilen der Ausgabe: {err}");
    assert!(c
        .get(&format!("/api/sites/{sid}/captures"))
        .await
        .json()
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!stub.lock().unwrap().removed.is_empty());
    // fehlgeschlagen lässt sich löschen, samt Fotos
    assert_eq!(
        c.delete(&format!("/api/jobs/{id}")).await.status,
        StatusCode::NO_CONTENT
    );
    assert!(!c.app.cfg.job_dir(id).exists());
}

#[tokio::test]
async fn drafts_are_checked() {
    let (mut c, _, sid) = setup(false).await;
    let id = draft(&mut c, sid, 2).await;
    let r = c
        .json(Method::POST, &format!("/api/jobs/{id}/start"), json!({}))
        .await;
    assert_eq!(r.json()["error"], "odm.tooFewImages");
    let r = c
        .upload(&format!("/api/jobs/{id}/images?file=wolke.laz"), vec![1])
        .await;
    assert_eq!(r.json()["error"], "odm.unsupportedImage");
    let r = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/odm"),
            json!({"name": "  "}),
        )
        .await;
    assert_eq!(r.json()["error"], "auth.nameRequired");
    // Fotos nur bei Photogrammetrie-Aufträgen
    let other: Value = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "volume", "params": {}}),
        )
        .await
        .json();
    if let Some(oid) = other["id"].as_i64() {
        let r = c
            .upload(&format!("/api/jobs/{oid}/images?file=a.jpg"), vec![1])
            .await;
        assert_eq!(r.status, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn without_nodeodm_the_feature_is_off() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    assert_eq!(c.get("/api/odm").await.json(), json!({"configured": false}));
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Ohne"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let r = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/odm"),
            json!({"name": "Flug"}),
        )
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.json()["error"], "odm.notConfigured");
}

#[tokio::test]
async fn after_a_restart_the_same_task_is_polled_again() {
    let (mut c, stub, sid) = setup(false).await;
    let id = draft(&mut c, sid, 3).await;
    c.json(Method::POST, &format!("/api/jobs/{id}/start"), json!({}))
        .await;
    // Stand wie nach einem Neustart: Aufgabe in NodeODM läuft schon, alle Fotos sind oben
    std::fs::write(
        c.app.cfg.job_dir(id).join("nodeodm.json"),
        r#"{"uuid": "aufgabe-1", "committed": true}"#,
    )
    .unwrap();
    jobs::run_pending(&c.app).await.unwrap();
    assert_eq!(
        c.get(&format!("/api/jobs/{id}")).await.json()["status"],
        "done"
    );
    let s = stub.lock().unwrap();
    assert_eq!(s.images, 0, "keine Fotos erneut hochgeladen");
    assert!(s.init.is_empty(), "keine neue Aufgabe");
}
