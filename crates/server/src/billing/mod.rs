//! Abrechnung einer Baustelle: Kennwerte je Material (Dichte, Auflockerung) und LKW-Nutzlast, Aufmaß je
//! Zeitraum mit Leistungsposition und Preis, Lieferscheine zum Gegenrechnen.
//!
//! - `GET|PUT /api/sites/{id}/billing/settings`
//! - `GET /api/sites/{id}/billing?from=&to=` (JSON) und `/billing.pdf?from=&to=` (Aufmaßblatt)
//! - `GET|POST /api/sites/{id}/deliveries`, `PUT /api/sites/{id}/deliveries.csv`, `DELETE /api/deliveries/{id}`

pub mod convert;
pub mod deliveries;
pub mod layers;
pub mod positions;
pub mod shares;
pub mod statement;

use crate::db::{self, Capture, Job, RegionRow};
use crate::error::{code, ApiError, ApiResult};
use crate::events::Topic;
use crate::state::AppRef;
use axum::extract::{Path, Query, State};
use axum::Json;
use convert::Props;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use statement::Settings;

pub(crate) async fn site_exists(app: &AppRef, site: i64) -> ApiResult<db::Site> {
    db::get_site(&app.db, site)
        .await?
        .ok_or_else(ApiError::not_found)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MaterialProps {
    pub name: String,
    pub density: Option<f64>,
    pub bulking: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsBody {
    pub truck_t: Option<f64>,
    pub materials: Vec<MaterialProps>,
    /// Schichten von oben nach unten; fehlt das Feld, bleiben sie unverändert
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<layers::Layer>>,
}

pub async fn load_settings(app: &AppRef, site: i64) -> ApiResult<Settings> {
    let truck_t: Option<f64> = sqlx::query_scalar("SELECT truck_t FROM sites WHERE id = ?")
        .bind(site)
        .fetch_one(&app.db)
        .await?;
    let rows: Vec<(String, Option<f64>, Option<f64>)> = sqlx::query_as(
        "SELECT name, density, bulking FROM site_materials WHERE site_id = ? ORDER BY name",
    )
    .bind(site)
    .fetch_all(&app.db)
    .await?;
    Ok(Settings {
        truck_t,
        layers: layers::load(app, site).await?,
        materials: rows
            .into_iter()
            .map(|(n, density, bulking)| (n, Props { density, bulking }))
            .collect(),
    })
}

/// Kennwerte mit allen Materialien, die in Bereichen oder Lieferscheinen vorkommen (auch ohne Werte)
pub async fn get_settings(
    State(app): State<AppRef>,
    Path(site): Path<i64>,
) -> ApiResult<Json<SettingsBody>> {
    site_exists(&app, site).await?;
    let s = load_settings(&app, site).await?;
    let used: Vec<String> = sqlx::query_scalar(
        "SELECT material FROM regions WHERE site_id = ? AND material IS NOT NULL
         UNION SELECT material FROM deliveries WHERE site_id = ? AND material IS NOT NULL",
    )
    .bind(site)
    .bind(site)
    .fetch_all(&app.db)
    .await?;
    let mut materials: Vec<MaterialProps> = s
        .materials
        .iter()
        .map(|(name, p)| MaterialProps {
            name: name.clone(),
            density: p.density,
            bulking: p.bulking,
        })
        .collect();
    // jede Schicht ist ein Material
    for name in used
        .into_iter()
        .chain(s.layers.iter().map(|l| l.name.clone()))
    {
        if !materials.iter().any(|m| m.name == name) {
            materials.push(MaterialProps {
                name,
                density: None,
                bulking: None,
            });
        }
    }
    materials.sort_by_key(|m| m.name.to_lowercase());
    Ok(Json(SettingsBody {
        truck_t: s.truck_t,
        materials,
        layers: Some(s.layers),
    }))
}

/// Grenzen gegen Tippfehler: Dichte 0,1–5 t/m³, Auflockerung 1–2, Nutzlast bis 60 t
pub fn validate_settings(b: &SettingsBody) -> Result<(), String> {
    let within = |v: Option<f64>, lo: f64, hi: f64| {
        v.is_none_or(|v| v.is_finite() && (lo..=hi).contains(&v))
    };
    if !within(b.truck_t, 0.5, 60.0) {
        return Err("Nutzlast 0,5–60 t".into());
    }
    for m in &b.materials {
        if m.name.trim().is_empty() {
            return Err("Material ohne Namen".into());
        }
        if !within(m.density, 0.1, 5.0) {
            return Err(format!("Dichte von {} (0,1–5 t/m³)", m.name));
        }
        if !within(m.bulking, 1.0, 2.0) {
            return Err(format!("Auflockerung von {} (1–2)", m.name));
        }
    }
    layers::validate(b.layers.as_deref().unwrap_or_default())
}

/// Kennwerte ersetzen; Materialien ohne Werte werden nicht gespeichert
pub async fn put_settings(
    State(app): State<AppRef>,
    Path(site): Path<i64>,
    Json(b): Json<SettingsBody>,
) -> ApiResult<Json<SettingsBody>> {
    site_exists(&app, site).await?;
    validate_settings(&b).map_err(|e| ApiError::bad_request(code::INVALID_INPUT).with_detail(e))?;
    let mut tx = app.db.begin().await?;
    sqlx::query("UPDATE sites SET truck_t = ? WHERE id = ?")
        .bind(b.truck_t)
        .bind(site)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM site_materials WHERE site_id = ?")
        .bind(site)
        .execute(&mut *tx)
        .await?;
    for m in b
        .materials
        .iter()
        .filter(|m| m.density.is_some() || m.bulking.is_some())
    {
        sqlx::query(
            "INSERT OR REPLACE INTO site_materials(site_id, name, density, bulking) VALUES(?, ?, ?, ?)",
        )
        .bind(site)
        .bind(m.name.trim())
        .bind(m.density)
        .bind(m.bulking)
        .execute(&mut *tx)
        .await?;
    }
    if let Some(ls) = &b.layers {
        layers::save(&mut tx, site, ls).await?;
    }
    tx.commit().await?;
    app.events.changed(Topic::Billing, Some(site));
    get_settings(State(app), Path(site)).await
}

#[derive(Debug, Deserialize)]
pub struct Period {
    pub from: i64,
    pub to: i64,
}

/// Aufmaß als JSON, mit Baustelle für den Kopf des Berichts
pub async fn compute(app: &AppRef, site: i64, p: &Period) -> ApiResult<(db::Site, Value)> {
    let s = site_exists(app, site).await?;
    let captures: Vec<Capture> = sqlx::query_as("SELECT * FROM captures WHERE site_id = ?")
        .bind(site)
        .fetch_all(&app.db)
        .await?;
    let regions: Vec<RegionRow> =
        sqlx::query_as("SELECT * FROM regions WHERE site_id = ? ORDER BY name COLLATE NOCASE")
            .bind(site)
            .fetch_all(&app.db)
            .await?;
    let jobs: Vec<Job> = sqlx::query_as("SELECT * FROM jobs WHERE site_id = ? AND kind = 'volume'")
        .bind(site)
        .fetch_all(&app.db)
        .await?;
    let ds = deliveries::load(app, site).await?;
    let settings = load_settings(app, site).await?;
    let v = statement::statement(&captures, &regions, &jobs, &ds, &settings, p.from, p.to)
        .map_err(|e| ApiError::bad_request(code::INVALID_INPUT).with_detail(e.to_string()))?;
    Ok((s, v))
}

pub async fn get_statement(
    State(app): State<AppRef>,
    Path(site): Path<i64>,
    Query(p): Query<Period>,
) -> ApiResult<Json<Value>> {
    Ok(Json(compute(&app, site, &p).await?.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_limits() {
        let body = |truck, density, bulking| SettingsBody {
            truck_t: truck,
            materials: vec![MaterialProps {
                name: "Lehm".into(),
                density,
                bulking,
            }],
            layers: None,
        };
        assert!(validate_settings(&body(Some(15.0), Some(2.0), Some(1.25))).is_ok());
        assert!(validate_settings(&body(None, None, None)).is_ok());
        assert!(validate_settings(&body(Some(100.0), None, None)).is_err());
        // Dichte in kg/m³ statt t/m³ vertippt
        assert!(validate_settings(&body(None, Some(2000.0), None)).is_err());
        assert!(validate_settings(&body(None, None, Some(0.8))).is_err());
        assert!(validate_settings(&body(None, Some(f64::NAN), None)).is_err());
    }
}
