//! Lieferscheine und Wiegescheine: einzeln erfassen oder als CSV einlesen.
//!
//! CSV, eine Zeile je Schein, Kopfzeile erlaubt: `Datum;Material;Menge;Einheit;Lieferschein`
//! - Datum `17.11.2026` oder `2026-11-17`
//! - Menge mit Komma oder Punkt als Dezimaltrenner
//! - Einheit `t`, `m3`/`m³` (lose, wie geladen) oder `Ladung`/`Fahrt`/`LKW`
//! - Trennzeichen `;`, sonst `,` oder Tab

use super::convert::DELIVERY_UNITS;
use crate::db::now_ms;
use crate::error::{code, ApiError, ApiResult};
use crate::events::Topic;
use crate::state::AppRef;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, FromRow, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Delivery {
    pub id: i64,
    pub site_id: i64,
    /// Tag als Millisekunden (Mitternacht UTC)
    pub delivered_at: i64,
    pub material: Option<String>,
    pub quantity: f64,
    /// `t`, `m3` (lose) oder `loads`
    pub unit: String,
    pub reference: Option<String>,
    pub created_at: i64,
}

/// Ein neuer Schein, aus JSON oder einer CSV-Zeile
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewDelivery {
    /// `2026-11-17`
    pub date: String,
    pub material: Option<String>,
    pub quantity: f64,
    pub unit: String,
    pub reference: Option<String>,
}

/// Tag als Millisekunden seit 1970 aus `2026-11-17` oder `17.11.2026`
pub fn parse_date(s: &str) -> Option<i64> {
    let s = s.trim();
    let (y, m, d): (i32, u8, u8) = if let Some((d, rest)) = s.split_once('.') {
        let (m, y) = rest.split_once('.')?;
        (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?)
    } else {
        let mut it = s.splitn(3, '-');
        (
            it.next()?.parse().ok()?,
            it.next()?.parse().ok()?,
            it.next()?.parse().ok()?,
        )
    };
    let month = time::Month::try_from(m).ok()?;
    let date = time::Date::from_calendar_date(y, month, d).ok()?;
    Some(date.midnight().assume_utc().unix_timestamp() * 1000)
}

/// Einheit aus Text: `t`, `m3`, `loads` und gängige Schreibweisen
pub fn parse_unit(s: &str) -> Option<&'static str> {
    match s.trim().to_lowercase().as_str() {
        "t" | "to" | "tonne" | "tonnen" => Some("t"),
        "m3" | "m³" | "cbm" | "m3 lose" | "m³ lose" => Some("m3"),
        "loads" | "ladung" | "ladungen" | "fahrt" | "fahrten" | "lkw" => Some("loads"),
        _ => None,
    }
}

/// Zahl mit Komma oder Punkt; Tausenderpunkte bei Komma als Dezimaltrenner
pub fn parse_number(s: &str) -> Option<f64> {
    let s = s.trim().replace([' ', '\u{a0}'], "");
    let s = if s.contains(',') {
        s.replace('.', "").replace(',', ".")
    } else {
        s
    };
    s.parse().ok().filter(|v: &f64| v.is_finite())
}

/// geprüfter Schein: (Tag in ms, Material, Menge, Einheit, Lieferschein-Nr.)
type Valid = (i64, Option<String>, f64, &'static str, Option<String>);

pub fn validate(d: &NewDelivery) -> Result<Valid, String> {
    let at = parse_date(&d.date).ok_or_else(|| format!("Datum „{}“", d.date))?;
    let unit = parse_unit(&d.unit).ok_or_else(|| format!("Einheit „{}“", d.unit))?;
    if !(d.quantity.is_finite() && d.quantity > 0.0 && d.quantity < 1e7) {
        return Err(format!("Menge {}", d.quantity));
    }
    let clean = |s: &Option<String>| {
        s.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(Into::into)
    };
    debug_assert!(DELIVERY_UNITS.contains(&unit));
    Ok((
        at,
        clean(&d.material),
        d.quantity,
        unit,
        clean(&d.reference),
    ))
}

/// CSV in Scheine; Fehler nennen die Zeile (ab 1)
pub fn parse_csv(text: &str) -> Result<Vec<NewDelivery>, String> {
    let sep = if text.contains(';') {
        ';'
    } else if text.contains('\t') {
        '\t'
    } else {
        ','
    };
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line
            .split(sep)
            .map(|x| x.trim().trim_matches('"'))
            .collect();
        // Kopfzeile: erstes Feld ist kein Datum
        if i == 0 && parse_date(f[0]).is_none() {
            continue;
        }
        let row = |e: String| format!("Zeile {}: {e}", i + 1);
        if f.len() < 4 {
            return Err(row("Datum, Material, Menge und Einheit erwartet".into()));
        }
        let quantity = parse_number(f[2]).ok_or_else(|| row(format!("Menge „{}“", f[2])))?;
        let d = NewDelivery {
            date: f[0].into(),
            material: Some(f[1].into()),
            quantity,
            unit: f[3].into(),
            reference: f.get(4).map(|s| s.to_string()),
        };
        validate(&d).map_err(row)?;
        out.push(d);
    }
    if out.is_empty() {
        return Err("keine Lieferscheine in der Datei".into());
    }
    Ok(out)
}

