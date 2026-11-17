//! Eigener Überblick: jeder Benutzer wählt je Baustelle, welche Abschnitte er sieht und in welcher Reihenfolge.
//! `GET/PUT/DELETE /api/me/layouts/{site}` – am eigenen Konto, also auch für „Nur lesen“.

use crate::auth::CurrentUser;
use crate::db::{self, now_ms};
use crate::error::{ApiError, ApiResult};
use crate::state::AppRef;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

/// Abschnitte des Überblicks in der Grundreihenfolge
pub const SECTIONS: &[&str] = &[
    "kpis",
    "progress",
    "targets",
    "stock",
    "history",
    "attention",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub sections: Vec<Item>,
    /// selbst angepasst (sonst die Grundeinstellung)
    #[serde(default)]
    pub custom: bool,
}

/// Bekannte Abschnitte in der gespeicherten Reihenfolge, jeder einmal; neue (z. B. nach einem Update) hinten
/// sichtbar angehängt
pub fn normalize(saved: &[Item]) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    for i in saved {
        if SECTIONS.contains(&i.id.as_str()) && !out.iter().any(|o| o.id == i.id) {
            out.push(i.clone());
        }
    }
    for s in SECTIONS {
        if !out.iter().any(|o| o.id == *s) {
            out.push(Item {
                id: (*s).into(),
                visible: true,
            });
        }
    }
    out
}

async fn site_exists(app: &AppRef, site: i64) -> ApiResult<()> {
    db::get_site(&app.db, site)
        .await?
        .map(|_| ())
        .ok_or_else(ApiError::not_found)
}

pub async fn get(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(site): Path<i64>,
) -> ApiResult<Json<Layout>> {
    site_exists(&app, site).await?;
    let saved: Option<String> =
        sqlx::query_scalar("SELECT layout FROM user_layouts WHERE user_id = ? AND site_id = ?")
            .bind(me.id)
            .bind(site)
            .fetch_optional(&app.db)
            .await?;
    let items: Vec<Item> = saved
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    Ok(Json(Layout {
        sections: normalize(&items),
        custom: saved.is_some(),
    }))
}

#[derive(Deserialize)]
pub struct Save {
    sections: Vec<Item>,
}

pub async fn put(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(site): Path<i64>,
    Json(s): Json<Save>,
) -> ApiResult<Json<Layout>> {
    site_exists(&app, site).await?;
    let sections = normalize(&s.sections);
    sqlx::query(
        "INSERT INTO user_layouts(user_id, site_id, layout, updated_at) VALUES(?, ?, ?, ?)
         ON CONFLICT(user_id, site_id) DO UPDATE SET layout = excluded.layout, updated_at = excluded.updated_at",
    )
    .bind(me.id)
    .bind(site)
    .bind(serde_json::to_string(&sections)?)
    .bind(now_ms())
    .execute(&app.db)
    .await?;
    Ok(Json(Layout {
        sections,
        custom: true,
    }))
}

pub async fn reset(
    State(app): State<AppRef>,
    Extension(me): Extension<CurrentUser>,
    Path(site): Path<i64>,
) -> ApiResult<StatusCode> {
    sqlx::query("DELETE FROM user_layouts WHERE user_id = ? AND site_id = ?")
        .bind(me.id)
        .bind(site)
        .execute(&app.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, visible: bool) -> Item {
        Item {
            id: id.into(),
            visible,
        }
    }

    #[test]
    fn unknown_dropped_duplicates_once_missing_appended() {
        let n = normalize(&[
            item("stock", true),
            item("kpis", false),
            item("gibt-es-nicht", true),
            item("stock", false),
        ]);
        let ids: Vec<&str> = n.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "stock",
                "kpis",
                "progress",
                "targets",
                "history",
                "attention"
            ]
        );
        assert_eq!(
            (n[0].visible, n[1].visible, n[2].visible),
            (true, false, true)
        );
        assert_eq!(normalize(&[]).len(), SECTIONS.len());
    }
}
