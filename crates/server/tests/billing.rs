//! Abrechnung über die API mit echter Aushub-Rechnung: Kennwerte, Preis je Bereich, Aufmaß, Lieferscheine.

mod common;

use axum::http::{Method, StatusCode};
use common::{terrain_xyz, Client};
use fixpunkt_server::jobs;
use serde_json::json;

const DAY: i64 = 86_400_000;

#[tokio::test]
async fn statement_with_prices_and_deliveries() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Abrechnung"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();

    // Urgelände am 2.1.1970, Aushub am 1.2.1970, jeweils 9 Uhr
    let mut ids = Vec::new();
    for (pit, name, at) in [(false, "Urgelände", DAY), (true, "Januar", 31 * DAY)] {
        let at = at + 9 * 3_600_000;
        let r = c
            .upload(
                &format!("/api/sites/{sid}/captures?file={name}.xyz&capturedAt={at}"),
                terrain_xyz(pit),
            )
            .await;
        ids.push(r.json()["id"].as_i64().unwrap());
    }
    jobs::run_pending(&c.app).await.unwrap();
    // Bereich genau um die Grube: 20 × 20 m, 2 m tief = 800 m³
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

    // Preis: ungültige Einheit, negativer Preis
    let url = format!("/api/regions/{rid}");
    for bad in [
        json!({"unitPrice": 5.0, "priceUnit": "kg"}),
        json!({"unitPrice": -1.0}),
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
            json!({"material": "Lehm", "lvPosition": " 01.02.0010 ", "unitPrice": 12.5, "priceUnit": "t"}),
        )
        .await
        .json();
    assert_eq!(
        (&r["lvPosition"], &r["unitPrice"], &r["priceUnit"]),
        (&json!("01.02.0010"), &json!(12.5), &json!("t"))
    );

    // Kennwerte: das Material des Bereichs steht schon in der Liste, ohne Werte
    let settings = format!("/api/sites/{sid}/billing/settings");
    let s = c.get(&settings).await.json();
    assert_eq!(
        s,
        json!({"truckT": null, "materials": [{"name": "Lehm", "density": null, "bulking": null}], "layers": []})
    );
    let typo = json!({"truckT": 15.0, "materials": [{"name": "Lehm", "density": 2000.0, "bulking": 1.25}]});
    assert_eq!(
        c.json(Method::PUT, &settings, typo).await.status,
        StatusCode::BAD_REQUEST
    );
    let saved = c
        .json(
            Method::PUT,
            &settings,
            json!({"truckT": 15.0, "materials": [{"name": "Lehm", "density": 2.0, "bulking": 1.25}]}),
        )
        .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.json());
    assert_eq!(saved.json()["materials"][0]["density"], 2.0);

    // ohne Aushub-Berechnung: Menge fehlt
    let period = format!("from={}&to={}", ids[0], ids[1]);
    let s = c
        .get(&format!("/api/sites/{sid}/billing?{period}"))
        .await
        .json();
    assert_eq!(s["missing"], json!(["Januar.xyz"]));
    assert_eq!(s["positions"][0]["period"], json!(null));
    let wrong = c
        .get(&format!(
            "/api/sites/{sid}/billing?from={}&to={}",
            ids[1], ids[0]
        ))
        .await;
    assert_eq!(wrong.status, StatusCode::BAD_REQUEST);

    c.json(
        Method::POST,
        &format!("/api/sites/{sid}/jobs"),
        json!({"kind": "volume", "params": {"before": ids[0], "after": ids[1], "regionIds": [rid]}}),
    )
    .await;
    jobs::run_pending(&c.app).await.unwrap();

    // Lieferscheine: einer einzeln, zwei aus CSV; eine fehlerhafte CSV ändert nichts
    let d = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/deliveries"),
            json!({"date": "1970-01-10", "material": "Lehm", "quantity": 900.0, "unit": "t", "reference": "LS-1"}),
        )
        .await;
    assert_eq!(d.status, StatusCode::CREATED, "{}", d.json());
    let csv_url = format!("/api/sites/{sid}/deliveries.csv");
    let bad = c
        .upload(
            &csv_url,
            b"Datum;Material;Menge;Einheit\n20.01.1970;Lehm;3;kg\n".to_vec(),
        )
        .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
    assert!(bad.json()["detail"].as_str().unwrap().contains("Zeile 2"));
    let csv = "Datum;Material;Menge;Einheit;Lieferschein\n\
               20.01.1970;Lehm;250;m³;LS-2\n\
               25.01.1970;Lehm;20;Fahrten;LS-3\n";
    let ok = c.upload(&csv_url, csv.as_bytes().to_vec()).await;
    assert_eq!(ok.json(), json!({"imported": 2}));
    let list = c.get(&format!("/api/sites/{sid}/deliveries")).await.json();
    assert_eq!(list.as_array().unwrap().len(), 3);
    assert_eq!(list[0]["deliveredAt"], 9 * DAY);
    assert_eq!(list[1]["unit"], "m3");

    let s = c
        .get(&format!("/api/sites/{sid}/billing?{period}"))
        .await
        .json();
    assert_eq!(s["missing"], json!([]));
    let p = &s["positions"][0];
    let bank = p["period"]["bankM3"].as_f64().unwrap();
    assert!((bank - 800.0).abs() < 8.0, "{p}");
    // unabhängig nachgerechnet: m³ × 2 t/m³ × 12,50 €/t, Fahrten = ⌈t ÷ 15⌉
    let t = p["period"]["tonnes"].as_f64().unwrap();
    assert!((t - bank * 2.0).abs() < 1e-6);
    assert_eq!(p["period"]["loads"], (t / 15.0).ceil());
    assert!((p["amount"].as_f64().unwrap() - (t * 12.5 * 100.0).round() / 100.0).abs() < 1e-6);
    assert_eq!(s["amount"], p["amount"]);
    // geliefert: 900 t = 450 m³, 250 m³ lose = 200 m³, 20 × 15 t = 150 m³ → 800 m³ fest
    let check = &s["checks"][0];
    assert_eq!(check["deliveredBank"], 800.0);
    assert!(
        check["deviationPercent"].as_f64().unwrap().abs() < 1.0,
        "{check}"
    );

    let pdf = c
        .get(&format!("/api/sites/{sid}/billing.pdf?{period}"))
        .await;
    assert_eq!(pdf.status, StatusCode::OK);
    assert_eq!(pdf.headers["content-type"], "application/pdf");
    assert!(pdf.body.starts_with(b"%PDF"));

    // löschen
    let id = list[0]["id"].as_i64().unwrap();
    assert_eq!(
        c.delete(&format!("/api/deliveries/{id}")).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        c.delete(&format!("/api/deliveries/{id}")).await.status,
        StatusCode::NOT_FOUND
    );
}

