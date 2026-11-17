//! Freigabe eines Aufmaßes durch den Auftraggeber: ein Link mit Zufallstoken zeigt das Aufmaß ohne Anmeldung, so
//! wie es beim Anlegen war (eingefroren), und nimmt genau eine Entscheidung an: anerkannt oder Einwand, mit Name
//! und Zeit. Gespeichert wird nur der SHA-256 des Tokens; der Link erscheint einmal beim Anlegen.
//!
//! - `GET|POST /api/sites/{id}/billing/shares`, `DELETE /api/billing/shares/{id}` (zurückziehen),
//!   `GET /api/billing/shares/{id}/pdf`
//! - ohne Anmeldung: `GET /api/public/shares/{token}`, `POST …/{token}/decision`, `GET …/{token}/pdf`

use super::{compute, load_settings, Period};
use crate::auth::CurrentUser;
use crate::crypto::{random_token, sha256_hex};
use crate::db::now_ms;
use crate::error::{code, ApiError, ApiResult};
use crate::events::Topic;
use crate::state::AppRef;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct ShareRow {
    pub id: i64,
    pub site_id: i64,
    pub token_hash: String,
    pub from_capture: i64,
    pub to_capture: i64,
    pub snapshot: String,
    pub note: Option<String>,
    pub created_by: Option<i64>,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
    pub decision: Option<String>,
    pub decided_name: Option<String>,
    pub decided_org: Option<String>,
    pub comment: Option<String>,
    pub decided_at: Option<i64>,
}

/// Entscheidung des Auftraggebers
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    /// `approved` oder `objected`
    pub decision: String,
    pub name: String,
    pub organization: Option<String>,
    pub comment: Option<String>,
    pub at: i64,
}

impl ShareRow {
    pub fn decision(&self) -> Option<Decision> {
        Some(Decision {
            decision: self.decision.clone()?,
            name: self.decided_name.clone().unwrap_or_default(),
            organization: self.decided_org.clone(),
            comment: self.comment.clone(),
            at: self.decided_at.unwrap_or(0),
        })
    }

    /// das eingefrorene Aufmaß mit Baustelle und Bezugssystem
    pub fn snapshot(&self) -> Value {
        serde_json::from_str(&self.snapshot).unwrap_or(Value::Null)
    }

