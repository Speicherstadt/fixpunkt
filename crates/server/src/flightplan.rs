//! Flugplanung je Abrechnungsbereich: `GET /api/regions/{id}/flightplan` liefert Bahnen und Kennzahlen für die
//! Vorschau, `GET /api/regions/{id}/flightplan.kmz` die Mission für DJI Pilot 2. Einstellungen als Query
//! (`gsd` in cm, `front`/`side` in %, `direction` in Grad, `speed` in m/s, `drone`).
//!
//! Gibt es eine geprüfte Aufnahme, kommt der Höhenunterschied im Bereich mit: die Höhe gilt über dem Startpunkt,
//! bei viel Gefälle schwankt die Bodenauflösung.

use crate::compute::parse_region;
use crate::db::{self, status, Capture, RegionRow};
use crate::error::{code, ApiError, ApiResult};
use crate::report::file_name;
use crate::state::AppRef;
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use fixpunkt::crs::{utm_zone, Utm};
use fixpunkt::flightplan::{self, FlightParams, FlightPlan};
use fixpunkt::geom::Polygon;
use fixpunkt::grid::Grid;
use fixpunkt::wpml::{self, Drone};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PlanQuery {
    /// Bodenauflösung in cm
    pub gsd: f64,
    /// Überlappung in %
    pub front: f64,
    pub side: f64,
    pub direction: f64,
    pub speed: f64,
    /// `m4e` oder `m3e`
    pub drone: String,
}

impl Default for PlanQuery {
    fn default() -> Self {
        let d = FlightParams::default();
        PlanQuery {
            gsd: d.gsd * 100.0,
            front: d.front_overlap * 100.0,
            side: d.side_overlap * 100.0,
            direction: d.direction,
            speed: d.speed,
            drone: "m4e".into(),
        }
    }
}

impl PlanQuery {
    pub fn params(&self) -> FlightParams {
        FlightParams {
            gsd: self.gsd / 100.0,
            front_overlap: self.front / 100.0,
            side_overlap: self.side / 100.0,
            direction: self.direction,
            speed: self.speed,
        }
    }

    pub fn drone(&self) -> Result<(Drone, flightplan::Camera), ApiError> {
        match self.drone.as_str() {
            "m4e" => Ok((wpml::M4E, flightplan::M4E)),
            // Mavic 3E: gleicher 4/3-Sensor mit 20 MP und 12,29 mm
            "m3e" => Ok((
                wpml::M3E,
                flightplan::Camera {
                    name: "DJI Mavic 3 Enterprise",
                    max_speed: 15.0,
                    ..flightplan::M4E
                },
            )),
            _ => Err(ApiError::bad_request(code::INVALID_INPUT).with_detail("drone: m4e oder m3e")),
        }
    }
}

struct Context {
    region: RegionRow,
    site: db::Site,
    polygon: Polygon,
    utm: Utm,
}

async fn context(app: &AppRef, id: i64) -> ApiResult<Context> {
    let region: RegionRow = sqlx::query_as("SELECT * FROM regions WHERE id = ?")
        .bind(id)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let site = db::get_site(&app.db, region.site_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let polygon = parse_region(&region.name, &region.geojson)?.polygon;
    let zone = utm_zone(site.epsg as u32).unwrap_or(32);
    Ok(Context {
        region,
        site,
        polygon,
        utm: Utm::new(zone)?,
    })
}

fn make(q: &PlanQuery, polygon: &Polygon) -> ApiResult<(FlightPlan, Drone, &'static str)> {
    let (drone, cam) = q.drone()?;
    let plan = flightplan::plan(polygon, &cam, &q.params())
        .map_err(|e| ApiError::bad_request(code::INVALID_INPUT).with_detail(e.to_string()))?;
    Ok((plan, drone, cam.name))
}

/// Tiefster und höchster Punkt des Geländes im Bereich
pub fn terrain_range(g: &Grid, polygon: &Polygon) -> Option<(f64, f64)> {
    let inside = g.spec.mask(Some(polygon));
    g.data
        .iter()
        .zip(inside)
        .filter(|(z, k)| *k && z.is_finite())
        .map(|(z, _)| f64::from(*z))
        .fold(None, |acc, z| match acc {
            None => Some((z, z)),
            Some((lo, hi)) => Some((lo.min(z), hi.max(z))),
        })
}

async fn terrain(app: &AppRef, c: &Context) -> ApiResult<Option<Value>> {
    let last: Option<Capture> = sqlx::query_as(
        "SELECT * FROM captures WHERE site_id = ? AND status = ? ORDER BY captured_at DESC, id DESC LIMIT 1",
    )
    .bind(c.site.id)
    .bind(status::READY)
    .fetch_optional(&app.db)
    .await?;
    let Some(last) = last else { return Ok(None) };
    let g = crate::view::capture_view(app, &last).await?;
    Ok(terrain_range(&g, &c.polygon)
        .map(|(lo, hi)| json!({ "capture": last.name, "min": lo, "max": hi })))
}

pub async fn preview(
    State(app): State<AppRef>,
    Path(id): Path<i64>,
    Query(q): Query<PlanQuery>,
) -> ApiResult<Json<Value>> {
    let c = context(&app, id).await?;
    let (plan, _, camera) = make(&q, &c.polygon)?;
    Ok(Json(json!({
        "region": c.region.name,
        "camera": camera,
        "plan": plan,
        "terrain": terrain(&app, &c).await?,
    })))
}

pub async fn kmz(
    State(app): State<AppRef>,
    Path(id): Path<i64>,
    Query(q): Query<PlanQuery>,
) -> ApiResult<Response> {
    let c = context(&app, id).await?;
    let (plan, drone, _) = make(&q, &c.polygon)?;
    let bytes = wpml::kmz(&plan, &c.utm, &drone, db::now_ms())?;
    let pdf = file_name(
        "flugplan",
        &format!("{} {}", c.site.name, c.region.name),
        id,
    );
    let name = format!("{}.kmz", pdf.strip_suffix(".pdf").unwrap_or(&pdf));
    Ok((
        [
            (
                header::CONTENT_TYPE,
                "application/vnd.google-earth.kmz".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixpunkt::grid::GridSpec;

    #[test]
    fn query_defaults_and_units() {
        let q = PlanQuery::default();
        assert_eq!(q.params(), FlightParams::default());
        let q = PlanQuery {
            gsd: 1.5,
            front: 75.0,
            ..Default::default()
        };
        assert_eq!(q.params().gsd, 0.015);
        assert_eq!(q.params().front_overlap, 0.75);
        assert!(PlanQuery {
            drone: "phantom".into(),
            ..Default::default()
        }
        .drone()
        .is_err());
    }

    #[test]
    fn terrain_inside_the_polygon_only() {
        // 10 × 10 Zellen à 1 m, Höhe = x; Bereich x 2…5 → Zellmitten 2,5 … 4,5
        let spec = GridSpec::covering(
            &fixpunkt::geom::BBox {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 10.0,
                max_y: 10.0,
            },
            1.0,
        )
        .unwrap();
        let mut g = Grid::new_nodata(spec);
        for r in 0..g.spec.rows {
            for col in 0..g.spec.cols {
                let x = g.spec.center(col, r).0;
                g.set(col, r, x as f32);
            }
        }
        let (lo, hi) = terrain_range(&g, &Polygon::rect(2.0, 2.0, 5.0, 5.0)).unwrap();
        assert_eq!((lo, hi), (2.5, 4.5));
        assert!(terrain_range(&g, &Polygon::rect(50.0, 50.0, 60.0, 60.0)).is_none());
    }
}
