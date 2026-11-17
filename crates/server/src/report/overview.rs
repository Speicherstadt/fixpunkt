//! Bausteine aus dem Überblick einer Baustelle (JSON von `dashboard::data`) für den eigenen Bericht: Kennzahlen,
//! Aushub je Befliegung, Massenziele, Lagerbestand und Verlauf, letzter Soll/Ist, was Aufmerksamkeit braucht.

use super::doc::{col, Block, Kpi, Table, Tone};
use super::site::signed_cm;
use super::text::{cm, date, number, percent, volume};
use serde_json::Value;

fn f(v: &Value) -> Option<f64> {
    v.as_f64()
}

fn m3(v: f64) -> String {
    format!("{} m³", number(v, 0))
}

/// „+40 m³“ / „−450 m³“
pub fn signed_m3(v: f64) -> String {
    match v.round() {
        r if r > 0.0 => format!("+{}", m3(r)),
        r if r < 0.0 => format!("−{}", m3(-r)),
        _ => "±0 m³".into(),
    }
}

pub fn kpis(d: &Value) -> Vec<Kpi> {
    let last = d["progress"].as_array().and_then(|p| p.last());
    let k = &d["constraints"];
    let violated = k["violated"].as_array().map_or(0, Vec::len) as i64;
    let total = k["total"].as_i64().unwrap_or(0);
    vec![
        Kpi::new(
            "Aufnahmen",
            d["captures"]["count"].as_i64().unwrap_or(0).to_string(),
            Tone::Neutral,
        ),
        Kpi::new(
            "Aushub bisher",
            last.and_then(|p| f(&p["cut"])).map_or("–".into(), volume),
            Tone::Removed,
        ),
        Kpi::new(
            // kurz: vier Kacheln nebeneinander
            "Soll/Ist in Toleranz",
            f(&d["latestCompare"]["within"]).map_or("–".into(), percent),
            Tone::Neutral,
        ),
        Kpi::new(
            "Zwangspunkte ok",
            if total > 0 {
                format!(
                    "{} / {total}",
                    k["checked"].as_i64().unwrap_or(0) - violated
                )
            } else {
                "–".into()
            },
            if violated > 0 {
                Tone::Removed
            } else {
                Tone::Ok
            },
        ),
    ]
}

pub fn progress(d: &Value) -> Vec<Block> {
    let Some(p) = d["progress"].as_array().filter(|p| !p.is_empty()) else {
        return Vec::new();
    };
    vec![
        Block::Heading("Aushub je Befliegung (gegen das Urgelände)".into()),
        Block::Table(Table {
            columns: vec![
                col("Befliegung", false, 2.4),
                col("Datum", false, 1.2),
                col("Abtrag", true, 1.2),
                col("Auftrag", true, 1.2),
                col("Netto", true, 1.2),
            ],
            rows: p
                .iter()
                .map(|x| {
                    vec![
                        x["name"].as_str().unwrap_or("").into(),
                        x["at"].as_i64().map(date).unwrap_or_default(),
                        f(&x["cut"]).map(volume).unwrap_or_default(),
                        f(&x["fill"]).map(volume).unwrap_or_default(),
                        f(&x["net"]).map(volume).unwrap_or_default(),
                    ]
                })
                .collect(),
            total: None,
        }),
    ]
}

