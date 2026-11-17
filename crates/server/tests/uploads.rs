//! Fortsetzbarer Upload (tus 1.0): Stücke, Abbruch mitten im Stück, Nachfragen mit HEAD, Fortsetzen; fertige
//! Dateien landen als Aufnahme, Drohnenfoto oder Orthofoto.

mod common;

use axum::body::Body;
use axum::http::{Method, StatusCode};
use base64::Engine;
use common::{terrain_xyz, Client};
use fixpunkt_server::jobs;
use serde_json::json;

fn meta(pairs: &[(&str, &str)]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD;
    pairs
        .iter()
        .map(|(k, v)| format!("{k} {}", b64.encode(v)))
        .collect::<Vec<_>>()
        .join(",")
}

async fn create(
    c: &mut Client,
    length: usize,
    metadata: String,
) -> (StatusCode, Option<String>, String) {
    let r = c
        .send_with(
            Method::POST,
            "/api/uploads",
            Body::empty(),
            None,
            true,
            &[
                ("Tus-Resumable", "1.0.0".into()),
                ("Upload-Length", length.to_string()),
                ("Upload-Metadata", metadata),
            ],
        )
        .await;
    let loc = r
        .headers
        .get("location")
        .map(|v| v.to_str().unwrap().to_string());
    (r.status, loc, String::from_utf8_lossy(&r.body).into_owned())
}

async fn patch(
    c: &mut Client,
    url: &str,
    offset: usize,
    data: Vec<u8>,
) -> (StatusCode, Option<u64>) {
    let r = c
        .send_with(
            Method::PATCH,
            url,
            Body::from(data),
            Some("application/offset+octet-stream"),
            true,
            &[
                ("Tus-Resumable", "1.0.0".into()),
                ("Upload-Offset", offset.to_string()),
            ],
        )
        .await;
    let off = r
        .headers
        .get("upload-offset")
        .and_then(|v| v.to_str().ok()?.parse().ok());
    (r.status, off)
}

async fn head(c: &mut Client, url: &str) -> (StatusCode, Option<u64>) {
    let r = c
        .send_with(
            Method::HEAD,
            url,
            Body::empty(),
            None,
            false,
            &[("Tus-Resumable", "1.0.0".into())],
        )
        .await;
    let off = r
        .headers
        .get("upload-offset")
        .and_then(|v| v.to_str().ok()?.parse().ok());
    (r.status, off)
}

#[tokio::test]
async fn resumable_capture_upload() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Upload"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let data = terrain_xyz(true);
    let n = data.len();
    let (st, loc, body) = create(
        &mut c,
        n,
        meta(&[
            ("filename", "flug.xyz"),
            ("target", "capture"),
            ("siteId", &sid.to_string()),
            ("name", "Flug Süd"),
            ("capturedAt", "1790000000000"),
        ]),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{body}");
    let url = loc.unwrap();

    // erstes Drittel, dann „Verbindung weg“ – der Server weiß, wie viel da ist
    let third = n / 3;
    assert_eq!(
        patch(&mut c, &url, 0, data[..third].to_vec()).await,
        (StatusCode::NO_CONTENT, Some(third as u64))
    );
    assert_eq!(
        head(&mut c, &url).await,
        (StatusCode::OK, Some(third as u64))
    );
    // falscher Offset wird abgelehnt
    assert_eq!(
        patch(&mut c, &url, 0, data[..10].to_vec()).await.0,
        StatusCode::CONFLICT
    );
    // nichts ist schon als Aufnahme sichtbar
    assert!(c
        .get(&format!("/api/sites/{sid}/captures"))
        .await
        .json()
        .as_array()
        .unwrap()
        .is_empty());
    // Rest in einem Stück
    assert_eq!(
        patch(&mut c, &url, third, data[third..].to_vec()).await,
        (StatusCode::NO_CONTENT, Some(n as u64))
    );
    // fertig: Teil-Upload weg, Aufnahme da und geprüft
    assert_eq!(head(&mut c, &url).await.0, StatusCode::NOT_FOUND);
    jobs::run_pending(&c.app).await.unwrap();
    let cap = c.get(&format!("/api/sites/{sid}/captures")).await.json()[0].clone();
    assert_eq!(cap["name"], "Flug Süd");
    assert_eq!(cap["capturedAt"], 1_790_000_000_000i64);
    assert_eq!(cap["fileSize"], n);
    assert_eq!(cap["status"], "ready");
    assert_eq!(cap["info"]["points"], 40_000);
}

