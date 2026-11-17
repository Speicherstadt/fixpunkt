//! Integrationstests der API: echter Router, echte SQLite-Datenbank und Rechenkern in einem
//! temporären Verzeichnis – ohne Netzwerk. Aufträge werden mit `run_pending` deterministisch abgearbeitet.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use fixpunkt_server::config::Config;
use fixpunkt_server::{jobs, router, state::AppRef};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const X0: f64 = 512_000.0;
const Y0: f64 = 5_403_000.0;

struct Client {
    app: AppRef,
    cookie: Option<String>,
    csrf: Option<String>,
    _dir: tempfile::TempDir,
}

struct Res {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: Vec<u8>,
}

impl Res {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }
}

impl Client {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let app = fixpunkt_server::init(Config::for_dir(dir.path()))
            .await
            .unwrap();
        Client {
            app,
            cookie: None,
            csrf: None,
            _dir: dir,
        }
    }

    /// Zweite Sitzung auf derselben Instanz (anderer Benutzer)
    fn other(&self) -> Client {
        Client {
            app: self.app.clone(),
            cookie: None,
            csrf: None,
            _dir: tempfile::tempdir().unwrap(),
        }
    }

    async fn send(
        &mut self,
        method: Method,
        uri: &str,
        body: Body,
        content_type: Option<&str>,
        csrf: bool,
    ) -> Res {
        let mut req = Request::builder().method(method).uri(uri);
        if let Some(ct) = content_type {
            req = req.header(header::CONTENT_TYPE, ct);
        }
        if let Some(c) = &self.cookie {
            req = req.header(header::COOKIE, c);
        }
        if let (true, Some(t)) = (csrf, &self.csrf) {
            req = req.header("x-csrf-token", t);
        }
        let res = router(self.app.clone())
            .oneshot(req.body(body).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        if let Some(sc) = headers
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
        {
            let pair = sc.split(';').next().unwrap().to_string();
            self.cookie = (!pair.ends_with('=')).then_some(pair);
        }
        let body = res.into_body().collect().await.unwrap().to_bytes().to_vec();
        Res {
            status,
            headers,
            body,
        }
    }

    async fn get(&mut self, uri: &str) -> Res {
        self.send(Method::GET, uri, Body::empty(), None, false)
            .await
    }

    async fn json(&mut self, method: Method, uri: &str, v: Value) -> Res {
        self.send(
            method,
            uri,
            Body::from(v.to_string()),
            Some("application/json"),
            true,
        )
        .await
    }

    async fn delete(&mut self, uri: &str) -> Res {
        self.send(Method::DELETE, uri, Body::empty(), None, true)
            .await
    }

    async fn upload(&mut self, uri: &str, data: Vec<u8>) -> Res {
        self.send(
            Method::PUT,
            uri,
            Body::from(data),
            Some("application/octet-stream"),
            true,
        )
        .await
    }

    async fn refresh_csrf(&mut self) {
        self.csrf = self.get("/api/auth/state").await.json()["csrf"]
            .as_str()
            .map(String::from);
    }

    async fn setup_admin(&mut self) {
        let r = self
            .json(
                Method::POST,
                "/api/auth/setup",
                json!({"name": "admin", "password": "geheim123"}),
            )
            .await;
        assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.json());
        self.refresh_csrf().await;
    }

    async fn login(&mut self, name: &str, password: &str) -> StatusCode {
        let s = self
            .json(
                Method::POST,
                "/api/auth/login",
                json!({"name": name, "password": password}),
            )
            .await
            .status;
        self.refresh_csrf().await;
        s
    }
}

/// Gelände als XYZ-Text: 40 × 40 m, Raster 0,2 m; mit Grube 20 × 20 × 2 m in der Mitte
fn terrain_xyz(pit: bool) -> Vec<u8> {
    let mut s = String::from("x y z\n");
    for j in 0..200 {
        for i in 0..200 {
            let (dx, dy) = ((i as f64 + 0.5) * 0.2, (j as f64 + 0.5) * 0.2);
            let deep = pit && (10.0..30.0).contains(&dx) && (10.0..30.0).contains(&dy);
            let z = 300.0 + 0.01 * dx - if deep { 2.0 } else { 0.0 };
            s += &format!("{:.3} {:.3} {:.3}\n", X0 + dx, Y0 + dy, z);
        }
    }
    s.into_bytes()
}

fn rect(name: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> Value {
    json!({"type": "Feature", "properties": {"name": name}, "geometry": {"type": "Polygon",
        "coordinates": [[[X0 + x0, Y0 + y0], [X0 + x1, Y0 + y0], [X0 + x1, Y0 + y1], [X0 + x0, Y0 + y1], [X0 + x0, Y0 + y0]]]}})
}

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