/// Schichten: Grube 2 m tief, Oberboden bis 0,5 m → ein Viertel, Lehm darunter drei Viertel
#[tokio::test]
async fn statement_per_layer() {
    let (mut c, sid, ids, rid) = pit_site("Schichten").await;
    let volume = json!({"kind": "volume", "params": {"before": ids[0], "after": ids[1], "regionIds": [rid]}});
    c.json(
        Method::POST,
        &format!("/api/sites/{sid}/jobs"),
        volume.clone(),
    )
    .await;
    jobs::run_pending(&c.app).await.unwrap();

    let settings = format!("/api/sites/{sid}/billing/settings");
    // die unterste Schicht hat keine Unterkante
    let bad = json!({"truckT": null, "materials": [],
        "layers": [{"name": "Oberboden", "bottomM": 0.5}, {"name": "Lehm", "bottomM": 3.0}]});
    assert_eq!(
        c.json(Method::PUT, &settings, bad).await.status,
        StatusCode::BAD_REQUEST
    );
    let layers = json!({"truckT": null, "materials": [], "layers": [
        {"name": "Oberboden", "bottomM": 0.5, "lvPosition": "02.01.0010", "unitPrice": 6.0, "priceUnit": "m3"},
        {"name": "Lehm", "bottomM": null, "lvPosition": "02.01.0020", "unitPrice": 9.0, "priceUnit": "m3"}]});
    let saved = c.json(Method::PUT, &settings, layers).await.json();
    assert_eq!(saved["layers"][1]["name"], "Lehm");
    // jede Schicht ist ein Material für die Kennwerte
    let names: Vec<&str> = saved["materials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Lehm", "Oberboden"]);

    // die Berechnung ist älter als die Schichten
    let url = format!("/api/sites/{sid}/billing?from={}&to={}", ids[0], ids[1]);
    let s = c.get(&url).await.json();
    assert_eq!(s["layersOutdated"], true);
    assert_eq!(s["positions"].as_array().unwrap().len(), 1);

    c.json(Method::POST, &format!("/api/sites/{sid}/jobs"), volume)
        .await;
    jobs::run_pending(&c.app).await.unwrap();
    let s = c.get(&url).await.json();
    assert_eq!(s["layersOutdated"], false);
    let p = s["positions"].as_array().unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(
        (&p[0]["layer"], &p[1]["layer"]),
        (&json!("Oberboden"), &json!("Lehm"))
    );
    let top = p[0]["period"]["bankM3"].as_f64().unwrap();
    let rest = p[1]["period"]["bankM3"].as_f64().unwrap();
    // 400 m² × 0,5 m = 200 m³ und 400 m² × 1,5 m = 600 m³ (Randzellen ±1 %)
    assert!((top - 200.0).abs() < 4.0, "{top}");
    assert!((rest - 600.0).abs() < 8.0, "{rest}");
    let amount = s["amount"].as_f64().unwrap();
    assert!((amount - (top * 6.0 + rest * 9.0)).abs() < 0.02, "{amount}");
}

