//! Login, Sitzungen, CSRF und Rollen. Vorbild: firsttris/haul crates/haul/src/auth.rs –
//! erweitert um mehrere Benutzer mit Rollen und ein CSRF-Token pro Sitzung.

use crate::crypto::{hash_password, random_token, same, sha256_hex, verify_password};
use crate::db::{self, now_ms, User};
use crate::error::{code, ApiError, ApiResult};
use crate::events::Topic;
use crate::state::AppRef;
use axum::extract::{Path, Request, State};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const SESSION_COOKIE: &str = "fixpunkt_session";
pub const CSRF_HEADER: &str = "x-csrf-token";
const SESSION_DAYS: i64 = 30;
const MIN_PASSWORD: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// nur lesen
    Viewer,
    /// Baustellen, Aufnahmen, Berechnungen bearbeiten
    Editor,
    /// zusätzlich Benutzer verwalten
    Admin,
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "viewer" => Some(Role::Viewer),
            "editor" => Some(Role::Editor),
            "admin" => Some(Role::Admin),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Role::Viewer => "viewer",
            Role::Editor => "editor",
            Role::Admin => "admin",
        }
    }
}

/// Angemeldeter Benutzer, von [`require_auth`] in die Anfrage gelegt.
#[derive(Debug, Clone, Serialize)]
pub struct CurrentUser {
    pub id: i64,
    pub name: String,
    pub role: Role,
    #[serde(skip)]
    pub csrf: String,
}

impl CurrentUser {
    pub fn require(&self, role: Role) -> ApiResult<()> {
        if self.role >= role {
            Ok(())
        } else {
            Err(ApiError::new(StatusCode::FORBIDDEN, code::FORBIDDEN))
        }
    }
}

/// Mindestrolle für eine Anfrage: Lesen darf jeder Angemeldete, Ändern ab „editor“.
pub fn role_for(method: &Method) -> Role {
    if is_safe(method) {
        Role::Viewer
    } else {
        Role::Editor
    }
}

fn is_safe(m: &Method) -> bool {
    matches!(*m, Method::GET | Method::HEAD | Method::OPTIONS)
}

pub async fn ensure_initial_user(app: &AppRef) -> anyhow::Result<()> {
    if let Some((name, pass)) = &app.cfg.initial_user {
        if db::user_count(&app.db).await? == 0 {
            create_user(app, name, pass, Role::Admin)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e.code))?;
            tracing::info!("Benutzer {name} aus FIXPUNKT_USER/FIXPUNKT_PASSWORD angelegt");
        }
    }
    Ok(())
}

async fn session_user(app: &AppRef, token: &str) -> Option<CurrentUser> {
    let row: Option<(i64, String, String, String)> = sqlx::query_as(
        "SELECT u.id, u.name, u.role, s.csrf FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = ? AND s.expires_at > ?",
    )
    .bind(sha256_hex(token))
    .bind(now_ms())
    .fetch_optional(&app.db)
    .await
    .ok()
    .flatten();
    let (id, name, role, csrf) = row?;
    Some(CurrentUser {
        id,
        name,
        role: Role::parse(&role)?,
        csrf,
    })
}

