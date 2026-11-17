//! Inhalt eines Berichts, unabhängig vom PDF: Kopf, Kennzahlen, Abschnitte mit Text, Tabellen, Karte.

/// Farbe einer Kennzahl wie in der Web-UI: Orange heißt Material weg, Blau Material dazu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Removed,
    Added,
    Ok,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Kpi {
    pub label: String,
    pub value: String,
    pub tone: Tone,
}

impl Kpi {
    pub fn new(label: &str, value: String, tone: Tone) -> Self {
        Kpi {
            label: label.into(),
            value,
            tone,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub title: String,
    /// rechtsbündig (Zahlen)
    pub right: bool,
    /// Anteil an der Tabellenbreite
    pub share: f32,
}

pub fn col(title: &str, right: bool, share: f32) -> Column {
    Column {
        title: title.into(),
        right,
        share,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
    /// fett mit Linie darüber
    pub total: Option<Vec<String>>,
}

/// Karte als RGB-Bild; Umrisse in Bildkoordinaten 0…1 (x nach rechts, y nach unten)
#[derive(Debug, Clone, PartialEq)]
pub struct Map {
    pub cols: usize,
    pub rows: usize,
    pub rgb: Vec<u8>,
    /// Breite des Bildes im Gelände in m (für die Maßstabsleiste)
    pub width_m: f64,
    pub outlines: Vec<(String, Vec<[f64; 2]>)>,
    /// Farbverlauf der Legende von links nach rechts und die Beschriftung darunter
    pub legend: Vec<[u8; 3]>,
    pub legend_labels: [String; 3],
    pub caption: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading(String),
    Text(String),
    /// hervorgehobener Hinweis (Rahmen in Warnfarbe)
    Note(String),
    Table(Table),
    Map(Map),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Doc {
    pub title: String,
    pub subtitle: String,
    /// Angaben im Kopf: Baustelle, Stand, Bezugssystem …
    pub meta: Vec<(String, String)>,
    pub kpis: Vec<Kpi>,
    pub blocks: Vec<Block>,
}

impl Doc {
    /// alle Texte des Berichts, für Tests und Suche
    pub fn texts(&self) -> Vec<String> {
        let mut out = vec![self.title.clone(), self.subtitle.clone()];
        for (k, v) in &self.meta {
            out.extend([k.clone(), v.clone()]);
        }
        for k in &self.kpis {
            out.extend([k.label.clone(), k.value.clone()]);
        }
        for b in &self.blocks {
            match b {
                Block::Heading(s) | Block::Text(s) | Block::Note(s) => out.push(s.clone()),
                Block::Table(t) => {
                    out.extend(t.columns.iter().map(|c| c.title.clone()));
                    out.extend(t.rows.iter().flatten().cloned());
                    out.extend(t.total.iter().flatten().cloned());
                }
                Block::Map(m) => {
                    out.push(m.caption.clone());
                    out.extend(m.legend_labels.iter().cloned());
                }
            }
        }
        out
    }
}
