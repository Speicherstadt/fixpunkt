//! Auftragsarten mit ihren Parametern: prüfen beim Anlegen, ausführen im Worker.
//! Jede Art hat ein eigenes, typisiertes Parameterobjekt; gespeichert wird es als JSON in `jobs.params`.

use crate::compute::compare::DesignInput;
use crate::compute::export::Format;
use crate::compute::{self, SurfaceParams};
use crate::db::{self, status};
use crate::error::{code, ApiError, ApiResult};
use crate::state::AppRef;
use anyhow::{anyhow, bail, Context, Result};
use axum::http::StatusCode;
use fixpunkt::geom::Region;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::PathBuf;

/// Arten, die man über die API anlegen kann (`inspect` legt der Upload selbst an)
pub const USER_KINDS: &[&str] = &["volume", "stockpile", "dem", "compare", "series", "export"];

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "params", rename_all = "lowercase")]
pub enum JobSpec {
    Inspect(Inspect),
    Volume(Volume),
    Stockpile(Stockpile),
    Dem(Dem),
    Compare(Compare),
    Series(Series),
    Export(Export),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspect {
    pub capture_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub before: i64,
    pub after: i64,
    #[serde(default)]
    pub region_ids: Vec<i64>,
    #[serde(default)]
    pub surface: SurfaceParams,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stockpile {
    pub capture_id: i64,
    pub region_ids: Vec<i64>,
    /// `"plane"`, `"lowest"` oder eine Höhe
    #[serde(default = "plane")]
    pub base: Value,
    #[serde(default)]
    pub surface: SurfaceParams,
}

fn plane() -> Value {
    json!("plane")
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dem {
    pub capture_id: i64,
    #[serde(default)]
    pub surface: SurfaceParams,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DesignSpec {
    Height {
        height: f64,
    },
    Slope {
        x: f64,
        y: f64,
        z: f64,
        percent: f64,
        azimuth: f64,
    },
    #[serde(rename_all = "camelCase")]
    Surface {
        capture_id: i64,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Compare {
    pub capture_id: i64,
    pub design: DesignSpec,
    pub region_id: Option<i64>,
    #[serde(default = "tolerance")]
    pub tolerance: f64,
    #[serde(default)]
    pub surface: SurfaceParams,
}

fn tolerance() -> f64 {
    0.03
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    /// zeitlich geordnet, die erste ist die Basis (Urgelände)
    pub capture_ids: Vec<i64>,
    pub region_id: Option<i64>,
    #[serde(default)]
    pub surface: SurfaceParams,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Export {
    pub capture_id: i64,
    pub formats: Vec<Format>,
    #[serde(default)]
    pub region_ids: Vec<i64>,
    #[serde(default = "interval")]
    pub contour_interval: f64,
    #[serde(default)]
    pub surface: SurfaceParams,
}

fn interval() -> f64 {
    0.5
}

impl JobSpec {
    pub fn parse(kind: &str, params: &Value) -> Result<JobSpec> {
        Ok(serde_json::from_value(
            json!({ "kind": kind, "params": params }),
        )?)
    }

    /// Aufnahmen, die bereit sein müssen
    pub fn captures(&self) -> Vec<i64> {
        match self {
            JobSpec::Inspect(_) => Vec::new(),
            JobSpec::Volume(v) => vec![v.before, v.after],
            JobSpec::Stockpile(s) => vec![s.capture_id],
            JobSpec::Dem(d) => vec![d.capture_id],
            JobSpec::Compare(c) => match c.design {
                DesignSpec::Surface { capture_id } => vec![c.capture_id, capture_id],
                _ => vec![c.capture_id],
            },
            JobSpec::Series(s) => s.capture_ids.clone(),
            JobSpec::Export(e) => vec![e.capture_id],
        }
    }

    pub fn regions(&self) -> Vec<i64> {
        match self {
            JobSpec::Volume(v) => v.region_ids.clone(),
            JobSpec::Stockpile(s) => s.region_ids.clone(),
            JobSpec::Export(e) => e.region_ids.clone(),
            JobSpec::Compare(Compare { region_id, .. })
            | JobSpec::Series(Series { region_id, .. }) => region_id.iter().copied().collect(),
            JobSpec::Inspect(_) | JobSpec::Dem(_) => Vec::new(),
        }
    }

    fn surface(&self) -> SurfaceParams {
        match self {
            JobSpec::Inspect(_) => SurfaceParams::default(),
            JobSpec::Volume(v) => v.surface,
            JobSpec::Stockpile(s) => s.surface,
            JobSpec::Dem(d) => d.surface,
            JobSpec::Compare(c) => c.surface,
            JobSpec::Series(s) => s.surface,
            JobSpec::Export(e) => e.surface,
        }
    }

    /// Prüfungen ohne Datenbank
    pub fn validate(&self) -> Result<()> {
        self.surface().validate()?;
        match self {
            JobSpec::Volume(v) if v.before == v.after => bail!("zwei verschiedene Aufnahmen nötig"),
            JobSpec::Stockpile(s) => {
                if s.region_ids.is_empty() {
                    bail!("mindestens ein Bereich nötig");
                }
                compute::parse_base(&s.base)?;
            }
            JobSpec::Compare(c) if !(0.0..=1.0).contains(&c.tolerance) => {
                bail!("Toleranz muss zwischen 0 und 1 m liegen")
            }
            JobSpec::Series(s) => {
                let distinct: HashSet<_> = s.capture_ids.iter().collect();
                if s.capture_ids.len() < 2 || distinct.len() != s.capture_ids.len() {
                    bail!("mindestens zwei verschiedene Aufnahmen nötig");
                }
            }
            JobSpec::Export(e) if e.formats.is_empty() => bail!("kein Format gewählt"),
            _ => {}
        }
        Ok(())
    }
}

/// Prüft einen neuen Auftrag gegen die Datenbank: Aufnahmen bereit und von dieser Baustelle, Bereiche auch.
pub async fn check(app: &AppRef, site_id: i64, kind: &str, params: &Value) -> ApiResult<JobSpec> {
    if !USER_KINDS.contains(&kind) {
        return Err(ApiError::bad_request(code::UNKNOWN_JOB));
    }
    let invalid =
        |e: anyhow::Error| ApiError::bad_request(code::INVALID_INPUT).with_detail(format!("{e:#}"));
    let spec = JobSpec::parse(kind, params).map_err(invalid)?;
    for id in spec.captures() {
        let c = db::get_capture(&app.db, id)
            .await?
            .ok_or_else(ApiError::not_found)?;
        if c.site_id != site_id {
            return Err(ApiError::bad_request(code::WRONG_SITE));
        }
        if c.status != status::READY {
            return Err(ApiError::new(StatusCode::CONFLICT, code::CAPTURE_NOT_READY));
        }
    }
    for id in spec.regions() {
        let found: Option<i64> =
            sqlx::query_scalar("SELECT id FROM regions WHERE id = ? AND site_id = ?")
                .bind(id)
                .bind(site_id)
                .fetch_optional(&app.db)
                .await?;
        if found.is_none() {
            return Err(ApiError::bad_request(code::WRONG_SITE));
        }
    }
    spec.validate().map_err(invalid)?;
    Ok(spec)
}

pub fn capture_path(app: &AppRef, c: &db::Capture) -> PathBuf {
    app.cfg.capture_dir(c.id).join(&c.file_name)
}

async fn capture(app: &AppRef, site_id: i64, id: i64) -> Result<(String, PathBuf)> {
    let c = db::get_capture(&app.db, id)
        .await?
        .filter(|c| c.site_id == site_id)
        .ok_or_else(|| anyhow!("Aufnahme {id} gibt es nicht"))?;
    Ok((c.name.clone(), capture_path(app, &c)))
}

async fn file(app: &AppRef, site_id: i64, id: i64) -> Result<PathBuf> {
    Ok(capture(app, site_id, id).await?.1)
}

async fn regions(app: &AppRef, site_id: i64, ids: &[i64]) -> Result<Vec<Region>> {
    let mut out = Vec::new();
    for id in ids {
        let (name, gj): (String, String) =
            sqlx::query_as("SELECT name, geojson FROM regions WHERE id = ? AND site_id = ?")
                .bind(id)
                .bind(site_id)
                .fetch_optional(&app.db)
                .await?
                .ok_or_else(|| anyhow!("Bereich {id} gibt es nicht"))?;
        out.push(compute::parse_region(&name, &gj)?);
    }
    Ok(out)
}

/// Rechnet im Thread-Pool, damit der Server weiter antwortet
async fn blocking(f: impl FnOnce() -> Result<Value> + Send + 'static) -> Result<Value> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| anyhow!("{e}"))?
}

/// Führt einen Auftrag aus. `out_dir` nimmt die Ergebnisdateien auf.
pub async fn execute(app: &AppRef, site_id: i64, spec: JobSpec, out_dir: PathBuf) -> Result<Value> {
    let site = db::get_site(&app.db, site_id)
        .await?
        .context("Baustelle fehlt")?;
    let epsg = u16::try_from(site.epsg).unwrap_or(25832);
    let s = site_id;
    match spec {
        JobSpec::Inspect(i) => {
            let c = file(app, s, i.capture_id).await?;
            blocking(move || compute::inspect(&c)).await
        }
        JobSpec::Volume(v) => {
            let (a, b) = (file(app, s, v.before).await?, file(app, s, v.after).await?);
            let rs = regions(app, s, &v.region_ids).await?;
            blocking(move || compute::volume::volume(&a, &b, &rs, &v.surface, epsg, &out_dir)).await
        }
        JobSpec::Stockpile(p) => {
            let c = file(app, s, p.capture_id).await?;
            let rs = regions(app, s, &p.region_ids).await?;
            let base = compute::parse_base(&p.base)?;
            blocking(move || compute::volume::stockpile(&c, &rs, base, &p.surface)).await
        }
        JobSpec::Dem(d) => {
            let c = file(app, s, d.capture_id).await?;
            blocking(move || compute::volume::dem(&c, &d.surface, epsg, &out_dir)).await
        }
        JobSpec::Compare(c) => {
            let scan = file(app, s, c.capture_id).await?;
            let design = match c.design {
                DesignSpec::Height { height } => DesignInput::Height(height),
                DesignSpec::Slope {
                    x,
                    y,
                    z,
                    percent,
                    azimuth,
                } => DesignInput::Slope {
                    x,
                    y,
                    z,
                    percent,
                    azimuth_deg: azimuth,
                },
                DesignSpec::Surface { capture_id } => {
                    DesignInput::Surface(file(app, s, capture_id).await?)
                }
            };
            let region = regions(app, s, &c.region_id.into_iter().collect::<Vec<_>>())
                .await?
                .pop();
            blocking(move || {
                compute::compare::compare(
                    &scan,
                    &design,
                    region.as_ref(),
                    c.tolerance,
                    &c.surface,
                    epsg,
                    &out_dir,
                )
            })
            .await
        }
        JobSpec::Series(sr) => {
            let mut caps = Vec::new();
            for id in &sr.capture_ids {
                caps.push(capture(app, s, *id).await?);
            }
            let region = regions(app, s, &sr.region_id.into_iter().collect::<Vec<_>>())
                .await?
                .pop();
            blocking(move || compute::volume::series(&caps, region.as_ref(), &sr.surface)).await
        }
        JobSpec::Export(e) => {
            let (name, c) = capture(app, s, e.capture_id).await?;
            let rs = regions(app, s, &e.region_ids).await?;
            blocking(move || {
                compute::export::export(
                    &c,
                    &e.formats,
                    &rs,
                    e.contour_interval,
                    &name,
                    &e.surface,
                    epsg,
                    &out_dir,
                )
            })
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_validate_each_kind() {
        let ok = |k: &str, p: Value| JobSpec::parse(k, &p).and_then(|s| s.validate().map(|_| s));
        assert!(matches!(
            ok("volume", json!({"before": 1, "after": 2})),
            Ok(JobSpec::Volume(_))
        ));
        assert!(ok("volume", json!({"before": 1, "after": 1})).is_err());
        assert!(ok("stockpile", json!({"captureId": 1, "regionIds": []})).is_err());
        assert!(ok(
            "stockpile",
            json!({"captureId": 1, "regionIds": [3], "base": "lowest"})
        )
        .is_ok());
        assert!(ok(
            "stockpile",
            json!({"captureId": 1, "regionIds": [3], "base": "irgendwas"})
        )
        .is_err());
        let c = ok(
            "compare",
            json!({"captureId": 1, "design": {"type": "surface", "captureId": 2}}),
        )
        .unwrap();
        assert_eq!(c.captures(), vec![1, 2]);
        assert!(ok(
            "compare",
            json!({"captureId": 1, "design": {"type": "height", "height": 296.4}, "tolerance": 2})
        )
        .is_err());
        let s = ok("series", json!({"captureIds": [1, 2, 3], "regionId": 5})).unwrap();
        assert_eq!((s.captures(), s.regions()), (vec![1, 2, 3], vec![5]));
        assert!(ok("series", json!({"captureIds": [1, 1]})).is_err());
        assert!(ok(
            "export",
            json!({"captureId": 1, "formats": ["dxf", "landxml"]})
        )
        .is_ok());
        assert!(ok("export", json!({"captureId": 1, "formats": []})).is_err());
        assert!(ok("export", json!({"captureId": 1, "formats": ["pdf"]})).is_err());
        assert!(ok("dem", json!({"captureId": 1, "surface": {"cell": 0}})).is_err());
        assert!(JobSpec::parse("zaubern", &json!({})).is_err());
    }
}
