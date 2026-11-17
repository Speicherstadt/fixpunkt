//! Integrationstests der API: echter Router, echte SQLite-Datenbank und Rechenkern in einem
//! temporären Verzeichnis – ohne Netzwerk. Aufträge werden mit `run_pending` deterministisch abgearbeitet.

mod common;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use common::{rect, terrain_xyz, Client, X0, Y0};
use fixpunkt_server::config::Config;
use fixpunkt_server::{jobs, router};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn health_and_first_setup() {
    let mut c = Client::new().await;
    let h = c.get("/api/health").await;
    assert_eq!(h.status, StatusCode::OK);
    assert_eq!(h.json()["status"], "ok");

    let st = c.get("/api/auth/state").await.json();
    assert_eq!(st["setupRequired"], true);
    assert!(st["user"].is_null());

    assert_eq!(c.get("/api/sites").await.status, StatusCode::UNAUTHORIZED);
    let short = c
        .json(
            Method::POST,
            "/api/auth/setup",
            json!({"name": "admin", "password": "kurz"}),
        )
        .await;
    assert_eq!(short.json()["error"], "auth.passwordTooShort");

    c.setup_admin().await;
    let st = c.get("/api/auth/state").await.json();
    assert_eq!(st["setupRequired"], false);
    assert_eq!(st["user"]["name"], "admin");
    assert_eq!(st["user"]["role"], "admin");
    assert_eq!(st["csrf"].as_str().unwrap().len(), 64);

    let again = c
        .other()
        .json(
            Method::POST,
            "/api/auth/setup",
            json!({"name": "x", "password": "geheim123"}),
        )
        .await;
    assert_eq!(again.status, StatusCode::CONFLICT);

    // Abmelden beendet die Sitzung
    assert_eq!(
        c.json(Method::POST, "/api/auth/logout", json!({}))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(c.get("/api/sites").await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(c.login("admin", "falsch!!").await, StatusCode::UNAUTHORIZED);
    assert_eq!(
        c.login("ADMIN", "geheim123").await,
        StatusCode::NO_CONTENT,
        "Name ohne Groß-/Kleinschreibung"
    );
    assert_eq!(c.get("/api/sites").await.status, StatusCode::OK);
}

#[tokio::test]
async fn csrf_and_roles() {
    let mut admin = Client::new().await;
    admin.setup_admin().await;

    // ändernde Anfrage ohne CSRF-Token
    let r = admin
        .send(
            Method::POST,
            "/api/sites",
            Body::from(r#"{"name":"A"}"#),
            Some("application/json"),
            false,
        )
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.json()["error"], "auth.csrf");

    for (name, role) in [("leser", "viewer"), ("bauleiter", "editor")] {
        let r = admin
            .json(
                Method::POST,
                "/api/users",
                json!({"name": name, "password": "passwort1", "role": role}),
            )
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.json());
    }
    let dup = admin
        .json(
            Method::POST,
            "/api/users",
            json!({"name": "Leser", "password": "passwort1", "role": "viewer"}),
        )
        .await;
    assert_eq!(dup.json()["error"], "auth.nameTaken");
    let site = admin
        .json(Method::POST, "/api/sites", json!({"name": "Baugrube Nord"}))
        .await
        .json();

    let mut viewer = admin.other();
    assert_eq!(
        viewer.login("leser", "passwort1").await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        viewer
            .get("/api/sites")
            .await
            .json()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let r = viewer
        .json(Method::POST, "/api/sites", json!({"name": "B"}))
        .await;
    assert_eq!(
        (r.status, r.json()["error"].as_str()),
        (StatusCode::FORBIDDEN, Some("auth.forbidden"))
    );
    assert_eq!(viewer.get("/api/users").await.status, StatusCode::FORBIDDEN);

    let mut editor = admin.other();
    editor.login("bauleiter", "passwort1").await;
    let id = site["id"].as_i64().unwrap();
    let r = editor
        .json(
            Method::PATCH,
            &format!("/api/sites/{id}"),
            json!({"description": "Aushub bis -4 m"}),
        )
        .await;
    assert_eq!(r.json()["description"], "Aushub bis -4 m");
    assert_eq!(editor.get("/api/users").await.status, StatusCode::FORBIDDEN);

    // Passwort ändern
    let r = editor
        .json(
            Method::POST,
            "/api/auth/password",
            json!({"current": "falsch", "new": "neuespasswort"}),
        )
        .await;
    assert_eq!(r.json()["error"], "auth.wrongPassword");
    let r = editor
        .json(
            Method::POST,
            "/api/auth/password",
            json!({"current": "passwort1", "new": "neuespasswort"}),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(
        editor.other().login("bauleiter", "neuespasswort").await,
        StatusCode::NO_CONTENT
    );

    // der letzte Admin bleibt
    let users = admin.get("/api/users").await.json();
    let admin_id = users
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["role"] == "admin")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    assert!(
        users[0].get("passwordHash").is_none(),
        "kein Hash in der Antwort"
    );
    assert_eq!(
        admin.delete(&format!("/api/users/{admin_id}")).await.json()["error"],
        "auth.lastAdmin"
    );
}

#[tokio::test]
async fn excavation_volume_end_to_end() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let site = c
        .json(
            Method::POST,
            "/api/sites",
            json!({"name": "Baugrube Kirchstraße"}),
        )
        .await;
    assert_eq!(site.status, StatusCode::CREATED);
    let sid = site.json()["id"].as_i64().unwrap();

    // Aufnahmen hochladen
    let before = c
        .upload(
            &format!("/api/sites/{sid}/captures?file=vorher.xyz&name=Urgel%C3%A4nde"),
            terrain_xyz(false),
        )
        .await;
    assert_eq!(before.status, StatusCode::CREATED, "{}", before.json());
    assert_eq!(before.json()["status"], "uploaded");
    let after = c
        .upload(
            &format!("/api/sites/{sid}/captures?file=nachher.xyz"),
            terrain_xyz(true),
        )
        .await;
    let (b, a) = (
        before.json()["id"].as_i64().unwrap(),
        after.json()["id"].as_i64().unwrap(),
    );

    // vor der Prüfung kann noch nicht gerechnet werden
    let early = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "volume", "params": {"before": b, "after": a}}),
        )
        .await;
    assert_eq!(early.json()["error"], "jobs.captureNotReady");

    assert_eq!(jobs::run_pending(&c.app).await.unwrap(), 2);
    let caps = c.get(&format!("/api/sites/{sid}/captures")).await.json();
    assert_eq!(caps[0]["name"], "Urgelände");
    assert_eq!(caps[0]["status"], "ready");
    assert_eq!(caps[0]["info"]["points"], 40_000);
    assert_eq!(caps[1]["name"], "nachher.xyz");

    // falsches Format, leere Datei
    let bad = c
        .upload(
            &format!("/api/sites/{sid}/captures?file=foto.jpg"),
            vec![1, 2, 3],
        )
        .await;
    assert_eq!(bad.json()["error"], "captures.unsupportedFile");
    let empty = c
        .upload(&format!("/api/sites/{sid}/captures?file=leer.las"), vec![])
        .await;
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        c.get(&format!("/api/sites/{sid}/captures"))
            .await
            .json()
            .as_array()
            .unwrap()
            .len(),
        2
    );

    // Bereiche: zwei Polygone in einem metrischen GeoJSON
    let fc = json!({"type": "FeatureCollection",
        "crs": {"type": "name", "properties": {"name": "urn:ogc:def:crs:EPSG::25832"}},
        "features": [rect("Grube", 5.0, 5.0, 35.0, 35.0), rect("Rand", 36.0, 2.0, 39.0, 38.0)]});
    let regs = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/regions"),
            json!({"geojson": fc}),
        )
        .await;
    assert_eq!(regs.status, StatusCode::CREATED, "{}", regs.json());
    let regs = regs.json();
    assert_eq!(regs.as_array().unwrap().len(), 2);
    assert!((regs[0]["areaM2"].as_f64().unwrap() - 900.0).abs() < 1e-6);
    let bad = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/regions"),
            json!({"geojson": {"type": "Point"}}),
        )
        .await;
    assert_eq!(bad.json()["error"], "regions.invalidGeojson");
    let region_ids: Vec<i64> = regs
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_i64().unwrap())
        .collect();

    // Aushub berechnen
    let job = c
        .json(Method::POST, &format!("/api/sites/{sid}/jobs"), json!({"kind": "volume",
            "params": {"before": b, "after": a, "regionIds": region_ids, "surface": {"cell": 0.25}}}))
        .await;
    assert_eq!(job.status, StatusCode::CREATED, "{}", job.json());
    let jid = job.json()["id"].as_i64().unwrap();
    assert_eq!(job.json()["status"], "queued");
    assert_eq!(jobs::run_pending(&c.app).await.unwrap(), 1);

    let done = c.get(&format!("/api/jobs/{jid}")).await.json();
    assert_eq!(done["status"], "done", "{done}");
    let r = &done["result"];
    // 20 × 20 × 2 m = 800 m³ in der Grube, nichts am Rand
    let cut = r["regions"][0]["cut_m3"].as_f64().unwrap();
    assert!((cut - 800.0).abs() < 8.0, "{r}");
    assert_eq!(r["regions"][0]["name"], "Grube");
    assert!(r["regions"][1]["cut_m3"].as_f64().unwrap() < 1.0);
    assert_eq!(
        c.get(&format!("/api/sites/{sid}/jobs"))
            .await
            .json()
            .as_array()
            .unwrap()
            .len(),
        1,
        "Prüfaufträge nicht in der Liste"
    );

    // Differenzraster herunterladen; nur gelistete Dateien
    let tif = c.get(&format!("/api/jobs/{jid}/files/diff.tif")).await;
    assert_eq!(tif.status, StatusCode::OK);
    assert!(
        tif.body.starts_with(b"II*\0") || tif.body.starts_with(b"MM\0*"),
        "TIFF"
    );
    assert_eq!(
        c.get(&format!("/api/jobs/{jid}/files/..%2F..%2Ffixpunkt.db"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        c.get(&format!("/api/jobs/{jid}/files/dem.tif"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );

    // Original herunterladen
    let orig = c.get(&format!("/api/captures/{b}/file")).await;
    assert_eq!(orig.body, terrain_xyz(false));
    assert!(orig.headers[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .contains("vorher.xyz"));

    // Haufwerk und DGM
    let sp = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "stockpile",
            "params": {"captureId": b, "regionIds": [region_ids[0]], "base": 299.0}}),
        )
        .await;
    assert_eq!(sp.status, StatusCode::CREATED, "{}", sp.json());
    let dem = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "dem", "params": {"captureId": a, "surface": {"cell": 1.0}}}),
        )
        .await;
    jobs::run_pending(&c.app).await.unwrap();
    let sp = c
        .get(&format!("/api/jobs/{}", sp.json()["id"]))
        .await
        .json();
    // 30 × 30 m, im Mittel 1,2 m über 299 (Gefälle 1 %: 300,05 … 300,35)
    let fill = sp["result"]["total"]["fill_m3"].as_f64().unwrap();
    assert!((fill - 900.0 * 1.2).abs() < 10.0, "{sp}");
    let dem = c
        .get(&format!("/api/jobs/{}", dem.json()["id"]))
        .await
        .json();
    assert_eq!(dem["result"]["cols"], 40);
    assert_eq!(
        c.get(&format!("/api/jobs/{}/files/dem.tif", dem["id"]))
            .await
            .status,
        StatusCode::OK
    );

    // ungültige Aufträge
    let unknown = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "zaubern"}),
        )
        .await;
    assert_eq!(unknown.json()["error"], "jobs.unknownKind");
    let other = c
        .json(Method::POST, "/api/sites", json!({"name": "Andere"}))
        .await
        .json();
    let wrong = c
        .json(
            Method::POST,
            &format!("/api/sites/{}/jobs", other["id"]),
            json!({"kind": "dem", "params": {"captureId": a}}),
        )
        .await;
    assert_eq!(wrong.json()["error"], "jobs.wrongSite");
    let bad_cell = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "dem", "params": {"captureId": a, "surface": {"cell": 0}}}),
        )
        .await;
    assert_eq!(bad_cell.json()["error"], "api.invalidInput");

    // Baustelle löschen räumt Dateien auf
    let list = c.get("/api/sites").await.json();
    let item = list
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == sid)
        .unwrap()
        .clone();
    assert_eq!(item["captures"], 2);
    assert_eq!(item["jobs"], 3);
    assert_eq!(
        c.delete(&format!("/api/sites/{sid}")).await.status,
        StatusCode::NO_CONTENT
    );
    assert!(!c.app.cfg.capture_dir(b).exists());
    assert!(!c.app.cfg.job_dir(jid).exists());
    assert_eq!(
        c.get(&format!("/api/sites/{sid}")).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn failed_capture_and_failed_job() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "A"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let r = c
        .upload(
            &format!("/api/sites/{sid}/captures?file=kaputt.laz"),
            b"kein laz".to_vec(),
        )
        .await;
    let id = r.json()["id"].as_i64().unwrap();
    jobs::run_pending(&c.app).await.unwrap();
    let cap = &c.get(&format!("/api/sites/{sid}/captures")).await.json()[0];
    assert_eq!(cap["status"], "failed");
    assert!(
        cap["error"].as_str().unwrap().contains("kaputt.laz"),
        "{cap}"
    );

    // Auftrag, dessen Datei inzwischen fehlt, schlägt mit Meldung fehl
    let ok = c
        .upload(
            &format!("/api/sites/{sid}/captures?file=a.xyz"),
            terrain_xyz(false),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    jobs::run_pending(&c.app).await.unwrap();
    let job = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "dem", "params": {"captureId": ok}}),
        )
        .await
        .json();
    std::fs::remove_dir_all(c.app.cfg.capture_dir(ok)).unwrap();
    jobs::run_pending(&c.app).await.unwrap();
    let job = c.get(&format!("/api/jobs/{}", job["id"])).await.json();
    assert_eq!(job["status"], "failed");
    assert!(job["error"].as_str().is_some_and(|e| !e.is_empty()));
    assert_eq!(
        c.delete(&format!("/api/captures/{id}")).await.status,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn interrupted_jobs_are_requeued_on_start() {
    let dir = tempfile::tempdir().unwrap();
    let app = fixpunkt_server::init(Config::for_dir(dir.path()))
        .await
        .unwrap();
    sqlx::query("INSERT INTO sites(name, created_at) VALUES('A', 0)")
        .execute(&app.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO jobs(site_id, kind, params, status, created_at, started_at) VALUES(1, 'dem', '{}', 'running', 0, 1)")
        .execute(&app.db)
        .await
        .unwrap();
    drop(app);
    let app = fixpunkt_server::init(Config::for_dir(dir.path()))
        .await
        .unwrap();
    let st: String = sqlx::query_scalar("SELECT status FROM jobs WHERE id = 1")
        .fetch_one(&app.db)
        .await
        .unwrap();
    assert_eq!(st, "queued");
}

#[tokio::test]
async fn initial_user_from_env_and_ui_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = Config::for_dir(dir.path());
    cfg.initial_user = Some(("chef".into(), "baustelle1".into()));
    let app = fixpunkt_server::init(cfg).await.unwrap();
    let mut c = Client {
        app,
        cookie: None,
        csrf: None,
        _dir: dir,
    };
    assert_eq!(
        c.get("/api/auth/state").await.json()["setupRequired"],
        false
    );
    assert_eq!(c.login("chef", "baustelle1").await, StatusCode::NO_CONTENT);

    // Live-Ereignisse
    let ev = c.get_stream_headers("/api/events").await;
    assert_eq!(ev, "text/event-stream");

    // unbekannte API-Pfade sind 404, nicht die UI
    assert_eq!(c.get("/api/gibtsnicht").await.status, StatusCode::NOT_FOUND);
}

impl Client {
    /// Nur den Content-Type einer Streaming-Antwort (ohne den endlosen Body zu lesen)
    async fn get_stream_headers(&mut self, uri: &str) -> String {
        let req = Request::builder()
            .uri(uri)
            .header(header::COOKIE, self.cookie.clone().unwrap())
            .body(Body::empty())
            .unwrap();
        let res = router(self.app.clone()).oneshot(req).await.unwrap();
        res.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .to_string()
    }
}

#[tokio::test]
async fn compare_series_and_export() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Grube Süd"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let up = |pit: bool, name: &'static str, at: i64| (pit, name, at);
    let mut ids = Vec::new();
    for (pit, name, at) in [up(false, "Urgelände", 1), up(true, "Aushub", 2)] {
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
        "features": [rect("Sohle", 12.0, 12.0, 28.0, 28.0)]});
    let region = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/regions"),
            json!({"geojson": fc}),
        )
        .await
        .json()[0]["id"]
        .as_i64()
        .unwrap();
    let start = |kind: &str, params: Value| (kind.to_string(), params);
    let specs = [
        // Sohle soll bei 298,10 liegen; Ist 298,0 + 1 % Gefälle → im Mittel 0,1 m zu hoch
        start(
            "compare",
            json!({"captureId": ids[1], "regionId": region, "tolerance": 0.05,
            "design": {"type": "height", "height": 298.1}}),
        ),
        start("series", json!({"captureIds": ids, "regionId": region})),
        start(
            "export",
            json!({"captureId": ids[1], "formats": ["tif", "dxf", "landxml", "csv"], "regionIds": [region]}),
        ),
    ];
    let mut jobs_ = Vec::new();
    for (kind, params) in specs {
        let r = c
            .json(
                Method::POST,
                &format!("/api/sites/{sid}/jobs"),
                json!({"kind": kind, "params": params}),
            )
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{kind}: {}", r.json());
        jobs_.push(r.json()["id"].as_i64().unwrap());
    }
    jobs::run_pending(&c.app).await.unwrap();
    let get = |id: i64| format!("/api/jobs/{id}");

    let cmp = c.get(&get(jobs_[0])).await.json();
    assert_eq!(cmp["status"], "done", "{cmp}");
    let d = &cmp["result"]["deviation"];
    // Ist = 298,0 + 0,01·dx, dx 12…28 → Mittel 298,2; gegen 298,1 also +0,1 m
    assert!((d["mean"].as_f64().unwrap() - 0.1).abs() < 0.01, "{cmp}");
    // Abweichung 0,01·dx − 0,1 liegt über 5 cm, sobald dx > 15 m: 13 von 16 m = 81,25 %
    let above = d["above_tolerance_percent"].as_f64().unwrap();
    assert!((above - 81.25).abs() < 3.0, "{above}");
    let tif = c
        .get(&format!("/api/jobs/{}/files/abweichung.tif", jobs_[0]))
        .await;
    assert_eq!(tif.headers[header::CONTENT_TYPE], "image/tiff");

    let ser = c.get(&get(jobs_[1])).await.json();
    assert_eq!(
        ser["result"]["base"], "Urgelände.xyz",
        "ohne Namen heißt die Aufnahme nach der Datei"
    );
    // die Sohle (16 × 16 m) liegt ganz in der Grube: 256 m² × 2 m
    let cut = ser["result"]["steps"][0]["since_start"]["cut_m3"]
        .as_f64()
        .unwrap();
    assert!((cut - 512.0).abs() < 6.0, "{ser}");

    let exp = c.get(&get(jobs_[2])).await.json();
    assert_eq!(
        exp["result"]["files"],
        json!(["dgm.tif", "dgm.dxf", "dgm.xml", "dgm.csv"]),
        "{exp}"
    );
    let xml = c
        .get(&format!("/api/jobs/{}/files/dgm.xml", jobs_[2]))
        .await;
    assert_eq!(xml.headers[header::CONTENT_TYPE], "application/xml");
    assert!(String::from_utf8_lossy(&xml.body).contains("<LandXML"));
    let csv = c
        .get(&format!("/api/jobs/{}/files/dgm.csv", jobs_[2]))
        .await;
    assert!(String::from_utf8_lossy(&csv.body).starts_with("x;y;z"));

    // ungültige Parameter werden sofort abgelehnt
    let bad = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "series", "params": {"captureIds": [ids[0]]}}),
        )
        .await;
    assert_eq!(bad.json()["error"], "api.invalidInput");
    let bad = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "export", "params": {"captureId": ids[0], "formats": ["pdf"]}}),
        )
        .await;
    assert_eq!(bad.json()["error"], "api.invalidInput");
}

