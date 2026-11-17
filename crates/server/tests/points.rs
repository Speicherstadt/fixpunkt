//! Punktwolke für die 3D-Ansicht über die API: Farben aus Text und E57, Lage relativ zum Ursprung.

mod common;

use axum::http::{Method, StatusCode};
use common::{terrain_xyz, Client, X0, Y0};
use fixpunkt_server::{jobs, points};
use serde_json::json;

#[tokio::test]
async fn points_with_and_without_colors() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "3D"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    // Text mit Farben: Grube rot, Rest grau
    let mut colored = String::from("x y z r g b\n");
    for line in String::from_utf8(terrain_xyz(true))
        .unwrap()
        .lines()
        .skip(1)
    {
        let v: Vec<f64> = line.split(' ').map(|s| s.parse().unwrap()).collect();
        let rgb = if v[2] < 299.0 {
            "200 30 30"
        } else {
            "128 128 128"
        };
        colored += &format!("{line} {rgb}\n");
    }
    let a = c
        .upload(
            &format!("/api/sites/{sid}/captures?file=bunt.xyz"),
            colored.into_bytes(),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let b = c
        .upload(
            &format!("/api/sites/{sid}/captures?file=grau.xyz"),
            terrain_xyz(false),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    jobs::run_pending(&c.app).await.unwrap();

    let r = c.get(&format!("/api/captures/{a}/points")).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.headers["x-points-count"], "40000");
    assert_eq!(r.headers["x-points-colors"], "1");
    let (n, colored, origin) = points::header(&r.body).unwrap();
    assert_eq!((n, colored), (40_000, true));
    // Mitte des Geländes (0,1 … 39,9 m), Unterkante = tiefster Punkt der Sohle (300 + 0,01 · 10,1 − 2)
    assert!((origin[0] - (X0 + 20.0)).abs() < 1e-9 && (origin[1] - (Y0 + 20.0)).abs() < 1e-9);
    assert!((origin[2] - 298.101).abs() < 1e-6, "{origin:?}");
    let rgb = &r.body[29 + n * 12..];
    let red = rgb.chunks(3).filter(|c| c == &[200, 30, 30]).count();
    // Grube 20 × 20 m bei 0,2 m: 100 × 100 Punkte
    assert_eq!(red, 10_000);

    let r = c.get(&format!("/api/captures/{b}/points")).await;
    assert_eq!(r.headers["x-points-colors"], "0");
    assert_eq!(r.body.len(), 29 + 40_000 * 12);
    // zweiter Abruf aus dem Zwischenspeicher, gleich
    assert_eq!(
        c.get(&format!("/api/captures/{b}/points")).await.body,
        r.body
    );
    assert!(c.app.cfg.capture_dir(b).join("punkte.bin").exists());
    assert_eq!(
        c.get("/api/captures/999/points").await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn e57_scan_as_capture() {
    use e57::{E57Writer, Record, RecordValue};
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Scanner"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    // E57 mit 400 Punkten auf einer schrägen Ebene, Farbe aus dem Scanner
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scan.e57");
    {
        let mut w = E57Writer::from_file(&path, "guid-test").unwrap();
        let proto = vec![
            Record::CARTESIAN_X_F64,
            Record::CARTESIAN_Y_F64,
            Record::CARTESIAN_Z_F64,
            Record::COLOR_RED_U8,
            Record::COLOR_GREEN_U8,
            Record::COLOR_BLUE_U8,
        ];
        let mut pc = w.add_pointcloud("scan", proto).unwrap();
        for i in 0..20 {
            for j in 0..20 {
                let (x, y) = (X0 + f64::from(i), Y0 + f64::from(j));
                pc.add_point(vec![
                    RecordValue::Double(x),
                    RecordValue::Double(y),
                    RecordValue::Double(300.0 + 0.01 * f64::from(i)),
                    RecordValue::Integer(10),
                    RecordValue::Integer(200),
                    RecordValue::Integer(10),
                ])
                .unwrap();
            }
        }
        pc.finalize().unwrap();
        w.finalize().unwrap();
    }
    let id = c
        .upload(
            &format!("/api/sites/{sid}/captures?file=scan.e57"),
            std::fs::read(&path).unwrap(),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    jobs::run_pending(&c.app).await.unwrap();
    let cap = c.get(&format!("/api/sites/{sid}/captures")).await.json()[0].clone();
    assert_eq!(cap["status"], "ready", "{cap}");
    assert_eq!(cap["info"]["points"], 400);
    assert!((cap["info"]["zMax"].as_f64().unwrap() - 300.19).abs() < 1e-6);
    let r = c.get(&format!("/api/captures/{id}/points")).await;
    assert_eq!(r.headers["x-points-colors"], "1");
    assert_eq!(&r.body[29 + 400 * 12..][..3], &[10, 200, 10]);
}
