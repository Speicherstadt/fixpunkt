//! Überblick einer Baustelle über die API, mit echten Rechnungen.

mod common;

use axum::http::{Method, StatusCode};
use common::{terrain_xyz, Client, X0, Y0};
use fixpunkt_server::jobs;
use serde_json::json;

#[tokio::test]
async fn dashboard_of_a_pit() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Überblick"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let empty = c.get(&format!("/api/sites/{sid}/dashboard")).await.json();
    assert_eq!(empty["captures"]["count"], 0);
    assert_eq!(empty["progress"], json!([]));

    let mut ids = Vec::new();
    for (pit, name, at) in [(false, "Urgelände", 1), (true, "Aushub", 2)] {
        let r = c
            .upload(
                &format!("/api/sites/{sid}/captures?file={name}.xyz&capturedAt={at}"),
                terrain_xyz(pit),
            )
            .await;
        ids.push(r.json()["id"].as_i64().unwrap());
    }
    jobs::run_pending(&c.app).await.unwrap();
    for (kind, params) in [
        ("volume", json!({"before": ids[0], "after": ids[1]})),
        (
            "compare",
            json!({"captureId": ids[1], "design": {"type": "height", "height": 298.2}, "tolerance": 0.05}),
        ),
    ] {
        c.json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": kind, "params": params}),
        )
        .await;
    }
    // Zwangspunkte in der Grubenmitte (Gelände 298,20 m): einer eingehalten, einer 20 cm verletzt
    for (title, z) in [("Sohle Mitte", 298.21), ("Kanal", 298.0)] {
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
        json!({"kind": "note", "title": "Notiz"}),
    )
    .await;
    jobs::run_pending(&c.app).await.unwrap();

    let d = c.get(&format!("/api/sites/{sid}/dashboard")).await.json();
    assert_eq!(d["captures"]["count"], 2);
    assert_eq!(d["captures"]["last"]["name"], "Aushub.xyz");
    // Grube 20 × 20 × 2 m
    let p = &d["progress"];
    assert_eq!(p.as_array().unwrap().len(), 1);
    assert_eq!(p[0]["name"], "Aushub.xyz");
    assert!((p[0]["cut"].as_f64().unwrap() - 800.0).abs() < 8.0, "{p}");
    assert!(d["latestCompare"]["within"].as_f64().unwrap() > 0.0);
    assert_eq!(d["latestCompare"]["tolerance"], 0.05);
    assert_eq!(d["jobs"], json!({"queued": 0, "running": 0, "failed": 0}));
    let k = &d["constraints"];
    assert_eq!(
        (k["total"].as_u64(), k["checked"].as_u64()),
        (Some(2), Some(2))
    );
    assert_eq!(k["violated"].as_array().unwrap().len(), 1);
    assert_eq!(k["violated"][0]["title"], "Kanal");
    assert!(
        (k["violated"][0]["diff"].as_f64().unwrap() - 0.2).abs() < 0.01,
        "{k}"
    );
    assert_eq!(k["against"], "Aushub.xyz");
    assert_eq!(d["documentation"], json!({"photos": 0, "notes": 1}));
    assert_eq!(d["plans"], 0);
}

