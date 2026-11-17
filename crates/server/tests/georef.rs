//! Passpunkte über die API: Vorschau der Transformation und Georeferenzieren als Auftrag, der eine neue
//! Aufnahme anlegt. Geprüft gegen eine bekannte Verschiebung des Testgeländes.

mod common;

use axum::http::{Method, StatusCode};
use common::{terrain_xyz, Client, X0, Y0};
use fixpunkt_server::jobs;
use serde_json::{json, Value};

/// Scan im lokalen System: 100 m östlich, 50 m südlich, 2 m zu hoch
const SHIFT: [f64; 3] = [100.0, -50.0, 2.0];

fn shifted_terrain() -> Vec<u8> {
    let text = String::from_utf8(terrain_xyz(false)).unwrap();
    let mut out = String::from("x y z\n");
    for line in text.lines().skip(1) {
        let v: Vec<f64> = line.split(' ').map(|s| s.parse().unwrap()).collect();
        out += &format!(
            "{:.3} {:.3} {:.3}\n",
            v[0] + SHIFT[0],
            v[1] + SHIFT[1],
            v[2] + SHIFT[2]
        );
    }
    out.into_bytes()
}

/// Vier Ecken: Sollkoordinate aus dem Gelände (z = 300 + 0,01 · dx), Scan-Koordinate verschoben
fn points() -> Value {
    let corners = [(0.1, 0.1), (39.9, 0.1), (39.9, 39.9), (0.1, 39.9)];
    Value::Array(
        corners
            .iter()
            .enumerate()
            .map(|(i, &(dx, dy))| {
                let dst = [X0 + dx, Y0 + dy, 300.0 + 0.01 * dx];
                let src = [dst[0] + SHIFT[0], dst[1] + SHIFT[1], dst[2] + SHIFT[2]];
                json!({"name": format!("P{}", i + 1), "src": src, "dst": dst})
            })
            .collect(),
    )
}

#[tokio::test]
async fn control_points_georeference_a_scan() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Passpunkte"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let scan = c
        .upload(
            &format!("/api/sites/{sid}/captures?file=scan.xyz&name=Scan%20lokal"),
            shifted_terrain(),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    jobs::run_pending(&c.app).await.unwrap();

    // Vorschau: Verschiebung zurück, Restklaffen null, Maßstab 1
    let fit = c
        .json(
            Method::POST,
            "/api/georef/fit",
            json!({"mode": "similarity", "points": points()}),
        )
        .await;
    assert_eq!(fit.status, StatusCode::OK, "{}", fit.json());
    let f = fit.json();
    for (i, s) in SHIFT.iter().enumerate() {
        assert!(
            (f["translation"][i].as_f64().unwrap() + s).abs() < 1e-3,
            "{f}"
        );
    }
    assert!(f["rms_3d"].as_f64().unwrap() < 1e-3);
    assert!(f["scale_ppm"].as_f64().unwrap().abs() < 1e-3);
    assert_eq!(f["residuals"].as_array().unwrap().len(), 4);

    // zwei Punkte reichen für Helmert nicht
    let two = Value::Array(points().as_array().unwrap()[..2].to_vec());
    let bad = c
        .json(Method::POST, "/api/georef/fit", json!({"points": two}))
        .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
    assert_eq!(bad.json()["error"], "georef.invalid");
    let r = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "georef", "params": {"captureId": scan, "points": two}}),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);

    // Auftrag: neue Aufnahme, die genau auf dem Gelände liegt
    let r = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "georef", "params": {"captureId": scan, "points": points()}}),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.json());
    let id = r.json()["id"].as_i64().unwrap();
    jobs::run_pending(&c.app).await.unwrap();
    let job = c.get(&format!("/api/jobs/{id}")).await.json();
    assert_eq!(job["status"], "done", "{job}");
    let new = job["result"]["captureId"].as_i64().unwrap();
    let caps = c.get(&format!("/api/sites/{sid}/captures")).await.json();
    let cap = caps
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == new)
        .unwrap();
    assert_eq!(cap["name"], "Scan lokal (georeferenziert)");
    assert_eq!(cap["fileName"], "scan-georef.laz");
    assert_eq!(cap["status"], "ready");
    let info = &cap["info"];
    assert_eq!(info["points"], 40_000);
    assert!(
        (info["bbox"]["minX"].as_f64().unwrap() - (X0 + 0.1)).abs() < 0.002,
        "{info}"
    );
    assert!(
        (info["bbox"]["maxY"].as_f64().unwrap() - (Y0 + 39.9)).abs() < 0.002,
        "{info}"
    );
    assert!(
        (info["zMin"].as_f64().unwrap() - 300.001).abs() < 0.002,
        "{info}"
    );
    // der lokale Scan bleibt, wie er war
    assert_eq!(caps.as_array().unwrap().len(), 2);
}