fn invalid(detail: String) -> ApiError {
    ApiError::bad_request(code::INVALID_INPUT).with_detail(detail)
}

async fn insert(db: &mut sqlx::SqliteConnection, site: i64, d: &NewDelivery) -> ApiResult<i64> {
    let (at, material, quantity, unit, reference) = validate(d).map_err(invalid)?;
    Ok(sqlx::query(
        "INSERT INTO deliveries(site_id, delivered_at, material, quantity, unit, reference, created_at)
         VALUES(?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(site)
    .bind(at)
    .bind(material)
    .bind(quantity)
    .bind(unit)
    .bind(reference)
    .bind(now_ms())
    .execute(db)
    .await?
    .last_insert_rowid())
}

pub async fn load(app: &AppRef, site: i64) -> ApiResult<Vec<Delivery>> {
    Ok(
        sqlx::query_as("SELECT * FROM deliveries WHERE site_id = ? ORDER BY delivered_at, id")
            .bind(site)
            .fetch_all(&app.db)
            .await?,
    )
}

pub async fn list(
    State(app): State<AppRef>,
    Path(site): Path<i64>,
) -> ApiResult<Json<Vec<Delivery>>> {
    super::site_exists(&app, site).await?;
    Ok(Json(load(&app, site).await?))
}

pub async fn create(
    State(app): State<AppRef>,
    Path(site): Path<i64>,
    Json(d): Json<NewDelivery>,
) -> ApiResult<(StatusCode, Json<Delivery>)> {
    super::site_exists(&app, site).await?;
    let id = insert(&mut *app.db.acquire().await?, site, &d).await?;
    app.events.changed(Topic::Billing, Some(site));
    let row = sqlx::query_as("SELECT * FROM deliveries WHERE id = ?")
        .bind(id)
        .fetch_one(&app.db)
        .await?;
    Ok((StatusCode::CREATED, Json(row)))
}

/// CSV einlesen: alles oder nichts
pub async fn import(
    State(app): State<AppRef>,
    Path(site): Path<i64>,
    body: String,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    super::site_exists(&app, site).await?;
    let rows = parse_csv(&body).map_err(invalid)?;
    let mut tx = app.db.begin().await?;
    for d in &rows {
        insert(&mut tx, site, d).await?;
    }
    tx.commit().await?;
    app.events.changed(Topic::Billing, Some(site));
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"imported": rows.len()})),
    ))
}

pub async fn delete(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<StatusCode> {
    let site: i64 = sqlx::query_scalar("SELECT site_id FROM deliveries WHERE id = ?")
        .bind(id)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(ApiError::not_found)?;
    sqlx::query("DELETE FROM deliveries WHERE id = ?")
        .bind(id)
        .execute(&app.db)
        .await?;
    app.events.changed(Topic::Billing, Some(site));
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_units_numbers() {
        // 17.11.2026 00:00 UTC = 20.774 Tage × 86.400 s
        assert_eq!(parse_date("2026-11-17"), Some(1_794_873_600_000));
        assert_eq!(parse_date("17.11.2026"), Some(1_794_873_600_000));
        assert_eq!(parse_date("31.02.2026"), None);
        assert_eq!(parse_date("Datum"), None);
        assert_eq!(parse_unit("m³"), Some("m3"));
        assert_eq!(parse_unit(" Fahrten "), Some("loads"));
        assert_eq!(parse_unit("kg"), None);
        assert_eq!(parse_number("1.234,5"), Some(1234.5));
        assert_eq!(parse_number("26.5"), Some(26.5));
        assert_eq!(parse_number("x"), None);
    }

    #[test]
    fn csv_with_header_and_errors() {
        let csv = "\u{feff}Datum;Material;Menge;Einheit;Lieferschein\n\
                   17.11.2026;Lehm;26,4;t;LS-1001\n\
                   \n\
                   2026-11-18;\"Oberboden\";12;m³\n";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].quantity, 26.4);
        assert_eq!(rows[0].reference.as_deref(), Some("LS-1001"));
        assert_eq!(rows[1].material.as_deref(), Some("Oberboden"));
        assert_eq!(rows[1].reference, None);
        // Komma als Trenner
        assert_eq!(
            parse_csv("2026-11-17,Lehm,3,Fahrt").unwrap()[0].unit,
            "Fahrt"
        );
        assert_eq!(
            parse_csv("Datum;Material;Menge;Einheit\n17.11.2026;Lehm;viel;t").unwrap_err(),
            "Zeile 2: Menge „viel“"
        );
        assert_eq!(
            parse_csv("17.11.2026;Lehm;3;kg").unwrap_err(),
            "Zeile 1: Einheit „kg“"
        );
        assert!(parse_csv("Datum;Material;Menge;Einheit\n").is_err());
        assert!(parse_csv("17.11.2026;Lehm").is_err());
        assert!(validate(&NewDelivery {
            date: "2026-11-17".into(),
            material: None,
            quantity: -1.0,
            unit: "t".into(),
            reference: None,
        })
        .is_err());
    }
}