/// Massenziel: Bereich genau um die Grube (20 × 20 m, 2 m tief = 800 m³), Ziel 1000 m³ Abtrag → 80 %
#[tokio::test]
async fn region_targets() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Ziele"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let fc = json!({"type": "FeatureCollection",
        "crs": {"type": "name", "properties": {"name": "urn:ogc:def:crs:EPSG::25832"}},
        "features": [common::rect("Grube", 10.0, 10.0, 30.0, 30.0)]});
    let created = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/regions"),
            json!({"geojson": fc}),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.json());
    let rid = created.json()[0]["id"].as_i64().unwrap();
    let url = format!("/api/regions/{rid}");
    for bad in [
        json!({"targetM3": -5.0}),
        json!({"targetM3": 10.0, "targetKind": "weg"}),
    ] {
        assert_eq!(
            c.json(Method::PATCH, &url, bad).await.status,
            StatusCode::BAD_REQUEST
        );
    }
    let r = c
        .json(
            Method::PATCH,
            &url,
            json!({"material": " Boden 3–5 ", "targetM3": 1000.0}),
        )
        .await
        .json();
    assert_eq!(
        (&r["material"], &r["targetM3"], &r["targetKind"], &r["name"]),
        (
            &json!("Boden 3–5"),
            &json!(1000.0),
            &json!("cut"),
            &json!("Grube")
        )
    );

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
    // vor der Berechnung: Ziel ohne Stand
    let d = c.get(&format!("/api/sites/{sid}/dashboard")).await.json();
    assert_eq!(d["targets"][0]["done"], json!(null));
    c.json(
        Method::POST,
        &format!("/api/sites/{sid}/jobs"),
        json!({"kind": "volume", "params": {"before": ids[0], "after": ids[1], "regionIds": [rid]}}),
    )
    .await;
    jobs::run_pending(&c.app).await.unwrap();
    let d = c.get(&format!("/api/sites/{sid}/dashboard")).await.json();
    let t = &d["targets"][0];
    assert_eq!(t["name"], "Grube");
    let done = t["done"].as_f64().unwrap();
    assert!((done - 800.0).abs() < 8.0, "{t}");
    assert!((t["percent"].as_f64().unwrap() - done / 10.0).abs() < 1e-9);
    assert!((t["open"].as_f64().unwrap() - (1000.0 - done)).abs() < 1e-9);
    assert_eq!(d["materials"][0]["material"], "Boden 3–5");

    // Ziel löschen: fehlt im Überblick
    c.json(Method::PATCH, &url, json!({})).await;
    let d = c.get(&format!("/api/sites/{sid}/dashboard")).await.json();
    assert_eq!(d["targets"], json!([]));
}

/// Lager: Bereich 30 × 30 m, Basis 299 m. Woche 1 ohne Grube: Gelände im Mittel 300,2 m → 900 m² · 1,2 m =
/// 1.080 m³. Woche 2 mit Grube (20 × 20 m, Sohle unter der Basis): dort fehlen die 400 m² · 1,2 m = 480 m³ über
/// 299 m – was unter der Basis liegt, zählt nicht negativ → 600 m³, Änderung −480 m³.
#[tokio::test]
async fn stock_per_material() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Lager"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let fc = json!({"type": "FeatureCollection",
        "crs": {"type": "name", "properties": {"name": "urn:ogc:def:crs:EPSG::25832"}},
        "features": [common::rect("Halde", 5.0, 5.0, 35.0, 35.0)]});
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
    // Lager ohne Soll-Menge; eine mitgeschickte wird nicht gespeichert
    let r = c
        .json(
            Method::PATCH,
            &format!("/api/regions/{rid}"),
            json!({"material": "Boden Klasse 4", "targetKind": "stock", "targetM3": 5.0}),
        )
        .await
        .json();
    assert_eq!(
        (&r["targetKind"], &r["targetM3"]),
        (&json!("stock"), &json!(null))
    );

    let mut caps = Vec::new();
    for (pit, at) in [(false, 1), (true, 2)] {
        let r = c
            .upload(
                &format!("/api/sites/{sid}/captures?file={at}.xyz&capturedAt={at}"),
                terrain_xyz(pit),
            )
            .await;
        caps.push(r.json()["id"].as_i64().unwrap());
    }
    jobs::run_pending(&c.app).await.unwrap();
    let d = c.get(&format!("/api/sites/{sid}/dashboard")).await.json();
    assert_eq!(
        d["stock"][0]["current"],
        json!(null),
        "noch nicht berechnet"
    );
    assert_eq!(d["targets"], json!([]), "ein Lager ist kein Ziel");
    for cap in &caps {
        c.json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "stockpile", "params": {"captureId": cap, "regionIds": [rid], "base": 299.0}}),
        )
        .await;
    }
    jobs::run_pending(&c.app).await.unwrap();
    let d = c.get(&format!("/api/sites/{sid}/dashboard")).await.json();
    // Verlauf: Woche 1 → Woche 2
    let h: Vec<f64> = d["stockHistory"][0]["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["value"].as_f64().unwrap())
        .collect();
    assert_eq!(h.len(), 2);
    assert!(
        (h[0] - 1080.0).abs() < 10.0 && (h[1] - 600.0).abs() < 10.0,
        "{h:?}"
    );
    let s = &d["stock"][0];
    assert_eq!(s["material"], "Boden Klasse 4");
    let (now, before) = (
        s["current"].as_f64().unwrap(),
        s["previous"].as_f64().unwrap(),
    );
    assert!((before - 1080.0).abs() < 10.0, "{s}");
    assert!((now - 600.0).abs() < 10.0, "{s}");
    assert!((s["change"].as_f64().unwrap() - (now - before)).abs() < 1e-9);
    assert!((now - before + 480.0).abs() < 5.0, "{s}");
}