#[tokio::test]
async fn targets_are_checked_before_the_first_byte() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Prüfen"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let s = sid.to_string();
    let (st, _, body) = create(
        &mut c,
        10,
        meta(&[
            ("filename", "x.docx"),
            ("target", "capture"),
            ("siteId", &s),
        ]),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert!(body.contains("captures.unsupportedFile"), "{body}");
    let (st, _, _) = create(
        &mut c,
        10,
        meta(&[
            ("filename", "a.laz"),
            ("target", "capture"),
            ("siteId", "999"),
        ]),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _, body) = create(
        &mut c,
        10,
        meta(&[("filename", "a.laz"), ("target", "rakete")]),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "{body}");
    // zu groß für die Einstellung
    let (st, _, _) = create(
        &mut c,
        usize::MAX / 2,
        meta(&[("filename", "a.laz"), ("target", "capture"), ("siteId", &s)]),
    )
    .await;
    assert_eq!(st, StatusCode::PAYLOAD_TOO_LARGE);
    // fremde und erfundene IDs
    assert_eq!(
        head(&mut c, "/api/uploads/..%2F..%2Ffixpunkt.db").await.0,
        StatusCode::NOT_FOUND
    );
    // abbrechen
    let (_, loc, _) = create(
        &mut c,
        10,
        meta(&[("filename", "a.laz"), ("target", "capture"), ("siteId", &s)]),
    )
    .await;
    let url = loc.unwrap();
    let r = c
        .send_with(Method::DELETE, &url, Body::empty(), None, true, &[])
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(head(&mut c, &url).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn ortho_by_upload() {
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
            terrain_xyz(false),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    jobs::run_pending(&c.app).await.unwrap();
    let tif = include_bytes!("../../../e2e/data/ortho.tif").to_vec();
    let (_, loc, _) = create(
        &mut c,
        tif.len(),
        meta(&[
            ("filename", "ortho.tif"),
            ("target", "ortho"),
            ("captureId", &cap.to_string()),
        ]),
    )
    .await;
    assert_eq!(
        patch(&mut c, &loc.unwrap(), 0, tif).await.0,
        StatusCode::NO_CONTENT
    );
    let caps = c.get(&format!("/api/sites/{sid}/captures")).await.json();
    assert_eq!(caps[0]["ortho"]["width"], 400);
}

#[tokio::test]
async fn interrupted_patch_keeps_what_arrived() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Abbruch"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let data = terrain_xyz(false);
    let (_, loc, _) = create(
        &mut c,
        data.len(),
        meta(&[
            ("filename", "g.xyz"),
            ("target", "capture"),
            ("siteId", &sid.to_string()),
        ]),
    )
    .await;
    let url = loc.unwrap();
    // 100 000 Bytes kommen an, dann reißt die Verbindung
    let first = data[..100_000].to_vec();
    let stream = futures::stream::iter(vec![
        Ok::<_, std::io::Error>(axum::body::Bytes::from(first)),
        Err(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "weg",
        )),
    ]);
    let r = c
        .send_with(
            Method::PATCH,
            &url,
            Body::from_stream(stream),
            Some("application/offset+octet-stream"),
            true,
            &[("Upload-Offset", "0".into())],
        )
        .await;
    assert_ne!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(head(&mut c, &url).await.1, Some(100_000));
    // fortsetzen ab dort
    let rest = data[100_000..].to_vec();
    assert_eq!(
        patch(&mut c, &url, 100_000, rest).await.0,
        StatusCode::NO_CONTENT
    );
    jobs::run_pending(&c.app).await.unwrap();
    let cap = c.get(&format!("/api/sites/{sid}/captures")).await.json()[0].clone();
    assert_eq!(cap["status"], "ready");
    assert_eq!(
        cap["info"]["points"], 40_000,
        "Datei vollständig und unverändert"
    );
}
