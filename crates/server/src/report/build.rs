//! Bericht je Auftragsart aus dem gespeicherten Ergebnis (reine Funktionen, Texte wie in der Web-UI).

use super::doc::{col, Block, Doc, Kpi, Map, Table, Tone};
use super::text::{area, cm, date_time, height, number, percent, volume};
use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;

/// Anteil der Fläche ohne Daten, ab dem der Bericht warnt (wie `MISSING_WARN` in der UI)
const MISSING_WARN: f64 = 0.05;

/// Angaben zum Auftrag, die nicht im Ergebnis stehen
#[derive(Debug, Clone)]
pub struct Context {
    pub site: String,
    /// was verglichen wurde, z. B. „Urgelände → Aushub“
    pub subject: String,
    pub finished_at: i64,
    pub epsg: i64,
    pub cell: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct Vol {
    area_m2: f64,
    cut_m3: f64,
    fill_m3: f64,
    net_m3: f64,
    cells_used: u64,
    cells_missing: u64,
}

impl Vol {
    fn missing(&self) -> f64 {
        let all = self.cells_used + self.cells_missing;
        if all == 0 {
            0.0
        } else {
            self.cells_missing as f64 / all as f64
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RegionVol {
    name: String,
    boundary_area_m2: f64,
    #[serde(flatten)]
    v: Vol,
}

#[derive(Debug, Clone, Deserialize)]
struct Volumes {
    regions: Vec<RegionVol>,
    total: Vol,
}

#[derive(Debug, Clone, Deserialize)]
struct Bin {
    from: Option<f64>,
    to: Option<f64>,
    percent: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct Deviation {
    min: f64,
    max: f64,
    mean: f64,
    rms: f64,
    std_dev: f64,
    tolerance: f64,
    within_tolerance_percent: f64,
    above_tolerance_percent: f64,
    below_tolerance_percent: f64,
    histogram: Vec<Bin>,
    volume: Vol,
}

#[derive(Debug, Clone, Deserialize)]
struct Step {
    label: String,
    since_start: Vol,
    since_previous: Vol,
}

fn reference(epsg: i64) -> String {
    if epsg == 25832 {
        "ETRS89/UTM32 (EPSG:25832), DHHN2016".into()
    } else {
        format!("EPSG:{epsg}, DHHN2016")
    }
}

fn head(ctx: &Context, title: &str, extra: Vec<(String, String)>) -> Doc {
    let mut meta = vec![
        ("Baustelle".to_string(), ctx.site.clone()),
        ("Berechnet".to_string(), date_time(ctx.finished_at)),
        ("Bezugssystem".to_string(), reference(ctx.epsg)),
        ("Rasterweite".to_string(), cm(ctx.cell)),
    ];
    meta.extend(extra);
    Doc {
        title: title.into(),
        subtitle: ctx.subject.clone(),
        meta,
        kpis: Vec::new(),
        blocks: Vec::new(),
    }
}

fn coverage(v: &Vol) -> String {
    percent((1.0 - v.missing()) * 100.0)
}

const COVERAGE_NOTE: &str =
    "In mindestens einem Bereich fehlen für mehr als 5 % der Fläche Daten. \
Diese Stellen sind nicht mitgerechnet – prüfe, ob die Aufnahmen den Bereich ganz abdecken.";

/// Zeilen je Bereich und Summe; ohne Bereiche nur die Summe über die ganze Aufnahme
fn rows(r: &Volumes, cells: impl Fn(&str, f64, &Vol) -> Vec<String>) -> Table {
    let total_area = if r.regions.is_empty() {
        r.total.area_m2
    } else {
        r.regions.iter().map(|x| x.boundary_area_m2).sum()
    };
    let total_name = if r.regions.is_empty() {
        "Gesamte Aufnahme"
    } else {
        "Summe"
    };
    Table {
        columns: Vec::new(),
        rows: r
            .regions
            .iter()
            .map(|x| cells(&x.name, x.boundary_area_m2, &x.v))
            .collect(),
        total: Some(cells(total_name, total_area, &r.total)),
    }
}

fn needs_note(r: &Volumes) -> bool {
    r.regions
        .iter()
        .map(|x| &x.v)
        .chain([&r.total])
        .any(|v| v.missing() > MISSING_WARN)
}

/// Aushub: Abtrag, Auftrag, Bilanz je Bereich, Karte der Höhenänderung
pub fn volume_doc(ctx: &Context, result: &Value, map: Option<Map>) -> Result<Doc> {
    let r: Volumes = Deserialize::deserialize(result)?;
    let n = r.regions.len().to_string();
    let mut d = head(ctx, "Mengenbericht", vec![("Bereiche".into(), n)]);
    d.kpis = vec![
        Kpi::new("Abtrag", volume(r.total.cut_m3), Tone::Removed),
        Kpi::new("Auftrag", volume(r.total.fill_m3), Tone::Added),
        Kpi::new("Bilanz", volume(r.total.net_m3), Tone::Neutral),
    ];
    if needs_note(&r) {
        d.blocks.push(Block::Note(COVERAGE_NOTE.into()));
    }
    let mut t = rows(&r, |name, a, v| {
        vec![
            name.into(),
            area(a),
            volume(v.cut_m3),
            volume(v.fill_m3),
            volume(v.net_m3),
            coverage(v),
        ]
    });
    t.columns = vec![
        col("Bereich", false, 2.2),
        col("Fläche", true, 1.2),
        col("Abtrag", true, 1.2),
        col("Auftrag", true, 1.2),
        col("Bilanz", true, 1.2),
        col("Daten", true, 0.9),
    ];
    d.blocks.extend([
        Block::Heading("Massen je Bereich".into()),
        Block::Table(t),
        Block::Text(format!(
            "Rastermethode, Zellgröße {}. Abtrag: Gelände nachher tiefer als vorher, Auftrag: höher. \
             Bilanz = Auftrag − Abtrag. „Daten“ ist der Anteil der Fläche, den beide Aufnahmen \
             abdecken; Lücken sind nicht mitgerechnet.",
            cm(ctx.cell)
        )),
    ]);
    if let Some(m) = map {
        d.blocks
            .extend([Block::Heading("Höhenänderung".into()), Block::Map(m)]);
    }
    Ok(d)
}

/// Haufwerk: Volumen über und unter der Basisfläche je Bereich
pub fn stockpile_doc(ctx: &Context, result: &Value, base: &str) -> Result<Doc> {
    let r: Volumes = Deserialize::deserialize(result)?;
    let mut d = head(ctx, "Haufwerk", vec![("Basisfläche".into(), base.into())]);
    d.kpis = vec![
        Kpi::new("Volumen", volume(r.total.fill_m3), Tone::Added),
        Kpi::new("Unter der Basis", volume(r.total.cut_m3), Tone::Removed),
        Kpi::new("Fläche", area(r.total.area_m2), Tone::Neutral),
    ];
    if needs_note(&r) {
        d.blocks.push(Block::Note(COVERAGE_NOTE.into()));
    }
    let mut t = rows(&r, |name, a, v| {
        vec![
            name.into(),
            area(a),
            volume(v.fill_m3),
            volume(v.cut_m3),
            coverage(v),
        ]
    });
    t.columns = vec![
        col("Bereich", false, 2.4),
        col("Fläche", true, 1.2),
        col("Volumen", true, 1.3),
        col("Unter der Basis", true, 1.4),
        col("Daten", true, 0.9),
    ];
    d.blocks.extend([
        Block::Heading("Massen je Bereich".into()),
        Block::Table(t),
        Block::Text(format!(
            "Rastermethode, Zellgröße {}. Volumen über der Basisfläche je Bereich; \
             „Unter der Basis“ sind Mulden, die tiefer liegen als die Basis.",
            cm(ctx.cell)
        )),
    ]);
    Ok(d)
}

/// Lage einer Histogrammklasse zur Toleranz (wie `binSide` in der UI)
fn side(b: &Bin, tol: f64) -> &'static str {
    let mid = match (b.from, b.to) {
        (None, Some(t)) => t - 1.0,
        (Some(f), None) => f + 1.0,
        (Some(f), Some(t)) => (f + t) / 2.0,
        (None, None) => 0.0,
    };
    if mid.abs() <= tol {
        "in Toleranz"
    } else if mid < 0.0 {
        "zu tief"
    } else {
        "zu hoch"
    }
}

/// Klasse in cm: „< −48“, „−3 … 3“, „> 48“
fn bin_label(b: &Bin) -> String {
    let c = |v: f64| number((v * 1000.0).round() / 10.0, 1).replace(",0", "");
    match (b.from, b.to) {
        (None, Some(t)) => format!("< {}", c(t)),
        (Some(f), None) => format!("> {}", c(f)),
        (Some(f), Some(t)) => format!("{} … {}", c(f), c(t)),
        (None, None) => "–".into(),
    }
}

/// Soll/Ist: Anteile in Toleranz, Kennwerte, Restmassen, Verteilung, Karte der Abweichung
pub fn compare_doc(ctx: &Context, result: &Value, design: &str, map: Option<Map>) -> Result<Doc> {
    let dv: Deviation = Deserialize::deserialize(&result["deviation"])?;
    let region = result["region"].as_str().unwrap_or("ganze Aufnahme");
    let mut d = head(
        ctx,
        "Soll/Ist-Vergleich",
        vec![
            ("Soll".into(), design.into()),
            ("Bereich".into(), region.into()),
        ],
    );
    let tol = dv.tolerance;
    d.kpis = vec![
        Kpi::new(
            &format!("In Toleranz (± {})", cm(tol)),
            percent(dv.within_tolerance_percent),
            Tone::Ok,
        ),
        Kpi::new(
            "Zu hoch",
            percent(dv.above_tolerance_percent),
            Tone::Removed,
        ),
        Kpi::new("Zu tief", percent(dv.below_tolerance_percent), Tone::Added),
    ];
    let pairs = |rows: Vec<(&str, String)>| Table {
        columns: vec![col("", false, 2.0), col("", true, 1.0)],
        rows: rows
            .into_iter()
            .map(|(k, v)| vec![k.to_string(), v])
            .collect(),
        total: None,
    };
    let mut stats = pairs(vec![
        ("Mittlere Abweichung", cm(dv.mean)),
        ("Quadratisches Mittel", cm(dv.rms)),
        ("Standardabweichung", cm(dv.std_dev)),
        ("Kleinste Abweichung", cm(dv.min)),
        ("Größte Abweichung", cm(dv.max)),
    ]);
    stats.columns = vec![col("Kennwert", false, 2.0), col("Ist − Soll", true, 1.0)];
    // Ist − Soll > 0 ist zu hoch: das muss noch weg (in der Differenz heißt das „Auftrag“)
    let mut rest = pairs(vec![
        ("Noch abzutragen", volume(dv.volume.fill_m3)),
        ("Noch aufzufüllen", volume(dv.volume.cut_m3)),
        ("Fläche mit Daten", area(dv.volume.area_m2)),
    ]);
    rest.columns = vec![col("Restmassen", false, 2.0), col("Menge", true, 1.0)];
    let hist = Table {
        columns: vec![
            col("Abweichung in cm", false, 1.6),
            col("Lage", false, 1.2),
            col("Anteil der Fläche", true, 1.2),
        ],
        rows: dv
            .histogram
            .iter()
            .map(|b| vec![bin_label(b), side(b, tol).into(), percent(b.percent)])
            .collect(),
        total: None,
    };
    d.blocks.extend([
        Block::Heading("Kennwerte".into()),
        Block::Table(stats),
        Block::Heading("Restmassen".into()),
        Block::Table(rest),
        Block::Heading("Verteilung der Abweichung".into()),
        Block::Table(hist),
    ]);
    if let Some(m) = map {
        d.blocks
            .extend([Block::Heading("Abweichung".into()), Block::Map(m)]);
    }
    Ok(d)
}

/// Zeitreihe: Abtrag seit Beginn und seit der vorigen Aufnahme
pub fn series_doc(ctx: &Context, result: &Value) -> Result<Doc> {
    let steps: Vec<Step> = Deserialize::deserialize(&result["steps"])?;
    let base = result["base"].as_str().unwrap_or("–");
    let region = result["region"].as_str().unwrap_or("ganze Aufnahme");
    let mut d = head(
        ctx,
        "Zeitreihe",
        vec![
            ("Urgelände".into(), base.into()),
            ("Bereich".into(), region.into()),
        ],
    );
    if let Some(last) = steps.last() {
        d.kpis = vec![
            Kpi::new(
                "Abtrag seit Beginn",
                volume(last.since_start.cut_m3),
                Tone::Removed,
            ),
            Kpi::new(
                "Seit der vorigen Aufnahme",
                volume(last.since_previous.cut_m3),
                Tone::Removed,
            ),
            Kpi::new("Aufnahmen", (steps.len() + 1).to_string(), Tone::Neutral),
        ];
    }
    let t = Table {
        columns: vec![
            col("Aufnahme", false, 2.0),
            col("Abtrag seit Beginn", true, 1.4),
            col("Auftrag seit Beginn", true, 1.4),
            col("Bilanz", true, 1.1),
            col("Seit der vorigen", true, 1.3),
        ],
        rows: steps
            .iter()
            .map(|s| {
                vec![
                    s.label.clone(),
                    volume(s.since_start.cut_m3),
                    volume(s.since_start.fill_m3),
                    volume(s.since_start.net_m3),
                    volume(s.since_previous.cut_m3),
                ]
            })
            .collect(),
        total: None,
    };
    d.blocks.extend([
        Block::Heading("Fortschritt".into()),
        Block::Table(t),
        Block::Text(format!(
            "Jede Aufnahme gegen das Urgelände „{base}“ und gegen die vorige, Rastermethode, \
             Zellgröße {}.",
            cm(ctx.cell)
        )),
    ]);
    Ok(d)
}

/// Beschreibung der Basis eines Haufwerks aus den Auftragsparametern
pub fn base_text(base: &Value) -> String {
    match base {
        Value::String(s) if s == "lowest" => "Tiefster Punkt am Rand".into(),
        Value::Number(n) => format!("Höhe {}", height(n.as_f64().unwrap_or(0.0))),
        _ => "Ebene durch den Rand".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx() -> Context {
        Context {
            site: "Kirchstraße 12".into(),
            subject: "Urgelände → Aushub".into(),
            finished_at: 1_794_906_000_000,
            epsg: 25832,
            cell: 0.25,
        }
    }

    fn vol(cut: f64, fill: f64, missing: u64) -> Value {
        json!({ "area_m2": 900.0, "cut_m3": cut, "fill_m3": fill, "net_m3": fill - cut,
                "cells_used": 100, "cells_missing": missing, "cell_size_m": 0.25 })
    }

    fn region(name: &str, area: f64, cut: f64, fill: f64, missing: u64) -> Value {
        let mut v = vol(cut, fill, missing);
        v["name"] = json!(name);
        v["boundary_area_m2"] = json!(area);
        v
    }

    #[test]
    fn volume_report_rows_and_total() {
        let result = json!({
            "regions": [region("Grube", 900.0, 800.05, 0.0, 0), region("Bagger", 100.0, 1.2, 3.4, 0)],
            "total": vol(801.25, 3.4, 0), "cell": 0.25,
        });
        let d = volume_doc(&ctx(), &result, None).unwrap();
        assert_eq!(d.kpis[0].value, "801,3 m³");
        assert_eq!(d.kpis[2].value, "−797,9 m³");
        let Block::Table(t) = &d.blocks[1] else {
            panic!("{:?}", d.blocks)
        };
        assert_eq!(
            t.rows[0],
            [
                "Grube",
                "900,0 m²",
                "800,1 m³",
                "0,0 m³",
                "−800,1 m³",
                "100,0 %"
            ]
        );
        // Summe der Grenzflächen, nicht der Datenfläche
        assert_eq!(t.total.as_ref().unwrap()[..2], ["Summe", "1.000,0 m²"]);
        assert!(d.texts().iter().any(|s| s.contains("Zellgröße 25,0 cm")));
        assert!(!d.blocks.iter().any(|b| matches!(b, Block::Note(_))));
    }

    #[test]
    fn coverage_note_above_five_percent() {
        // 6 von 106 Zellen fehlen = 5,7 %
        let result = json!({ "regions": [], "total": vol(10.0, 0.0, 6), "cell": 0.25 });
        let d = volume_doc(&ctx(), &result, None).unwrap();
        assert!(matches!(d.blocks[0], Block::Note(_)));
        let Block::Table(t) = &d.blocks[2] else {
            panic!()
        };
        assert!(t.rows.is_empty());
        assert_eq!(t.total.as_ref().unwrap()[0], "Gesamte Aufnahme");
        assert_eq!(t.total.as_ref().unwrap()[5], "94,3 %");
    }

    #[test]
    fn stockpile_uses_fill_as_volume() {
        let result = json!({ "regions": [region("Halde", 400.0, 0.5, 512.0, 0)],
                             "total": vol(0.5, 512.0, 0), "cell": 0.25 });
        let d = stockpile_doc(&ctx(), &result, &base_text(&json!(299.0))).unwrap();
        assert_eq!(d.kpis[0].value, "512,0 m³");
        assert!(d
            .meta
            .contains(&("Basisfläche".into(), "Höhe 299,00 m".into())));
        assert_eq!(base_text(&json!("lowest")), "Tiefster Punkt am Rand");
        assert_eq!(base_text(&json!("plane")), "Ebene durch den Rand");
    }

    #[test]
    fn compare_report_sides_and_rest_volumes() {
        let mut dev = json!({ "volume": vol(2.0, 12.0, 0) });
        for (k, v) in [
            ("min", json!(-0.02)),
            ("max", json!(0.08)),
            ("mean", json!(0.03)),
            ("rms", json!(0.035)),
            ("std_dev", json!(0.01)),
            ("tolerance", json!(0.05)),
            ("within_tolerance_percent", json!(81.25)),
            ("above_tolerance_percent", json!(18.75)),
            ("below_tolerance_percent", json!(0.0)),
            (
                "histogram",
                json!([
                    { "from": null, "to": -0.48, "cells": 0, "percent": 0.0 },
                    { "from": -0.03, "to": 0.03, "cells": 80, "percent": 81.25 },
                    { "from": 0.48, "to": null, "cells": 0, "percent": 0.0 },
                ]),
            ),
        ] {
            dev[k] = v;
        }
        let result = json!({ "deviation": dev, "region": "Sohle", "cell": 0.25 });
        let d = compare_doc(&ctx(), &result, "Sohlhöhe 296,00 m", None).unwrap();
        assert_eq!(d.kpis[0].label, "In Toleranz (± 5,0 cm)");
        assert_eq!(d.kpis[0].value, "81,3 %");
        let tables: Vec<&Table> = d
            .blocks
            .iter()
            .filter_map(|b| {
                if let Block::Table(t) = b {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(tables[1].rows[0], ["Noch abzutragen", "12,0 m³"]);
        assert_eq!(tables[2].rows[0], ["< −48", "zu tief", "0,0 %"]);
        assert_eq!(tables[2].rows[1], ["−3 … 3", "in Toleranz", "81,3 %"]);
        assert_eq!(tables[2].rows[2], ["> 48", "zu hoch", "0,0 %"]);
    }

    #[test]
    fn series_rows() {
        let step = |label: &str, total: f64, step: f64| json!({ "label": label, "since_start": vol(total, 0.0, 0), "since_previous": vol(step, 0.0, 0) });
        let result = json!({ "base": "Urgelände", "region": null, "cell": 0.25,
                             "steps": [step("Woche 1", 400.0, 400.0), step("Woche 2", 800.0, 400.0)] });
        let d = series_doc(&ctx(), &result).unwrap();
        assert_eq!(d.kpis[0].value, "800,0 m³");
        assert_eq!(d.kpis[2].value, "3");
        let Block::Table(t) = &d.blocks[1] else {
            panic!()
        };
        assert_eq!(t.rows[1][4], "400,0 m³");
    }
}