/// Baustelle mit Urgelände (2.1.1970) und Aushub (1.2.1970), Bereich „Grube“ 20 × 20 m um die 2 m tiefe Grube
async fn pit_site(name: &str) -> (Client, i64, Vec<i64>, i64) {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": name}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let mut ids = Vec::new();
    for (pit, name, at) in [(false, "Urgelände", DAY), (true, "Januar", 31 * DAY)] {
        let r = c
            .upload(
                &format!("/api/sites/{sid}/captures?file={name}.xyz&capturedAt={at}"),
                terrain_xyz(pit),
            )
            .await;
        ids.push(r.json()["id"].as_i64().unwrap());
    }
    jobs::run_pending(&c.app).await.unwrap();
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
    (c, sid, ids, rid)
}

/// Freigabe: Link ohne Anmeldung, eingefrorenes Aufmaß, genau eine Entscheidung, zurückziehen
#[tokio::test]
async fn share_for_approval() {
    let (mut c, sid, ids, rid) = pit_site("Freigabe").await;
    c.json(
        Method::POST,
        &format!("/api/sites/{sid}/jobs"),
        json!({"kind": "volume", "params": {"before": ids[0], "after": ids[1], "regionIds": [rid]}}),
    )
    .await;
    jobs::run_pending(&c.app).await.unwrap();
    c.json(
        Method::PATCH,
        &format!("/api/regions/{rid}"),
        json!({"unitPrice": 10.0, "priceUnit": "m3"}),
    )
    .await;
    let created = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/billing/shares"),
            json!({"from": ids[0], "to": ids[1], "note": "Abschlag 1"}),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.json());
    let token = created.json()["token"].as_str().unwrap().to_string();
    let amount = created.json()["amount"].as_f64().unwrap();
    assert!((amount - 8000.0).abs() < 80.0, "{amount}");
    // die Liste zeigt das Token nicht
    let list = c
        .get(&format!("/api/sites/{sid}/billing/shares"))
        .await
        .json();
    assert_eq!(list[0]["note"], "Abschlag 1");
    assert!(list[0].get("token").is_none());
    assert_eq!(list[0]["decision"], json!(null));

    // eingefroren: ein späterer Preis ändert die Freigabe nicht
    c.json(
        Method::PATCH,
        &format!("/api/regions/{rid}"),
        json!({"unitPrice": 99.0, "priceUnit": "m3"}),
    )
    .await;

    // ohne Anmeldung
    let mut guest = c.other();
    let view = guest.get(&format!("/api/public/shares/{token}")).await;
    assert_eq!(view.status, StatusCode::OK);
    assert_eq!(view.json()["site"], "Freigabe");
    assert_eq!(view.json()["statement"]["amount"], amount);
    assert_eq!(
        guest.get("/api/public/shares/falsch").await.status,
        StatusCode::NOT_FOUND
    );
    // die Baustelle selbst bleibt geschützt
    assert_eq!(
        guest
            .get(&format!("/api/sites/{sid}/billing/shares"))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let decide = format!("/api/public/shares/{token}/decision");
    let no_reason = guest
        .json(
            Method::POST,
            &decide,
            json!({"decision": "objected", "name": "M. Muster"}),
        )
        .await;
    assert_eq!(no_reason.status, StatusCode::BAD_REQUEST);
    let ok = guest
        .json(
            Method::POST,
            &decide,
            json!({"decision": "approved", "name": " M. Muster ", "organization": "Bau GmbH"}),
        )
        .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.json());
    assert_eq!(ok.json()["decision"]["name"], "M. Muster");
    let again = guest
        .json(
            Method::POST,
            &decide,
            json!({"decision": "objected", "name": "X", "comment": "doch nicht"}),
        )
        .await;
    assert_eq!(again.status, StatusCode::CONFLICT);
    assert_eq!(again.json()["error"], "shares.decided");
    let pdf = guest.get(&format!("/api/public/shares/{token}/pdf")).await;
    assert_eq!(pdf.headers["content-type"], "application/pdf");

    let list = c
        .get(&format!("/api/sites/{sid}/billing/shares"))
        .await
        .json();
    assert_eq!(list[0]["decision"]["decision"], "approved");
    assert_eq!(list[0]["decision"]["organization"], "Bau GmbH");
    let id = list[0]["id"].as_i64().unwrap();
    assert_eq!(
        c.get(&format!("/api/billing/shares/{id}/pdf")).await.status,
        StatusCode::OK
    );
    // zurückziehen: der Link öffnet nichts mehr, die Entscheidung bleibt
    assert_eq!(
        c.delete(&format!("/api/billing/shares/{id}")).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        guest
            .get(&format!("/api/public/shares/{token}"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let list = c
        .get(&format!("/api/sites/{sid}/billing/shares"))
        .await
        .json();
    assert!(list[0]["revokedAt"].is_number());
    assert_eq!(list[0]["decision"]["decision"], "approved");
}
