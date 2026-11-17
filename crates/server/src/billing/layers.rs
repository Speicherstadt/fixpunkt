//! Schichten (Homogenbereiche) einer Baustelle: von oben nach unten, je Schicht die Unterkante in m unter dem
//! Urgelände; die letzte reicht unbegrenzt nach unten. Jede Schicht ist zugleich ein Material (Kennwerte) und
//! kann eine eigene Leistungsposition mit Preis haben.

use crate::error::ApiResult;
use crate::state::AppRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub name: String,
    /// Unterkante in m unter dem Urgelände; leer bei der letzten Schicht
    pub bottom_m: Option<f64>,
    pub lv_position: Option<String>,
    pub unit_price: Option<f64>,
    /// `m3`, `m3_loose` oder `t`
    pub price_unit: Option<String>,
}

pub async fn load(app: &AppRef, site: i64) -> ApiResult<Vec<Layer>> {
    Ok(sqlx::query_as(
        "SELECT name, bottom_m, lv_position, unit_price, price_unit FROM site_layers
         WHERE site_id = ? ORDER BY sort",
    )
    .bind(site)
    .fetch_all(&app.db)
    .await?)
}

/// Namen und Grenzen für den Rechenauftrag; ohne Schichten `None`
pub fn split(layers: &[Layer]) -> Option<crate::compute::volume::Layers> {
    if layers.is_empty() {
        return None;
    }
    let names = layers.iter().map(|l| l.name.clone()).collect();
    let bottoms = layers[..layers.len() - 1]
        .iter()
        .filter_map(|l| l.bottom_m)
        .collect();
    Some((names, bottoms))
}

pub async fn for_job(
    app: &AppRef,
    site: i64,
) -> anyhow::Result<Option<crate::compute::volume::Layers>> {
    Ok(split(
        &load(app, site)
            .await
            .map_err(|e| anyhow::anyhow!("{e:?}"))?,
    ))
}

/// Namen eindeutig und nicht leer, Unterkanten 0,01–100 m aufsteigend, die letzte ohne, Preis wie am Bereich
pub fn validate(layers: &[Layer]) -> Result<(), String> {
    if layers.len() > 12 {
        return Err("höchstens 12 Schichten".into());
    }
    let mut top = 0.0;
    for (i, l) in layers.iter().enumerate() {
        let name = l.name.trim();
        if name.is_empty() {
            return Err("Schicht ohne Namen".into());
        }
        if layers[..i].iter().any(|o| o.name.trim() == name) {
            return Err(format!("Schicht {name} doppelt"));
        }
        let last = i + 1 == layers.len();
        match (l.bottom_m, last) {
            (Some(_), true) => {
                return Err(format!(
                    "die unterste Schicht {name} reicht unbegrenzt nach unten"
                ))
            }
            (None, false) => return Err(format!("Unterkante von {name} fehlt")),
            (Some(b), false) if !(b.is_finite() && b > top && b <= 100.0) => {
                return Err(format!("Unterkante von {name} muss unter {top} m liegen"))
            }
            (Some(b), false) => top = b,
            (None, true) => {}
        }
        if l.unit_price
            .is_some_and(|p| !p.is_finite() || !(0.0..=1e6).contains(&p))
        {
            return Err(format!("Einheitspreis von {name}"));
        }
        if !matches!(
            l.price_unit.as_deref(),
            None | Some("m3") | Some("m3_loose") | Some("t")
        ) {
            return Err(format!("Preiseinheit von {name}"));
        }
    }
    Ok(())
}

pub async fn save(tx: &mut sqlx::SqliteConnection, site: i64, layers: &[Layer]) -> ApiResult<()> {
    sqlx::query("DELETE FROM site_layers WHERE site_id = ?")
        .bind(site)
        .execute(&mut *tx)
        .await?;
    for (i, l) in layers.iter().enumerate() {
        let position = l
            .lv_position
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        sqlx::query(
            "INSERT INTO site_layers(site_id, sort, name, bottom_m, lv_position, unit_price, price_unit)
             VALUES(?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(site)
        .bind(i as i64)
        .bind(l.name.trim())
        .bind(l.bottom_m)
        .bind(position)
        .bind(l.unit_price)
        .bind(l.unit_price.map(|_| l.price_unit.clone().unwrap_or_else(|| "m3".into())))
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn layer(name: &str, bottom: Option<f64>) -> Layer {
        Layer {
            name: name.into(),
            bottom_m: bottom,
            lv_position: None,
            unit_price: None,
            price_unit: None,
        }
    }

    #[test]
    fn layers_are_checked() {
        let ok = [
            layer("Oberboden", Some(0.3)),
            layer("Lehm", Some(1.5)),
            layer("Fels", None),
        ];
        assert!(validate(&ok).is_ok());
        assert_eq!(
            split(&ok),
            Some((
                vec!["Oberboden".into(), "Lehm".into(), "Fels".into()],
                vec![0.3, 1.5]
            ))
        );
        assert_eq!(split(&[]), None);
        assert!(validate(&[layer("Lehm", None)]).is_ok());
        assert!(validate(&[layer("A", Some(1.0)), layer("B", Some(2.0))]).is_err());
        assert!(validate(&[layer("A", None), layer("B", None)]).is_err());
        assert!(validate(&[
            layer("A", Some(1.0)),
            layer("B", Some(0.5)),
            layer("C", None)
        ])
        .is_err());
        assert!(validate(&[layer("A", Some(1.0)), layer("A", None)]).is_err());
        assert!(validate(&[layer(" ", None)]).is_err());
        let mut priced = layer("Lehm", None);
        priced.price_unit = Some("kg".into());
        assert!(validate(&[priced]).is_err());
    }
}