#[tokio::test]
async fn grids_for_the_2d_view() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(Method::POST, "/api/sites", json!({"name": "Ansicht"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let a = c
        .upload(
            &format!("/api/sites/{sid}/captures?file=a.xyz"),
            terrain_xyz(false),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let b = c
        .upload(
            &format!("/api/sites/{sid}/captures?file=b.xyz"),
            terrain_xyz(true),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    jobs::run_pending(&c.app).await.unwrap();

    let g = c.get(&format!("/api/captures/{a}/grid")).await;
    assert_eq!(g.status, StatusCode::OK);
    assert_eq!(
        g.headers[header::CONTENT_TYPE],
        "application/x-fixpunkt-grid"
    );
    let num = |k: &str| g.headers[k].to_str().unwrap().parse::<f64>().unwrap();
    let (cols, rows) = (num("x-grid-cols") as usize, num("x-grid-rows") as usize);
    assert_eq!((cols, rows, num("x-grid-cell")), (160, 160, 0.25));
    assert_eq!(g.body.len(), cols * rows * 4);
    assert_eq!(num("x-grid-min-x"), X0);
    // Mitte: 300 + 0,01 · 20 = 300,2
    let i = (rows / 2 * cols + cols / 2) * 4;
    let z = f32::from_le_bytes(g.body[i..i + 4].try_into().unwrap());
    assert!((z - 300.2).abs() < 0.01, "{z}");
    assert!(
        c.app.cfg.capture_dir(a).join("ansicht.f32").exists(),
        "zwischengespeichert"
    );

    let job = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/jobs"),
            json!({"kind": "volume", "params": {"before": a, "after": b}}),
        )
        .await;
    let jid = job.json()["id"].as_i64().unwrap();
    jobs::run_pending(&c.app).await.unwrap();
    let d = c.get(&format!("/api/jobs/{jid}/grid/diff.tif")).await;
    assert_eq!(d.status, StatusCode::OK);
    let i = (rows / 2 * cols + cols / 2) * 4;
    let dz = f32::from_le_bytes(d.body[i..i + 4].try_into().unwrap());
    assert!((dz + 2.0).abs() < 0.01, "Grube 2 m tief: {dz}");
    assert_eq!(
        c.get(&format!("/api/jobs/{jid}/grid/..%2Ffixpunkt.db"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        c.get("/api/captures/999/grid").await.status,
        StatusCode::NOT_FOUND
    );
}

/// Text aller Seiten eines PDFs, unabhängig vom Erzeuger gelesen (lopdf)
fn pdf_text(bytes: &[u8]) -> (usize, String) {
    let pdf = lopdf::Document::load_mem(bytes)
        .unwrap_or_else(|e| panic!("kein PDF ({e}): {}", String::from_utf8_lossy(bytes)));
    let pages: Vec<u32> = pdf.get_pages().keys().copied().collect();
    (pages.len(), pdf.extract_text(&pages).expect("Text lesbar"))
}

#[tokio::test]
async fn pdf_reports_for_each_kind() {
    use fixpunkt_server::report::text;
    let mut c = Client::new().await;
    c.setup_admin().await;
    let sid = c
        .json(
            Method::POST,
            "/api/sites",
            json!({"name": "Kirchstraße 12"}),
        )
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let mut ids = Vec::new();
    for (pit, name) in [(false, "Urgelände"), (true, "Aushub")] {
        let r = c
            .upload(
                &format!("/api/sites/{sid}/captures?file={name}.xyz"),
                terrain_xyz(pit),
            )
            .await;
        ids.push(r.json()["id"].as_i64().unwrap());
    }
    jobs::run_pending(&c.app).await.unwrap();
    let fc =
        json!({"type": "FeatureCollection", "features": [rect("Grube", 5.0, 5.0, 35.0, 35.0)]});
    let region = c
        .json(
            Method::POST,
            &format!("/api/sites/{sid}/regions"),
            json!({"geojson": fc}),
        )
        .await
        .json()[0]["id"]
        .as_i64()
        .unwrap();
    let job = |kind: &str, params: Value| (kind.to_string(), params);
    let specs = [
        job(
            "volume",
            json!({"before": ids[0], "after": ids[1], "regionIds": [region]}),
        ),
        job(
            "stockpile",
            json!({"captureId": ids[0], "regionIds": [region], "base": 299.0}),
        ),
        job(
            "compare",
            json!({"captureId": ids[1], "regionId": region, "tolerance": 0.05,
            "design": {"type": "height", "height": 298.0}}),
        ),
        job("series", json!({"captureIds": ids})),
        job("dem", json!({"captureId": ids[0]})),
    ];
    let mut jids = Vec::new();
    for (kind, params) in specs {
        let r = c
            .json(
                Method::POST,
                &format!("/api/sites/{sid}/jobs"),
                json!({"kind": kind, "params": params}),
            )
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.json());
        jids.push(r.json()["id"].as_i64().unwrap());
    }
    // vor der Berechnung gibt es keinen Bericht
    assert_eq!(
        c.get(&format!("/api/jobs/{}/report", jids[0])).await.status,
        StatusCode::NOT_FOUND
    );
    jobs::run_pending(&c.app).await.unwrap();

    // Aushub: Grube 20 × 20 × 2 m = 800 m³, Zahl wie im Ergebnis formatiert, Karte eingebettet
    let res = c.get(&format!("/api/jobs/{}", jids[0])).await.json();
    let cut = res["result"]["total"]["cut_m3"].as_f64().unwrap();
    assert!((cut - 800.0).abs() < 2.0, "{cut}");
    let r = c.get(&format!("/api/jobs/{}/report", jids[0])).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.headers[header::CONTENT_TYPE], "application/pdf");
    assert_eq!(
        r.headers[header::CONTENT_DISPOSITION],
        format!(
            "attachment; filename=\"Mengenbericht-Kirchstrasse-12-{}.pdf\"",
            jids[0]
        )
        .as_str()
    );
    let (pages, t) = pdf_text(&r.body);
    assert!(pages >= 1);
    for s in [
        "Mengenbericht",
        "Urgelände.xyz » Aushub.xyz",
        "Kirchstraße 12",
        "Grube",
        "ETRS89/UTM32",
    ] {
        assert!(t.contains(s), "{s} fehlt: {t}");
    }
    assert!(
        t.contains(&text::volume(cut)),
        "{} fehlt: {t}",
        text::volume(cut)
    );
    assert!(t.contains("Höhenänderung"), "Karte fehlt: {t}");

    // Haufwerk über 299 m: 30 × 30 m Gelände 300,05…300,35 m → ~1,2 m × 900 m² ≈ 1.080 m³
    let (_, t) = pdf_text(&c.get(&format!("/api/jobs/{}/report", jids[1])).await.body);
    assert!(t.contains("Haufwerk") && t.contains("Höhe 299,00 m"), "{t}");
    assert!(
        t.contains("1.079,") || t.contains("1.080,"),
        "≈ 1.080 m³: {t}"
    );

    // Soll/Ist gegen 298 m: Sohle 298,1–298,3 m (10–30 cm zu hoch), Rand 2 m zu hoch → alles zu hoch;
    // noch abzutragen = 900 m² × mittlere Abweichung (Mittel × Fläche, unabhängig gerechnet)
    let res = c.get(&format!("/api/jobs/{}", jids[2])).await.json();
    let d = &res["result"]["deviation"];
    let rest = d["mean"].as_f64().unwrap() * 900.0;
    assert!((d["volume"]["fill_m3"].as_f64().unwrap() - rest).abs() < 1.0);
    let (_, t) = pdf_text(&c.get(&format!("/api/jobs/{}/report", jids[2])).await.body);
    assert!(
        t.contains("Soll/Ist-Vergleich") && t.contains("Sohlhöhe 298,00 m"),
        "{t}"
    );
    assert!(
        t.contains("100,0 %") && t.contains("Noch abzutragen"),
        "{t}"
    );
    let fill = d["volume"]["fill_m3"].as_f64().unwrap();
    assert!(t.contains(&text::volume(fill)), "{t}");

    let (_, t) = pdf_text(&c.get(&format!("/api/jobs/{}/report", jids[3])).await.body);
    assert!(t.contains("Zeitreihe") && t.contains("Aushub"), "{t}");

    // Höhenmodell hat keinen Bericht, unbekannte Aufträge auch nicht
    assert_eq!(
        c.get(&format!("/api/jobs/{}/report", jids[4])).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        c.get("/api/jobs/999/report").await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn sites_have_a_location_for_the_map() {
    use fixpunkt::crs::Utm;
    let mut c = Client::new().await;
    c.setup_admin().await;
    let s = |c: &Value, id: i64| {
        c.as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .unwrap()
            .clone()
    };
    let a = c
        .json(Method::POST, "/api/sites", json!({"name": "Mit Aufnahme"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let b = c
        .json(Method::POST, "/api/sites", json!({"name": "Nur Bereiche"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    let empty = c
        .json(Method::POST, "/api/sites", json!({"name": "Leer"}))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    c.upload(
        &format!("/api/sites/{a}/captures?file=a.xyz"),
        terrain_xyz(false),
    )
    .await;
    let fc = json!({"type": "FeatureCollection",
        "features": [rect("West", 0.0, 0.0, 10.0, 10.0), rect("Ost", 90.0, 0.0, 100.0, 10.0)]});
    c.json(
        Method::POST,
        &format!("/api/sites/{b}/regions"),
        json!({"geojson": fc}),
    )
    .await;
    // vor der Prüfung: Aufnahme zählt noch nicht
    let list = c.get("/api/sites").await.json();
    assert!(s(&list, a)["center"].is_null());
    jobs::run_pending(&c.app).await.unwrap();
    let list = c.get("/api/sites").await.json();

    // unabhängig geprüft: Breite/Länge zurück nach UTM32 ergibt die Mitte des Geländes
    let back = |site: &Value| {
        let ll = site["center"].as_array().unwrap();
        Utm::new(32)
            .unwrap()
            .forward(ll[0].as_f64().unwrap(), ll[1].as_f64().unwrap())
    };
    let (e, n) = back(&s(&list, a));
    assert!(
        (e - (X0 + 20.0)).abs() < 1e-3 && (n - (Y0 + 20.0)).abs() < 1e-3,
        "{e} {n}"
    );
    // beide Bereiche zusammen: 0 … 100 m → Mitte bei 50 m
    let (e, n) = back(&s(&list, b));
    assert!(
        (e - (X0 + 50.0)).abs() < 1e-3 && (n - (Y0 + 5.0)).abs() < 1e-3,
        "{e} {n}"
    );
    assert!(s(&list, empty)["center"].is_null());
    // Stuttgarter Gegend
    let ll = s(&list, a)["center"].clone();
    assert!((ll[0].as_f64().unwrap() - 48.78).abs() < 0.05, "{ll}");
    assert!((ll[1].as_f64().unwrap() - 9.16).abs() < 0.05, "{ll}");

    // Testkonfiguration ohne Kartenhintergrund
    assert_eq!(
        c.get("/api/settings").await.json(),
        json!({"mapTiles": null})
    );
}
