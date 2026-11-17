//! Gemeinsame Hilfen der Integrationstests: Client gegen den echten Router, Testgelände, Bereiche.
#![allow(dead_code)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use fixpunkt_server::config::Config;
use fixpunkt_server::{router, state::AppRef};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

pub const X0: f64 = 512_000.0;
pub const Y0: f64 = 5_403_000.0;

pub struct Client {
    pub app: AppRef,
    pub cookie: Option<String>,
    pub csrf: Option<String>,
    pub _dir: tempfile::TempDir,
}

pub struct Res {
    pub status: StatusCode,
    pub headers: axum::http::HeaderMap,
    pub body: Vec<u8>,
}

impl Res {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }
}

impl Client {
    pub async fn new() -> Self {
        Self::with(|_| {}).await
    }

    /// Wie [`new`](Self::new), mit angepasster Konfiguration
    pub async fn with(configure: impl FnOnce(&mut Config)) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = Config::for_dir(dir.path());
        configure(&mut cfg);
        let app = fixpunkt_server::init(cfg).await.unwrap();
        Client {
            app,
            cookie: None,
            csrf: None,
            _dir: dir,
        }
    }

    /// Zweite Sitzung auf derselben Instanz (anderer Benutzer)
    pub fn other(&self) -> Client {
        Client {
            app: self.app.clone(),
            cookie: None,
            csrf: None,
            _dir: tempfile::tempdir().unwrap(),
        }
    }

    pub async fn send(
        &mut self,
        method: Method,
        uri: &str,
        body: Body,
        content_type: Option<&str>,
        csrf: bool,
    ) -> Res {
        self.send_with(method, uri, body, content_type, csrf, &[])
            .await
    }

    /// Wie [`send`](Self::send), mit eigenen Kopfzeilen (z. B. für tus)
    pub async fn send_with(
        &mut self,
        method: Method,
        uri: &str,
        body: Body,
        content_type: Option<&str>,
        csrf: bool,
        extra: &[(&str, String)],
    ) -> Res {
        let mut req = Request::builder().method(method).uri(uri);
        for (k, v) in extra {
            req = req.header(*k, v);
        }
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

    pub async fn get(&mut self, uri: &str) -> Res {
        self.send(Method::GET, uri, Body::empty(), None, false)
            .await
    }

    pub async fn json(&mut self, method: Method, uri: &str, v: Value) -> Res {
        self.send(
            method,
            uri,
            Body::from(v.to_string()),
            Some("application/json"),
            true,
        )
        .await
    }

    pub async fn delete(&mut self, uri: &str) -> Res {
        self.send(Method::DELETE, uri, Body::empty(), None, true)
            .await
    }

    pub async fn upload(&mut self, uri: &str, data: Vec<u8>) -> Res {
        self.send(
            Method::PUT,
            uri,
            Body::from(data),
            Some("application/octet-stream"),
            true,
        )
        .await
    }

    pub async fn refresh_csrf(&mut self) {
        self.csrf = self.get("/api/auth/state").await.json()["csrf"]
            .as_str()
            .map(String::from);
    }

    pub async fn setup_admin(&mut self) {
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

    pub async fn login(&mut self, name: &str, password: &str) -> StatusCode {
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
pub fn terrain_xyz(pit: bool) -> Vec<u8> {
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

pub fn rect(name: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> Value {
    json!({"type": "Feature", "properties": {"name": name}, "geometry": {"type": "Polygon",
        "coordinates": [[[X0 + x0, Y0 + y0], [X0 + x1, Y0 + y0], [X0 + x1, Y0 + y1], [X0 + x0, Y0 + y1], [X0 + x0, Y0 + y0]]]}})
}
