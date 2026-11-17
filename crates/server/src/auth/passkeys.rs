//! Passkeys (WebAuthn) mit webauthn-rs: registrieren im eigenen Konto, anmelden mit Name und Passkey statt
//! Passwort. Die Zeremonie hat zwei Schritte (Optionen an den Browser, Antwort zurück); ihr Zustand liegt bis zu
//! fünf Minuten im Speicher. Nur mit `FIXPUNKT_PUBLIC_URL`: die Domain ist die Relying-Party-ID.

use super::{new_session, CurrentUser};
use crate::crypto::random_token;
use crate::db::now_ms;
use crate::error::{code, ApiError, ApiResult};
use crate::state::AppRef;
use anyhow::Context;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use webauthn_rs::prelude::*;

const CEREMONY_TTL: Duration = Duration::from_secs(300);

pub enum Ceremony {
    Register {
        user_id: i64,
        state: PasskeyRegistration,
    },
    Login {
        user_id: i64,
        state: PasskeyAuthentication,
    },
}

/// Laufende Zeremonien nach zufälliger ID
#[derive(Default)]
pub struct Ceremonies(Mutex<HashMap<String, (Ceremony, Instant)>>);

impl Ceremonies {
    pub fn put(&self, c: Ceremony) -> String {
        let id = random_token();
        let mut m = self.0.lock().unwrap_or_else(|e| e.into_inner());
        m.retain(|_, (_, at)| at.elapsed() < CEREMONY_TTL);
        m.insert(id.clone(), (c, Instant::now()));
        id
    }

    /// Einmal abholen; abgelaufene gelten als unbekannt
    pub fn take(&self, id: &str) -> Option<Ceremony> {
        let mut m = self.0.lock().unwrap_or_else(|e| e.into_inner());
        m.remove(id)
            .filter(|(_, at)| at.elapsed() < CEREMONY_TTL)
            .map(|(c, _)| c)
    }
}

/// Relying Party aus der öffentlichen Adresse: ID = Hostname, Herkunft = Adresse
pub fn webauthn(public_url: &str) -> anyhow::Result<Webauthn> {
    let origin = Url::parse(public_url).context("keine gültige Adresse")?;
    let host = origin.host_str().context("Adresse ohne Host")?.to_string();
    Ok(WebauthnBuilder::new(&host, &origin)?
        .rp_name("Fixpunkt")
        .build()?)
}

/// Stabile WebAuthn-Kennung je Benutzer (IDs werden in SQLite mit AUTOINCREMENT nie wiederverwendet)
pub fn user_handle(user_id: i64) -> Uuid {
    Uuid::from_u64_pair(0x6669_7870_756e_6b74, user_id as u64)
}

fn rp(app: &AppRef) -> ApiResult<&Webauthn> {
    app.webauthn
        .as_ref()
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, code::PASSKEYS_DISABLED))
}

fn failed(e: impl std::fmt::Display) -> ApiError {
    ApiError::bad_request(code::PASSKEY_FAILED).with_detail(e.to_string())
}

