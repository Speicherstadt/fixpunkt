//! PDF-Bericht eines fertigen Auftrags: `GET /api/jobs/{id}/report`. Aushub, Haufwerk, Soll/Ist und
//! Zeitreihe; der Inhalt entsteht in `build` (reine Funktionen), das Layout in `pdf`.

pub mod build;
pub mod doc;
pub mod map;
mod metrics;
pub mod pdf;
pub mod text;

use crate::db::{self, status};
use crate::error::{ApiError, ApiResult};
use crate::jobs::spec::{self, DesignSpec, JobSpec};
use crate::state::AppRef;
use anyhow::Result;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use build::Context;
use fixpunkt::geom::Region;
use fixpunkt::geotiff;
use serde_json::Value;
use std::path::PathBuf;

/// Dateiname nur aus ASCII: Umlaute umschreiben, alles andere wird „-“
pub fn file_name(prefix: &str, site: &str, id: i64) -> String {
    let mut s = String::new();
    for c in site.chars() {
        match c {
            'ä' => s.push_str("ae"),
            'ö' => s.push_str("oe"),
            'ü' => s.push_str("ue"),
            'Ä' => s.push_str("Ae"),
            'Ö' => s.push_str("Oe"),
            'Ü' => s.push_str("Ue"),
            'ß' => s.push_str("ss"),
            c if c.is_ascii_alphanumeric() => s.push(c),
            _ if !s.ends_with('-') => s.push('-'),
            _ => {}
        }
    }
    let site = s.trim_matches('-');
    if site.is_empty() {
        format!("{prefix}-{id}.pdf")
    } else {
        format!("{prefix}-{site}-{id}.pdf")
    }
}

/// Soll als Text: „Sohlhöhe 296,00 m“, „Ebene 1,0 % nach 90°, durch 296,00 m“, „Soll-DGM „Planung““
fn design_text(d: &DesignSpec, names: &impl Fn(i64) -> String) -> String {
    match d {
        DesignSpec::Height { height } => format!("Sohlhöhe {}", text::height(*height)),
        DesignSpec::Slope {
            z,
            percent,
            azimuth,
            ..
        } => format!(
            "Ebene {} % nach {}°, durch {}",
            text::number(*percent, 1),
            text::number(*azimuth, 0),
            text::height(*z)
        ),
        DesignSpec::Surface { capture_id } => format!("Soll-DGM „{}“", names(*capture_id)),
    }
}

/// Was der Bericht braucht, aus der Datenbank gesammelt
struct Inputs {
    spec: JobSpec,
    result: Value,
    ctx: Context,
    regions: Vec<Region>,
    names: Vec<(i64, String)>,
    dir: PathBuf,
}