    /// Übersicht ohne Token, für die Liste in der Baustelle
    fn summary(&self) -> Value {
        let s = self.snapshot();
        json!({
            "id": self.id,
            "from": s["statement"]["from"],
            "to": s["statement"]["to"],
            "amount": s["statement"]["amount"],
            "note": self.note,
            "createdAt": self.created_at,
            "revokedAt": self.revoked_at,
            "decision": self.decision(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewShare {
    pub from: i64,
    pub to: i64,
    pub note: Option<String>,
}

pub async fn create(
    State(app): State<AppRef>,
    Path(site): Path<i64>,
    Extension(me): Extension<CurrentUser>,
    Json(n): Json<NewShare>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let p = Period {
        from: n.from,
        to: n.to,
    };
    let (s, statement) = compute(&app, site, &p).await?;
    let settings = load_settings(&app, site).await?;
    let snapshot = json!({
        "site": s.name,
        "epsg": s.epsg,
        "statement": statement,
        "materials": settings.materials.iter().map(|(n, p)| json!([n, p.density, p.bulking])).collect::<Vec<_>>(),
    });
    let token = random_token();
    let note = n.note.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let id = sqlx::query(
        "INSERT INTO billing_shares(site_id, token_hash, from_capture, to_capture, snapshot, note, created_by,
           created_at) VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(site)
    .bind(sha256_hex(&token))
    .bind(n.from)
    .bind(n.to)
    .bind(snapshot.to_string())
    .bind(note)
    .bind(me.id)
    .bind(now_ms())
    .execute(&app.db)
    .await?
    .last_insert_rowid();
    app.events.changed(Topic::Billing, Some(site));
    let row = by_id(&app, id).await?;
    let mut out = row.summary();
    out["token"] = json!(token);
    Ok((StatusCode::CREATED, Json(out)))
}

async fn by_id(app: &AppRef, id: i64) -> ApiResult<ShareRow> {
    sqlx::query_as("SELECT * FROM billing_shares WHERE id = ?")
        .bind(id)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(ApiError::not_found)
}

/// Freigabe zum Token, solange sie nicht zurückgezogen ist
async fn by_token(app: &AppRef, token: &str) -> ApiResult<ShareRow> {
    let row: Option<ShareRow> =
        sqlx::query_as("SELECT * FROM billing_shares WHERE token_hash = ? AND revoked_at IS NULL")
            .bind(sha256_hex(token))
            .fetch_optional(&app.db)
            .await?;
    row.ok_or_else(ApiError::not_found)
}

pub async fn list(State(app): State<AppRef>, Path(site): Path<i64>) -> ApiResult<Json<Vec<Value>>> {
    super::site_exists(&app, site).await?;
    let rows: Vec<ShareRow> =
        sqlx::query_as("SELECT * FROM billing_shares WHERE site_id = ? ORDER BY id DESC")
            .bind(site)
            .fetch_all(&app.db)
            .await?;
    Ok(Json(rows.iter().map(ShareRow::summary).collect()))
}

/// Link zurückziehen: danach öffnet er nichts mehr; eine Entscheidung bleibt erhalten
pub async fn revoke(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<StatusCode> {
    let row = by_id(&app, id).await?;
    sqlx::query("UPDATE billing_shares SET revoked_at = COALESCE(revoked_at, ?) WHERE id = ?")
        .bind(now_ms())
        .bind(id)
        .execute(&app.db)
        .await?;
    app.events.changed(Topic::Billing, Some(row.site_id));
    Ok(StatusCode::NO_CONTENT)
}

/// Was der Auftraggeber sieht
fn public_view(row: &ShareRow) -> Value {
    let s = row.snapshot();
    json!({
        "site": s["site"],
        "statement": s["statement"],
        "note": row.note,
        "createdAt": row.created_at,
        "decision": row.decision(),
    })
}

pub async fn public_get(
    State(app): State<AppRef>,
    Path(token): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(public_view(&by_token(&app, &token).await?)))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionBody {
    /// `approved` oder `objected`
    pub decision: String,
    pub name: String,
    pub organization: Option<String>,
    pub comment: Option<String>,
}

/// Name nötig, bei einem Einwand auch eine Begründung; Längen begrenzt
pub fn validate_decision(d: &DecisionBody) -> Result<(), &'static str> {
    if !matches!(d.decision.as_str(), "approved" | "objected") {
        return Err("Entscheidung: approved oder objected");
    }
    let name = d.name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err("Name");
    }
    let comment = d.comment.as_deref().map(str::trim).unwrap_or("");
    if d.decision == "objected" && comment.is_empty() {
        return Err("Begründung des Einwands");
    }
    if comment.chars().count() > 4000
        || d.organization
            .as_deref()
            .is_some_and(|o| o.chars().count() > 200)
    {
        return Err("Text zu lang");
    }
    Ok(())
}

pub async fn public_decide(
    State(app): State<AppRef>,
    Path(token): Path<String>,
    Json(d): Json<DecisionBody>,
) -> ApiResult<Json<Value>> {
    let row = by_token(&app, &token).await?;
    validate_decision(&d).map_err(|e| ApiError::bad_request(code::INVALID_INPUT).with_detail(e))?;
    let clean = |s: Option<String>| s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    // nur die erste Entscheidung zählt
    let done = sqlx::query(
        "UPDATE billing_shares SET decision = ?, decided_name = ?, decided_org = ?, comment = ?, decided_at = ?
         WHERE id = ? AND decision IS NULL AND revoked_at IS NULL",
    )
    .bind(&d.decision)
    .bind(d.name.trim())
    .bind(clean(d.organization))
    .bind(clean(d.comment))
    .bind(now_ms())
    .bind(row.id)
    .execute(&app.db)
    .await?
    .rows_affected();
    if done == 0 {
        return Err(ApiError::new(StatusCode::CONFLICT, code::SHARE_DECIDED));
    }
    app.events.changed(Topic::Billing, Some(row.site_id));
    Ok(Json(public_view(&by_id(&app, row.id).await?)))
}

pub async fn public_pdf(
    State(app): State<AppRef>,
    Path(token): Path<String>,
) -> ApiResult<Response> {
    crate::report::site_routes::share_pdf(&by_token(&app, &token).await?).await
}

pub async fn pdf(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<Response> {
    crate::report::site_routes::share_pdf(&by_id(&app, id).await?).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(decision: &str, name: &str, comment: Option<&str>) -> DecisionBody {
        DecisionBody {
            decision: decision.into(),
            name: name.into(),
            organization: None,
            comment: comment.map(Into::into),
        }
    }

    #[test]
    fn decisions_need_a_name_and_a_reason_to_object() {
        assert!(validate_decision(&body("approved", "M. Muster", None)).is_ok());
        assert!(validate_decision(&body("approved", "  ", None)).is_err());
        assert!(validate_decision(&body("maybe", "M. Muster", None)).is_err());
        assert!(validate_decision(&body("objected", "M. Muster", None)).is_err());
        assert!(
            validate_decision(&body("objected", "M. Muster", Some("Bereich Rampe fehlt"))).is_ok()
        );
        assert!(validate_decision(&body("approved", &"x".repeat(121), None)).is_err());
    }
}
