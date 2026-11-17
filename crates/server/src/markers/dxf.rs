//! Dokumentation als DXF (R12) für CAD: je Art eine Ebene, Beschriftung daneben. Zwangspunkte mit Sollhöhe als z,
//! Linien als 3D-Polylinie (Stützpunkte ohne Gelände auf der Höhe ihrer Nachbarn, sonst 0).
//! `GET /api/sites/{id}/documentation.dxf`

use super::Marker;
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::report::file_name;
use crate::state::AppRef;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use fixpunkt::export::Dxf;

/// Schrifthöhe der Beschriftung in m
pub const TEXT_HEIGHT: f64 = 0.25;

pub fn layer(kind: &str) -> &'static str {
    match kind {
        "constraint" => "FP_ZWANGSPUNKTE",
        "photo" => "FP_FOTOS",
        "line" => "FP_LEITUNGEN",
        _ => "FP_NOTIZEN",
    }
}

/// Fehlende Höhen auffüllen: erste bekannte davor oder danach; ganz ohne Höhe 0
pub fn fill_heights(z: &[Option<f64>]) -> Vec<f64> {
    let first = z.iter().flatten().next().copied().unwrap_or(0.0);
    let mut last = first;
    z.iter()
        .map(|v| {
            if let Some(v) = v {
                last = *v;
            }
            last
        })
        .collect()
}

pub fn documentation_dxf(markers: &[Marker]) -> String {
    let mut d = Dxf::new();
    for m in markers {
        let layer = layer(&m.kind);
        if m.kind == "line" {
            let pts = m.line();
            let z = fill_heights(&pts.iter().map(|p| p.1).collect::<Vec<_>>());
            let xyz: Vec<[f64; 3]> = pts
                .iter()
                .zip(&z)
                .map(|((p, _), z)| [p[0], p[1], *z])
                .collect();
            if let Some(first) = xyz.first() {
                d.polyline(layer, &xyz, false);
                d.text(layer, *first, TEXT_HEIGHT, &m.title);
            }
            continue;
        }
        let (Some(x), Some(y)) = (m.x, m.y) else {
            continue;
        };
        let p = [x, y, m.z.unwrap_or(0.0)];
        d.point(layer, p);
        let label = match m.z {
            Some(z) if m.kind == "constraint" => format!("{} {:.2}", m.title, z),
            _ => m.title.clone(),
        };
        d.text(layer, [x + TEXT_HEIGHT, y, p[2]], TEXT_HEIGHT, &label);
    }
    d.render()
}

pub async fn documentation(
    State(app): State<AppRef>,
    Path(site): Path<i64>,
) -> ApiResult<Response> {
    let s = db::get_site(&app.db, site)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let list: Vec<Marker> = sqlx::query_as("SELECT * FROM markers WHERE site_id = ? ORDER BY id")
        .bind(site)
        .fetch_all(&app.db)
        .await?;
    let pdf = file_name("Dokumentation", &s.name, site);
    let name = format!("{}.dxf", pdf.strip_suffix(".pdf").unwrap_or(&pdf));
    Ok((
        [
            (header::CONTENT_TYPE, "image/vnd.dxf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        documentation_dxf(&list),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(kind: &str, title: &str, xyz: Option<[f64; 3]>, points: Option<&str>) -> Marker {
        Marker {
            id: 1,
            site_id: 1,
            kind: kind.into(),
            title: title.into(),
            note: String::new(),
            x: xyz.map(|p| p[0]),
            y: xyz.map(|p| p[1]),
            z: xyz.map(|p| p[2]),
            photo: None,
            taken_at: None,
            created_by: None,
            created_at: 0,
            points: points.map(Into::into),
        }
    }

    #[test]
    fn heights_are_filled_from_neighbours() {
        assert_eq!(
            fill_heights(&[None, Some(2.0), None, Some(3.0)]),
            [2.0, 2.0, 2.0, 3.0]
        );
        assert_eq!(fill_heights(&[None, None]), [0.0, 0.0]);
    }

    #[test]
    fn dxf_reads_back_with_the_plan_reader() {
        let ms = [
            marker(
                "constraint",
                "Kanal",
                Some([512_032.0, 5_403_020.0, 300.3]),
                None,
            ),
            marker(
                "note",
                "Bodenprobe",
                Some([512_015.0, 5_403_034.0, 0.0]),
                None,
            ),
            marker(
                "line",
                "Leitung",
                None,
                Some("[[512000,5403000,299.5],[512010,5403000],[512010,5403020,299.7]]"),
            ),
            marker("note", "ohne Lage", None, None),
        ];
        let text = documentation_dxf(&ms);
        // unabhängig mit dem DXF-Leser der Pläne (Crate dxf) gelesen
        let g = crate::plans::dxf::read(text.as_bytes()).unwrap();
        assert_eq!(
            g.layers,
            ["FP_LEITUNGEN", "FP_NOTIZEN", "FP_ZWANGSPUNKTE"].map(String::from)
        );
        let line = g.paths.iter().find(|p| p.layer == "FP_LEITUNGEN").unwrap();
        assert_eq!(
            line.points,
            [
                [512_000.0, 5_403_000.0],
                [512_010.0, 5_403_000.0],
                [512_010.0, 5_403_020.0]
            ]
        );
        assert!(!line.closed);
        // Zwangspunkt mit Sollhöhe als Höhenpunkt; die Beschriftung „Kanal 300.30“ ist keine Kote
        assert!(g
            .spots
            .iter()
            .any(|s| s.layer == "FP_ZWANGSPUNKTE" && s.at == [512_032.0, 5_403_020.0, 300.3]));
        assert!(g.labels.iter().any(|l| l.text == "Kanal 300.30"));
        assert!(!g.labels.iter().any(|l| l.text == "ohne Lage"));
    }
}
