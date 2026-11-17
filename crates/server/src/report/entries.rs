//! Bausteine aus der Dokumentation einer Baustelle: Zwangspunkte (gegen die letzte Aufnahme), Leitungen, Notizen,
//! Fotos. Der Dokumentationsbericht und der eigene Bericht setzen sie zusammen.

use super::doc::{col, Block, Kpi, Photo, Table, Tone};
use super::site::{compass, position, signed_cm};
use super::text::{cm, date_time, height, number};
use crate::dashboard::CONSTRAINT_TOLERANCE;

/// Ein Eintrag der Dokumentation; `ist` nur bei Zwangspunkten mit Gelände an der Stelle
pub struct Entry {
    pub kind: String,
    pub title: String,
    pub note: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    pub at: i64,
    pub ist: Option<f64>,
    /// Vorschaubild als JPEG (RGB) mit Größe
    pub photo: Option<(Vec<u8>, usize, usize)>,
    /// Linie: Länge in der Ebene und Zahl der Stützpunkte
    pub line: Option<(f64, usize)>,
    /// Foto: Blickrichtung in Grad von Gitter-Nord
    pub direction: Option<f64>,
}

pub fn status(e: &Entry) -> (&'static str, Option<f64>) {
    match (e.z, e.ist) {
        (Some(z), Some(ist)) => {
            let d = ist - z;
            (
                if d.abs() <= CONSTRAINT_TOLERANCE {
                    "eingehalten"
                } else {
                    "verletzt"
                },
                Some(d),
            )
        }
        _ => ("nicht geprüft", None),
    }
}

fn of<'a>(entries: &'a [Entry], kind: &str) -> Vec<&'a Entry> {
    entries.iter().filter(|e| e.kind == kind).collect()
}

/// Kennzahlen der Dokumentation: Zwangspunkte eingehalten, Notizen, Fotos
pub fn kpis(entries: &[Entry]) -> Vec<Kpi> {
    let constraints = of(entries, "constraint");
    let violated = constraints
        .iter()
        .filter(|e| status(e).0 == "verletzt")
        .count();
    let checked = constraints.iter().filter(|e| status(e).1.is_some()).count();
    vec![
        Kpi::new(
            "Zwangspunkte eingehalten",
            format!("{} / {}", checked - violated, constraints.len()),
            if violated > 0 {
                Tone::Removed
            } else {
                Tone::Ok
            },
        ),
        Kpi::new(
            "Notizen",
            of(entries, "note").len().to_string(),
            Tone::Neutral,
        ),
        Kpi::new(
            "Fotos",
            of(entries, "photo").len().to_string(),
            Tone::Neutral,
        ),
    ]
}

/// Zwangspunkte: Hinweis bei Verletzung, Tabelle, Erklärung
pub fn constraints(entries: &[Entry]) -> Vec<Block> {
    let constraints = of(entries, "constraint");
    if constraints.is_empty() {
        return Vec::new();
    }
    let violated = constraints
        .iter()
        .filter(|e| status(e).0 == "verletzt")
        .count();
    let mut b = Vec::new();
    if violated > 0 {
        b.push(Block::Note(format!(
            "{violated} Zwangspunkt(e) weichen mehr als {} von der Sollhöhe ab.",
            cm(CONSTRAINT_TOLERANCE)
        )));
    }
    b.push(Block::Heading("Zwangspunkte".into()));
    b.push(Block::Table(Table {
        columns: vec![
            col("Zwangspunkt", false, 2.2),
            col("Lage", false, 2.6),
            col("Soll", true, 1.1),
            col("Ist", true, 1.1),
            col("Ist − Soll", true, 1.1),
            col("Status", false, 1.3),
        ],
        rows: constraints
            .iter()
            .map(|e| {
                let (s, diff) = status(e);
                vec![
                    e.title.clone(),
                    position(e.x, e.y),
                    e.z.map(height).unwrap_or_default(),
                    e.ist.map(height).unwrap_or_else(|| "–".into()),
                    diff.map(signed_cm).unwrap_or_else(|| "–".into()),
                    s.into(),
                ]
            })
            .collect(),
        total: None,
    }));
    b.push(Block::Text(format!(
        "Ist: Gelände der letzten Aufnahme an der Stelle. Eingehalten bis ± {}.",
        cm(CONSTRAINT_TOLERANCE)
    )));
    b
}

pub fn lines(entries: &[Entry]) -> Vec<Block> {
    let lines = of(entries, "line");
    if lines.is_empty() {
        return Vec::new();
    }
    vec![
        Block::Heading("Leitungen".into()),
        Block::Table(Table {
            columns: vec![
                col("Leitung", false, 2.2),
                col("Anfang", false, 2.6),
                col("Länge", true, 1.2),
                col("Punkte", true, 0.9),
                col("Notiz", false, 2.6),
            ],
            rows: lines
                .iter()
                .map(|e| {
                    let (len, n) = e.line.unwrap_or((0.0, 0));
                    vec![
                        e.title.clone(),
                        position(e.x, e.y),
                        format!("{} m", number(len, 2)),
                        n.to_string(),
                        e.note.clone(),
                    ]
                })
                .collect(),
            total: None,
        }),
    ]
}

pub fn notes(entries: &[Entry]) -> Vec<Block> {
    let notes = of(entries, "note");
    if notes.is_empty() {
        return Vec::new();
    }
    vec![
        Block::Heading("Notizen".into()),
        Block::Table(Table {
            columns: vec![
                col("Notiz", false, 2.0),
                col("Lage", false, 2.4),
                col("Text", false, 4.0),
            ],
            rows: notes
                .iter()
                .map(|e| vec![e.title.clone(), position(e.x, e.y), e.note.clone()])
                .collect(),
            total: None,
        }),
    ]
}

/// Fotos mit Zeit, Lage, Blickrichtung und Notiz; ohne Vorschaubild als Textzeile
pub fn photos(entries: &[Entry]) -> Vec<Block> {
    let photos = of(entries, "photo");
    if photos.is_empty() {
        return Vec::new();
    }
    let mut b = vec![Block::Heading("Fotos".into())];
    for e in photos {
        let mut lines = vec![date_time(e.at), position(e.x, e.y)];
        if let Some(d) = e.direction {
            lines.push(format!("Blickrichtung {}", compass(d)));
        }
        if !e.note.is_empty() {
            lines.push(e.note.clone());
        }
        b.push(match &e.photo {
            Some((jpeg, cols, rows)) => Block::Photo(Photo {
                jpeg: jpeg.clone(),
                cols: *cols,
                rows: *rows,
                title: e.title.clone(),
                lines,
            }),
            None => Block::Text(format!("{}: {}", e.title, lines.join(" · "))),
        });
    }
    b
}