pub fn targets(d: &Value) -> Vec<Block> {
    let Some(t) = d["targets"].as_array().filter(|t| !t.is_empty()) else {
        return Vec::new();
    };
    let mut b = vec![
        Block::Heading("Massenziele".into()),
        Block::Table(Table {
            columns: vec![
                col("Bereich", false, 1.8),
                col("Material", false, 1.8),
                col("Art", false, 0.9),
                col("Soll", true, 1.1),
                col("Erledigt", true, 1.1),
                col("Anteil", true, 0.9),
                col("Offen", true, 1.1),
            ],
            rows: t
                .iter()
                .map(|x| {
                    vec![
                        x["name"].as_str().unwrap_or("").into(),
                        x["material"].as_str().unwrap_or("–").into(),
                        if x["kind"] == "fill" {
                            "Auftrag"
                        } else {
                            "Abtrag"
                        }
                        .into(),
                        f(&x["target"]).map(m3).unwrap_or_default(),
                        f(&x["done"]).map_or("–".into(), m3),
                        f(&x["percent"]).map_or("–".into(), |p| format!("{} %", number(p, 0))),
                        f(&x["open"]).map_or("–".into(), m3),
                    ]
                })
                .collect(),
            total: None,
        }),
    ];
    let mats: Vec<String> = d["materials"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|x| {
            format!(
                "{} {} von {}",
                x["material"].as_str().unwrap_or(""),
                m3(f(&x["done"]).unwrap_or(0.0)),
                m3(f(&x["target"]).unwrap_or(0.0))
            )
        })
        .collect();
    if !mats.is_empty() {
        b.push(Block::Text(format!("Je Material: {}.", mats.join(" · "))));
    }
    b
}

pub fn stock(d: &Value) -> Vec<Block> {
    let rows: Vec<Vec<String>> = d["stock"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| {
            Some(vec![
                x["material"].as_str()?.into(),
                m3(f(&x["current"])?),
                f(&x["change"]).map_or("–".into(), signed_m3),
                x["at"].as_i64().map(date).unwrap_or_default(),
            ])
        })
        .collect();
    if rows.is_empty() {
        return Vec::new();
    }
    vec![
        Block::Heading("Bestand je Material".into()),
        Block::Table(Table {
            columns: vec![
                col("Material", false, 2.4),
                col("Bestand", true, 1.2),
                col("seit vorher", true, 1.2),
                col("Stand", true, 1.2),
            ],
            rows,
            total: None,
        }),
    ]
}

pub fn history(d: &Value) -> Vec<Block> {
    let rows: Vec<Vec<String>> = d["stockHistory"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|h| {
            let m = h["material"].as_str().unwrap_or("").to_string();
            h["points"].as_array().into_iter().flatten().map(move |p| {
                vec![
                    m.clone(),
                    p["at"].as_i64().map(date).unwrap_or_default(),
                    f(&p["value"]).map(m3).unwrap_or_default(),
                ]
            })
        })
        .collect();
    if rows.is_empty() {
        return Vec::new();
    }
    vec![
        Block::Heading("Verlauf je Material".into()),
        Block::Table(Table {
            columns: vec![
                col("Material", false, 2.4),
                col("Befliegung", false, 1.4),
                col("Bestand", true, 1.2),
            ],
            rows,
            total: None,
        }),
    ]
}

pub fn compare(d: &Value) -> Vec<Block> {
    let c = &d["latestCompare"];
    let Some(within) = f(&c["within"]) else {
        return Vec::new();
    };
    vec![
        Block::Heading("Letzter Soll/Ist-Vergleich".into()),
        Block::Text(format!(
            "{} der Fläche innerhalb ± {} (berechnet am {}).",
            percent(within),
            f(&c["tolerance"]).map_or("–".into(), cm),
            c["at"].as_i64().map(date).unwrap_or_default()
        )),
    ]
}

pub fn attention(d: &Value) -> Vec<Block> {
    let k = &d["constraints"];
    let against = k["against"].as_str().unwrap_or("–");
    let mut lines: Vec<String> = k["violated"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|v| {
            format!(
                "Zwangspunkt „{}“: {} gegen Soll (Aufnahme „{against}“)",
                v["title"].as_str().unwrap_or(""),
                f(&v["diff"]).map(signed_cm).unwrap_or_default()
            )
        })
        .collect();
    if let Some(n) = d["jobs"]["failed"].as_i64().filter(|n| *n > 0) {
        lines.push(format!("{n} Berechnung(en) fehlgeschlagen"));
    }
    if lines.is_empty() {
        return Vec::new();
    }
    // je Hinweis ein Kasten: der Umbruch im PDF kennt keine Zeilenwechsel
    std::iter::once(Block::Heading("Braucht Aufmerksamkeit".into()))
        .chain(lines.into_iter().map(Block::Note))
        .collect()
}
