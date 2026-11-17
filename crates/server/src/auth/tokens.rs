//! API-Tokens für Programme (iOS-App, Skripte): `Authorization: Bearer fxp_…` statt Sitzungs-Cookie, ohne
//! CSRF-Token (der Browser schickt den Header nicht von selbst). Ein Token hat die Rolle seines Benutzers.
//! Gespeichert wird nur der SHA-256; das Token selbst zeigt die UI genau einmal.

use super::{CurrentUser, Role};
use crate::crypto::{random_token, sha256_hex};
use crate::db::now_ms;
use crate::error::{code, ApiError, ApiResult};
use crate::state::AppRef;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const PREFIX: &str = "fxp_";

/// Token aus `Authorization: Bearer fxp_…`, sonst nichts
pub fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|t| t.starts_with(PREFIX))
}

pub async fn token_user(app: &AppRef, token: &str) -> Option<CurrentUser> {
    let hash = sha256_hex(token);
    let row: Option<(i64, String, String)> = sqlx::query_as(
        "SELECT u.id, u.name, u.role FROM api_tokens t JOIN users u ON u.id = t.user_id WHERE t.token_hash = ?",
    )
    .bind(&hash)
    .fetch_optional(&app.db)
    .await
    .ok()
    .flatten();
    let (id, name, role) = row?;
    let _ = sqlx::query("UPDATE api_tokens SET last_used_at = ? WHERE token_hash = ?")
        .bind(now_ms())
        .bind(&hash)
        .execute(&app.db)
        .await;
    Some(CurrentUser {
        id,
        name,
        role: Role::parse(&role)?,
        csrf: String::new(),
        via_token: true,
    })
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TokenItem {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
}

pub async fn list(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
) -> ApiResult<Json<Vec<TokenItem>>> {
    me.require_session()?;
    Ok(Json(
        sqlx::query_as(
            "SELECT id, name, created_at, last_used_at FROM api_tokens WHERE user_id = ? ORDER BY id",
        )
        .bind(me.id)
        .fetch_all(&app.db)
        .await?,
    ))
}

#[derive(Deserialize)]
pub struct NewToken {
    name: String,
}

/// Neues Token; die Antwort enthält es im Klartext, danach nie wieder
pub async fn create(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Json(t): Json<NewToken>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    me.require_session()?;
    let name = t.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request(code::NAME_REQUIRED));
    }
    let token = format!("{PREFIX}{}", random_token());
    let id = sqlx::query(
        "INSERT INTO api_tokens(user_id, name, token_hash, created_at) VALUES(?, ?, ?, ?)",
    )
    .bind(me.id)
    .bind(name)
    .bind(sha256_hex(&token))
    .bind(now_ms())
    .execute(&app.db)
    .await?
    .last_insert_rowid();
    Ok((
        StatusCode::CREATED,
        Json(json!({ "id": id, "name": name, "token": token })),
    ))
}

pub async fn delete(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    me.require_session()?;
    let n = sqlx::query("DELETE FROM api_tokens WHERE id = ? AND user_id = ?")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_only_with_prefix() {
        let mut h = HeaderMap::new();
        assert_eq!(bearer(&h), None);
        h.insert(header::AUTHORIZATION, "Bearer fxp_abc".parse().unwrap());
        assert_eq!(bearer(&h), Some("fxp_abc"));
        h.insert(header::AUTHORIZATION, "Bearer eyJhbGciOi".parse().unwrap());
        assert_eq!(bearer(&h), None);
        h.insert(header::AUTHORIZATION, "Basic fxp_abc".parse().unwrap());
        assert_eq!(bearer(&h), None);
    }
}
