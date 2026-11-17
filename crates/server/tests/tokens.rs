//! API-Tokens und Passkeys über die API.

mod common;

use axum::body::Body;
use axum::http::{Method, StatusCode};
use common::Client;
use serde_json::json;

/// Anfrage nur mit `Authorization: Bearer`, ohne Cookie und CSRF
async fn with_token(
    c: &Client,
    method: Method,
    uri: &str,
    token: &str,
    body: Option<serde_json::Value>,
) -> common::Res {
    let mut anon = c.other();
    let (body, ct) = match body {
        Some(v) => (Body::from(v.to_string()), Some("application/json")),
        None => (Body::empty(), None),
    };
    anon.send_with(
        method,
        uri,
        body,
        ct,
        false,
        &[("authorization", format!("Bearer {token}"))],
    )
    .await
}

#[tokio::test]
async fn api_tokens() {
    let mut c = Client::new().await;
    c.setup_admin().await;

    let bad = c
        .json(Method::POST, "/api/me/tokens", json!({"name": " "}))
        .await;
    assert_eq!(bad.json()["error"], "auth.nameRequired");
    let created = c
        .json(
            Method::POST,
            "/api/me/tokens",
            json!({"name": "iPad Polier"}),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED);
    let t = created.json();
    let token = t["token"].as_str().unwrap().to_string();
    assert!(token.starts_with("fxp_") && token.len() == 4 + 64);

    // Token liest und ändert ohne CSRF; der Browser schickt den Header nie von selbst
    let sites = with_token(&c, Method::GET, "/api/sites", &token, None).await;
    assert_eq!(sites.status, StatusCode::OK);
    let made = with_token(
        &c,
        Method::POST,
        "/api/sites",
        &token,
        Some(json!({"name": "Per Token"})),
    )
    .await;
    assert_eq!(made.status, StatusCode::CREATED, "{:?}", made.json());

    // in der Liste ohne Klartext, mit letzter Nutzung
    let list = c.get("/api/me/tokens").await.json();
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["name"], "iPad Polier");
    assert!(list[0]["lastUsedAt"].as_i64().is_some());
    assert!(!list.to_string().contains(&token[4..]));

    // ein Token verwaltet keine Tokens
    let own = with_token(&c, Method::GET, "/api/me/tokens", &token, None).await;
    assert_eq!(own.json()["error"], "auth.sessionRequired");

    // falsches Token, fremdes Format
    for t in ["fxp_00", "abc"] {
        let r = with_token(&c, Method::GET, "/api/sites", t, None).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{t}");
    }

    // Token eines Lesers darf nur lesen, sein Konto aber selbst verwalten
    c.json(
        Method::POST,
        "/api/users",
        json!({"name": "leser", "password": "lesen1234", "role": "viewer"}),
    )
    .await;
    let mut viewer = c.other();
    assert_eq!(
        viewer.login("leser", "lesen1234").await,
        StatusCode::NO_CONTENT
    );
    let vt = viewer
        .json(Method::POST, "/api/me/tokens", json!({"name": "Skript"}))
        .await;
    assert_eq!(vt.status, StatusCode::CREATED);
    let vt = vt.json()["token"].as_str().unwrap().to_string();
    let denied = with_token(
        &c,
        Method::POST,
        "/api/sites",
        &vt,
        Some(json!({"name": "x"})),
    )
    .await;
    assert_eq!(denied.json()["error"], "auth.forbidden");
    // fremdes Token löschen geht nicht
    let id = t["id"].as_i64().unwrap();
    assert_eq!(
        viewer.delete(&format!("/api/me/tokens/{id}")).await.status,
        StatusCode::NOT_FOUND
    );

    // Widerruf
    assert_eq!(
        c.delete(&format!("/api/me/tokens/{id}")).await.status,
        StatusCode::NO_CONTENT
    );
    let gone = with_token(&c, Method::GET, "/api/sites", &token, None).await;
    assert_eq!(gone.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn passkeys_need_a_public_url() {
    let mut c = Client::new().await;
    c.setup_admin().await;
    assert_eq!(c.get("/api/auth/state").await.json()["passkeys"], false);
    let r = c
        .json(Method::POST, "/api/me/passkeys/start", json!({}))
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.json()["error"], "passkeys.disabled");
}

#[tokio::test]
async fn passkey_ceremonies() {
    let mut c =
        Client::with(|cfg| cfg.public_url = Some("https://fixpunkt.example.de".into())).await;
    c.setup_admin().await;
    assert_eq!(c.get("/api/auth/state").await.json()["passkeys"], true);
    assert_eq!(c.get("/api/me/passkeys").await.json(), json!([]));

    // Registrierung: Optionen für navigator.credentials.create mit der Domain als RP-ID
    let start = c
        .json(Method::POST, "/api/me/passkeys/start", json!({}))
        .await
        .json();
    let pk = &start["options"]["publicKey"];
    assert_eq!(pk["rp"]["id"], "fixpunkt.example.de");
    assert_eq!(pk["user"]["name"], "admin");
    assert_eq!(pk["authenticatorSelection"]["userVerification"], "required");
    assert!(pk["challenge"].as_str().unwrap().len() >= 43); // 32 Byte base64url

    // eine gefälschte Antwort scheitert, und die Zeremonie ist danach verbraucht
    let fake = json!({"id": start["id"], "name": "Laptop", "credential": {
        "id": "AAAA", "rawId": "AAAA", "type": "public-key",
        "response": {"attestationObject": "AAAA", "clientDataJSON": "AAAA"}}});
    let r = c
        .json(Method::POST, "/api/me/passkeys/finish", fake.clone())
        .await;
    assert_eq!(r.json()["error"], "passkeys.failed", "{:?}", r.json());
    let again = c.json(Method::POST, "/api/me/passkeys/finish", fake).await;
    assert_eq!(again.json()["error"], "passkeys.failed");

    // ohne Passkey keine Anmeldung damit; gleiche Antwort für unbekannte Namen
    let mut anon = c.other();
    for name in ["admin", "niemand"] {
        let r = anon
            .json(
                Method::POST,
                "/api/auth/passkey/start",
                json!({"name": name}),
            )
            .await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED);
        assert_eq!(r.json()["error"], "passkeys.none");
    }
}