/// Sitzungs-Cookie prüfen, bei ändernden Anfragen auch das CSRF-Token und die Rolle.
pub async fn require_auth(
    State(app): State<AppRef>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Response {
    let user = match jar.get(SESSION_COOKIE) {
        Some(c) => session_user(&app, c.value()).await,
        None => None,
    };
    let Some(user) = user else {
        return ApiError::new(StatusCode::UNAUTHORIZED, code::NOT_LOGGED_IN).into_response();
    };
    if !is_safe(req.method()) {
        let sent = req
            .headers()
            .get(CSRF_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !same(sent, &user.csrf) {
            return ApiError::new(StatusCode::FORBIDDEN, code::CSRF).into_response();
        }
    }
    if let Err(e) = user.require(role_for(req.method())) {
        return e.into_response();
    }
    req.extensions_mut().insert(user);
    next.run(req).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthState {
    setup_required: bool,
    user: Option<CurrentUser>,
    /// für den Header `x-csrf-token` bei ändernden Anfragen
    csrf: Option<String>,
}

pub async fn state(State(app): State<AppRef>, jar: CookieJar) -> ApiResult<Json<AuthState>> {
    let setup_required = db::user_count(&app.db).await? == 0;
    let user = match jar.get(SESSION_COOKIE) {
        Some(c) => session_user(&app, c.value()).await,
        None => None,
    };
    let csrf = user.as_ref().map(|u| u.csrf.clone());
    Ok(Json(AuthState {
        setup_required,
        user,
        csrf,
    }))
}

#[derive(Deserialize)]
pub struct Credentials {
    name: String,
    password: String,
}

async fn new_session(app: &AppRef, jar: CookieJar, user_id: i64) -> ApiResult<CookieJar> {
    let token = random_token();
    sqlx::query("INSERT INTO sessions(token_hash, user_id, csrf, expires_at) VALUES(?, ?, ?, ?)")
        .bind(sha256_hex(&token))
        .bind(user_id)
        .bind(random_token())
        .bind(now_ms() + SESSION_DAYS * 86_400_000)
        .execute(&app.db)
        .await?;
    sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
        .bind(now_ms())
        .execute(&app.db)
        .await?;
    let cookie = Cookie::build((SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .secure(app.cfg.secure_cookies)
        .max_age(time::Duration::days(SESSION_DAYS))
        .build();
    Ok(jar.add(cookie))
}

fn validate(name: &str, password: &str) -> ApiResult<()> {
    if name.trim().is_empty() {
        return Err(ApiError::bad_request(code::NAME_REQUIRED));
    }
    if password.chars().count() < MIN_PASSWORD {
        return Err(ApiError::bad_request(code::PASSWORD_TOO_SHORT));
    }
    Ok(())
}

async fn create_user(app: &AppRef, name: &str, password: &str, role: Role) -> ApiResult<i64> {
    validate(name, password)?;
    let res =
        sqlx::query("INSERT INTO users(name, password_hash, role, created_at) VALUES(?, ?, ?, ?)")
            .bind(name.trim())
            .bind(hash_password(password)?)
            .bind(role.as_str())
            .bind(now_ms())
            .execute(&app.db)
            .await;
    match res {
        Ok(r) => Ok(r.last_insert_rowid()),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            Err(ApiError::new(StatusCode::CONFLICT, code::NAME_TAKEN))
        }
        Err(e) => Err(e.into()),
    }
}

/// Erster Start: der erste Benutzer wird Admin.
pub async fn setup(
    State(app): State<AppRef>,
    jar: CookieJar,
    Json(c): Json<Credentials>,
) -> ApiResult<(CookieJar, StatusCode)> {
    if db::user_count(&app.db).await? > 0 {
        return Err(ApiError::new(StatusCode::CONFLICT, code::SETUP_DONE));
    }
    let id = create_user(&app, &c.name, &c.password, Role::Admin).await?;
    Ok((new_session(&app, jar, id).await?, StatusCode::NO_CONTENT))
}

pub async fn login(
    State(app): State<AppRef>,
    jar: CookieJar,
    Json(c): Json<Credentials>,
) -> ApiResult<(CookieJar, StatusCode)> {
    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE name = ?")
        .bind(c.name.trim())
        .fetch_optional(&app.db)
        .await?;
    match user {
        Some(u) if verify_password(&c.password, &u.password_hash) => {
            Ok((new_session(&app, jar, u.id).await?, StatusCode::NO_CONTENT))
        }
        _ => {
            // bremst Durchprobieren
            tokio::time::sleep(Duration::from_millis(500)).await;
            Err(ApiError::new(
                StatusCode::UNAUTHORIZED,
                code::WRONG_CREDENTIALS,
            ))
        }
    }
}

pub async fn logout(
    State(app): State<AppRef>,
    jar: CookieJar,
) -> ApiResult<(CookieJar, StatusCode)> {
    if let Some(c) = jar.get(SESSION_COOKIE) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(sha256_hex(c.value()))
            .execute(&app.db)
            .await?;
    }
    Ok((
        jar.remove(Cookie::build(SESSION_COOKIE).path("/")),
        StatusCode::NO_CONTENT,
    ))
}

#[derive(Deserialize)]
pub struct PasswordChange {
    current: String,
    new: String,
}

pub async fn change_password(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Json(p): Json<PasswordChange>,
) -> ApiResult<StatusCode> {
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = ?")
        .bind(me.id)
        .fetch_one(&app.db)
        .await?;
    if !verify_password(&p.current, &hash) {
        return Err(ApiError::bad_request(code::WRONG_PASSWORD));
    }
    validate(&me.name, &p.new)?;
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(hash_password(&p.new)?)
        .bind(me.id)
        .execute(&app.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- Benutzerverwaltung (Admin) ------------------------------------------------------

pub async fn list_users(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
) -> ApiResult<Json<Vec<User>>> {
    me.require(Role::Admin)?;
    Ok(Json(
        sqlx::query_as("SELECT * FROM users ORDER BY name")
            .fetch_all(&app.db)
            .await?,
    ))
}

#[derive(Deserialize)]
pub struct NewUser {
    name: String,
    password: String,
    role: Role,
}

pub async fn add_user(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Json(u): Json<NewUser>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    me.require(Role::Admin)?;
    let id = create_user(&app, &u.name, &u.password, u.role).await?;
    app.events.changed(Topic::Users, None);
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

pub async fn delete_user(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    me.require(Role::Admin)?;
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&app.db)
        .await?;
    let Some(role) = role else {
        return Err(ApiError::not_found());
    };
    if role == "admin" {
        let admins: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'")
            .fetch_one(&app.db)
            .await?;
        if admins <= 1 {
            return Err(ApiError::new(StatusCode::CONFLICT, code::LAST_ADMIN));
        }
    }
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(&app.db)
        .await?;
    app.events.changed(Topic::Users, None);
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_are_ordered_and_methods_map() {
        assert!(Role::Admin > Role::Editor && Role::Editor > Role::Viewer);
        assert_eq!(role_for(&Method::GET), Role::Viewer);
        assert_eq!(role_for(&Method::POST), Role::Editor);
        assert_eq!(role_for(&Method::DELETE), Role::Editor);
        for r in [Role::Viewer, Role::Editor, Role::Admin] {
            assert_eq!(Role::parse(r.as_str()), Some(r));
        }
        assert_eq!(Role::parse("root"), None);
        let u = CurrentUser {
            id: 1,
            name: "a".into(),
            role: Role::Editor,
            csrf: String::new(),
        };
        assert!(u.require(Role::Viewer).is_ok() && u.require(Role::Admin).is_err());
        assert!(
            validate("x", "1234567").is_err()
                && validate(" ", "12345678").is_err()
                && validate("x", "12345678").is_ok()
        );
    }
}