async fn inputs(app: &AppRef, id: i64) -> ApiResult<Inputs> {
    let job = db::get_job(&app.db, id)
        .await?
        .filter(|j| j.status == status::DONE)
        .ok_or_else(ApiError::not_found)?;
    let site = db::get_site(&app.db, job.site_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    // Arten ohne Bericht (z. B. Photogrammetrie) haben keine JobSpec
    let spec = JobSpec::parse(&job.kind, &serde_json::from_str(&job.params)?)
        .map_err(|_| ApiError::not_found())?;
    let result: Value = serde_json::from_str(job.result.as_deref().unwrap_or("null"))?;
    let mut names = Vec::new();
    for c in spec.captures() {
        if let Some(cap) = db::get_capture(&app.db, c).await? {
            names.push((c, cap.name));
        }
    }
    let regions = spec::regions(app, job.site_id, &spec.regions())
        .await
        .unwrap_or_default();
    Ok(Inputs {
        ctx: Context {
            site: site.name,
            subject: String::new(),
            finished_at: job.finished_at.unwrap_or(job.created_at),
            epsg: site.epsg,
            cell: result["cell"].as_f64().unwrap_or(0.0),
        },
        spec,
        result,
        regions,
        names,
        dir: app.cfg.job_dir(id),
    })
}

fn map_of(
    dir: &std::path::Path,
    file: &str,
    regions: &[Region],
    kind: map::Kind,
    caption: &str,
) -> Option<doc::Map> {
    let g = geotiff::read(&dir.join(file)).ok()?.grid;
    Some(map::build(&g, regions, kind, caption.into()))
}

/// Inhalt und Dateiname je Auftragsart; `None` für Arten ohne Bericht
fn document(mut i: Inputs) -> Result<Option<(doc::Doc, &'static str)>> {
    let name = |id: i64| {
        i.names
            .iter()
            .find(|(c, _)| *c == id)
            .map_or_else(|| format!("Aufnahme {id}"), |(_, n)| n.clone())
    };
    let north = "Norden oben. Weiß: keine Daten oder außerhalb der Bereiche.";
    Ok(Some(match &i.spec {
        JobSpec::Volume(v) => {
            i.ctx.subject = format!("{} → {}", name(v.before), name(v.after));
            let caption = format!("Höhenänderung nachher − vorher. {north}");
            let m = map_of(
                &i.dir,
                "diff.tif",
                &i.regions,
                map::Kind::Difference,
                &caption,
            );
            (build::volume_doc(&i.ctx, &i.result, m)?, "Mengenbericht")
        }
        JobSpec::Stockpile(s) => {
            i.ctx.subject = format!("Haufwerk auf „{}“", name(s.capture_id));
            let base = build::base_text(&s.base);
            (build::stockpile_doc(&i.ctx, &i.result, &base)?, "Haufwerk")
        }
        JobSpec::Compare(c) => {
            let design = design_text(&c.design, &name);
            i.ctx.subject = format!("{} gegen {design}", name(c.capture_id));
            let caption = format!("Abweichung Ist − Soll. {north}");
            let m = map_of(
                &i.dir,
                "abweichung.tif",
                &i.regions,
                map::Kind::Deviation,
                &caption,
            );
            (
                build::compare_doc(&i.ctx, &i.result, &design, m)?,
                "Soll-Ist",
            )
        }
        JobSpec::Series(s) => {
            let n = s.capture_ids.len();
            i.ctx.subject = format!("{n} Aufnahmen ab „{}“", name(s.capture_ids[0]));
            (build::series_doc(&i.ctx, &i.result)?, "Zeitreihe")
        }
        JobSpec::Inspect(_) | JobSpec::Dem(_) | JobSpec::Export(_) => return Ok(None),
    }))
}

pub async fn job_report(State(app): State<AppRef>, Path(id): Path<i64>) -> ApiResult<Response> {
    let i = inputs(&app, id).await?;
    let site = i.ctx.site.clone();
    let made = tokio::task::spawn_blocking(move || {
        Ok::<_, anyhow::Error>(document(i)?.map(|(d, prefix)| (pdf::render(&d), prefix)))
    })
    .await
    .map_err(anyhow::Error::from)??;
    let (bytes, prefix) = made.ok_or_else(ApiError::not_found)?;
    let disposition = format!("attachment; filename=\"{}\"", file_name(prefix, &site, id));
    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        bytes,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_file_names() {
        assert_eq!(
            file_name("Mengenbericht", "Kirchstraße 12", 7),
            "Mengenbericht-Kirchstrasse-12-7.pdf"
        );
        assert_eq!(
            file_name("Haufwerk", "Ölweg / Süd", 3),
            "Haufwerk-Oelweg-Sued-3.pdf"
        );
        assert_eq!(file_name("Zeitreihe", "„“", 1), "Zeitreihe-1.pdf");
    }

    #[test]
    fn design_texts() {
        let names = |_| "Planung".to_string();
        assert_eq!(
            design_text(&DesignSpec::Height { height: 296.0 }, &names),
            "Sohlhöhe 296,00 m"
        );
        let slope = DesignSpec::Slope {
            x: 0.0,
            y: 0.0,
            z: 296.5,
            percent: 1.0,
            azimuth: 90.0,
        };
        assert_eq!(
            design_text(&slope, &names),
            "Ebene 1,0 % nach 90°, durch 296,50 m"
        );
        assert_eq!(
            design_text(&DesignSpec::Surface { capture_id: 4 }, &names),
            "Soll-DGM „Planung“"
        );
    }
}