async fn passkeys_of(app: &AppRef, user_id: i64) -> ApiResult<Vec<(i64, Passkey)>> {
    let rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, passkey FROM passkeys WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(&app.db)
            .await?;
    rows.into_iter()
        .map(|(id, json)| Ok((id, serde_json::from_str(&json)?)))
        .collect::<anyhow::Result<_>>()
        .map_err(Into::into)
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PasskeyItem {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
}

pub async fn list(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
) -> ApiResult<Json<Vec<PasskeyItem>>> {
    me.require_session()?;
    Ok(Json(
        sqlx::query_as(
            "SELECT id, name, created_at, last_used_at FROM passkeys WHERE user_id = ? ORDER BY id",
        )
        .bind(me.id)
        .fetch_all(&app.db)
        .await?,
    ))
}

/// Schritt 1 der Registrierung: Optionen für `navigator.credentials.create`
pub async fn register_start(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
) -> ApiResult<Json<Value>> {
    me.require_session()?;
    let existing = passkeys_of(&app, me.id).await?;
    let exclude = existing.iter().map(|(_, p)| p.cred_id().clone()).collect();
    let (options, state) = rp(&app)?
        .start_passkey_registration(user_handle(me.id), &me.name, &me.name, Some(exclude))
        .map_err(failed)?;
    let id = app.ceremonies.put(Ceremony::Register {
        user_id: me.id,
        state,
    });
    Ok(Json(json!({ "id": id, "options": options })))
}

#[derive(Deserialize)]
pub struct RegisterFinish {
    id: String,
    name: String,
    credential: RegisterPublicKeyCredential,
}

/// Schritt 2: Antwort des Browsers prüfen und den Passkey speichern
pub async fn register_finish(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Json(r): Json<RegisterFinish>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    me.require_session()?;
    let Some(Ceremony::Register { user_id, state }) = app.ceremonies.take(&r.id) else {
        return Err(failed("Registrierung unbekannt oder abgelaufen"));
    };
    if user_id != me.id {
        return Err(failed("Registrierung eines anderen Benutzers"));
    }
    let key = rp(&app)?
        .finish_passkey_registration(&r.credential, &state)
        .map_err(failed)?;
    let name = match r.name.trim() {
        "" => "Passkey",
        n => n,
    };
    let id = sqlx::query(
        "INSERT INTO passkeys(user_id, name, cred_id, passkey, created_at) VALUES(?, ?, ?, ?, ?)",
    )
    .bind(me.id)
    .bind(name)
    .bind(hex::encode(key.cred_id()))
    .bind(serde_json::to_string(&key)?)
    .bind(now_ms())
    .execute(&app.db)
    .await?
    .last_insert_rowid();
    Ok((StatusCode::CREATED, Json(json!({ "id": id, "name": name }))))
}

pub async fn delete(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    me.require_session()?;
    let n = sqlx::query("DELETE FROM passkeys WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(me.id)
        .execute(&app.db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(ApiError::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct LoginStart {
    name: String,
}

/// Anmelden, Schritt 1: Optionen für `navigator.credentials.get` mit den Passkeys des Benutzers
pub async fn login_start(
    State(app): State<AppRef>,
    Json(l): Json<LoginStart>,
) -> ApiResult<Json<Value>> {
    let webauthn = rp(&app)?;
    let user: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE name = ?")
        .bind(l.name.trim())
        .fetch_optional(&app.db)
        .await?;
    let keys = match user {
        Some(u) => passkeys_of(&app, u).await?,
        None => Vec::new(),
    };
    let (Some(user_id), false) = (user, keys.is_empty()) else {
        tokio::time::sleep(Duration::from_millis(500)).await;
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, code::NO_PASSKEY));
    };
    let creds: Vec<Passkey> = keys.into_iter().map(|(_, p)| p).collect();
    let (options, state) = webauthn
        .start_passkey_authentication(&creds)
        .map_err(failed)?;
    let id = app.ceremonies.put(Ceremony::Login { user_id, state });
    Ok(Json(json!({ "id": id, "options": options })))
}

#[derive(Deserialize)]
pub struct LoginFinish {
    id: String,
    credential: PublicKeyCredential,
}

/// Anmelden, Schritt 2: Signatur prüfen, Zähler nachführen, Sitzung anlegen
pub async fn login_finish(
    State(app): State<AppRef>,
    jar: CookieJar,
    Json(l): Json<LoginFinish>,
) -> ApiResult<(CookieJar, StatusCode)> {
    let Some(Ceremony::Login { user_id, state }) = app.ceremonies.take(&l.id) else {
        return Err(failed("Anmeldung unbekannt oder abgelaufen"));
    };
    let result = rp(&app)?
        .finish_passkey_authentication(&l.credential, &state)
        .map_err(|e| {
            ApiError::new(StatusCode::UNAUTHORIZED, code::PASSKEY_FAILED).with_detail(e.to_string())
        })?;
    for (id, mut key) in passkeys_of(&app, user_id).await? {
        if key.update_credential(&result).is_some() {
            sqlx::query("UPDATE passkeys SET passkey = ?, last_used_at = ? WHERE id = ?")
                .bind(serde_json::to_string(&key)?)
                .bind(now_ms())
                .bind(id)
                .execute(&app.db)
                .await?;
        }
    }
    Ok((
        new_session(&app, jar, user_id).await?,
        StatusCode::NO_CONTENT,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relying_party_from_public_url() {
        assert!(webauthn("https://fixpunkt.example.de").is_ok());
        assert!(webauthn("http://localhost:8080").is_ok());
        assert!(webauthn("kein url").is_err());
        assert_ne!(user_handle(1), user_handle(2));
        assert_eq!(user_handle(7), user_handle(7));
    }

    #[test]
    fn ceremonies_are_taken_once() {
        let c = Ceremonies::default();
        let rp = webauthn("http://localhost:8080").unwrap();
        let (_, state) = rp
            .start_passkey_registration(user_handle(1), "a", "a", None)
            .unwrap();
        let id = c.put(Ceremony::Register { user_id: 1, state });
        assert!(matches!(
            c.take(&id),
            Some(Ceremony::Register { user_id: 1, .. })
        ));
        assert!(c.take(&id).is_none());
        assert!(c.take("unbekannt").is_none());
    }
}
