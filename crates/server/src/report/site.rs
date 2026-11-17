//! Berichte einer Baustelle neben den Berechnungen, als reine Funktionen: Dokumentation (Zwangspunkte gegen die
//! letzte Aufnahme, Notizen, Fotos) und Plan/Ist (Höhenpunkte eines DXF-Plans gegen die letzte Aufnahme).

use super::build::reference;
use super::doc::{col, Block, Doc, Kpi, Table, Tone};
use super::entries;
pub use super::entries::Entry;
use super::text::{cm, date_time, height, number};
use crate::dashboard::CONSTRAINT_TOLERANCE;

/// Kopf: Baustelle, Stand, Bezugssystem und gegen welche Aufnahme geprüft wurde
pub struct SiteContext {
    pub site: String,
    pub epsg: i64,
    pub now: i64,
    /// Name der letzten geprüften Aufnahme, falls es eine gibt
    pub against: Option<String>,
}

pub(crate) fn head(ctx: &SiteContext, title: &str, subtitle: String) -> Doc {
    Doc {
        title: title.into(),
        subtitle,
        meta: vec![
            ("Baustelle".into(), ctx.site.clone()),
            ("Stand".into(), date_time(ctx.now)),
            ("Bezugssystem".into(), reference(ctx.epsg)),
            (
                "Ist aus".into(),
                ctx.against
                    .clone()
                    .unwrap_or_else(|| "– (keine Aufnahme)".into()),
            ),
        ],
        kpis: Vec::new(),
        blocks: Vec::new(),
    }
}

/// Blickrichtung als „135° (SO)“
pub fn compass(deg: f64) -> String {
    const NAMES: [&str; 8] = ["N", "NO", "O", "SO", "S", "SW", "W", "NW"];
    let d = (deg.round() as i64).rem_euclid(360);
    format!("{d}° ({})", NAMES[((d as f64 / 45.0).round() as usize) % 8])
}

/// „+5,0 cm“ / „−2,0 cm“: Ist − Soll mit Vorzeichen
pub fn signed_cm(m: f64) -> String {
    let v = cm(m.abs());
    if m >= 0.0005 {
        format!("+{v}")
    } else if m <= -0.0005 {
        format!("−{v}")
    } else {
        v
    }
}

pub fn position(x: Option<f64>, y: Option<f64>) -> String {
    match (x, y) {
        (Some(x), Some(y)) => format!("E {} · N {}", number(x, 2), number(y, 2)),
        _ => "ohne Lage".into(),
    }
}

pub fn documentation_doc(ctx: &SiteContext, entries: &[Entry]) -> Doc {
    let mut d = head(
        ctx,
        "Dokumentation",
        "Zwangspunkte, Leitungen, Notizen und Fotos".into(),
    );
    d.kpis = entries::kpis(entries);
    for part in [
        entries::constraints,
        entries::lines,
        entries::notes,
        entries::photos,
    ] {
        d.blocks.extend(part(entries));
    }
    if entries.is_empty() {
        d.blocks.push(Block::Text(
            "Zu dieser Baustelle ist noch nichts dokumentiert.".into(),
        ));
    }
    d
}

/// Ein Höhenpunkt des Plans mit Gelände an der Stelle
pub struct Spot {
    pub layer: String,
    pub at: [f64; 3],
    pub ist: Option<f64>,
}

pub fn plan_doc(ctx: &SiteContext, plan: &str, spots: &[Spot]) -> Doc {
    let diffs: Vec<f64> = spots
        .iter()
        .filter_map(|s| s.ist.map(|i| i - s.at[2]))
        .collect();
    let within = diffs
        .iter()
        .filter(|d| d.abs() <= CONSTRAINT_TOLERANCE)
        .count();
    let mean = (!diffs.is_empty()).then(|| diffs.iter().sum::<f64>() / diffs.len() as f64);
    let max = diffs
        .iter()
        .copied()
        .max_by(|a, b| a.abs().total_cmp(&b.abs()));
    let mut d = head(ctx, "Plan/Ist", format!("Höhenpunkte aus „{plan}“"));
    d.kpis = vec![
        Kpi::new(
            &format!("In Toleranz (± {})", cm(CONSTRAINT_TOLERANCE)),
            format!("{within} / {}", diffs.len()),
            if within == diffs.len() {
                Tone::Ok
            } else {
                Tone::Removed
            },
        ),
        Kpi::new(
            "Mittlere Abweichung",
            mean.map(signed_cm).unwrap_or_else(|| "–".into()),
            Tone::Neutral,
        ),
        Kpi::new(
            "Größte Abweichung",
            max.map(signed_cm).unwrap_or_else(|| "–".into()),
            Tone::Neutral,
        ),
    ];
    if spots.is_empty() {
        d.blocks
            .push(Block::Text("Der Plan enthält keine Höhenpunkte.".into()));
        return d;
    }
    let outside = spots.len() - diffs.len();
    if outside > 0 {
        d.blocks.push(Block::Note(format!(
            "{outside} Höhenpunkt(e) liegen außerhalb der Aufnahme oder ohne Daten."
        )));
    }
    d.blocks.push(Block::Heading("Höhenpunkte".into()));
    d.blocks.push(Block::Table(Table {
        columns: vec![
            col("Layer", false, 1.6),
            col("Lage", false, 2.8),
            col("Soll", true, 1.1),
            col("Ist", true, 1.1),
            col("Ist − Soll", true, 1.1),
        ],
        rows: spots
            .iter()
            .map(|s| {
                vec![
                    s.layer.clone(),
                    position(Some(s.at[0]), Some(s.at[1])),
                    height(s.at[2]),
                    s.ist.map(height).unwrap_or_else(|| "–".into()),
                    s.ist
                        .map(|i| signed_cm(i - s.at[2]))
                        .unwrap_or_else(|| "–".into()),
                ]
            })
            .collect(),
        total: None,
    }));
    d.blocks.push(Block::Text(
        "Ist: Gelände der letzten Aufnahme, bilinear an der Stelle des Höhenpunkts. Positiv: Gelände liegt höher als geplant (noch abtragen)."
            .into(),
    ));
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> SiteContext {
        SiteContext {
            site: "Kirchstraße 12".into(),
            epsg: 25832,
            now: 1_795_000_000_000,
            against: Some("Aushub fertig".into()),
        }
    }

    fn entry(kind: &str, title: &str, z: Option<f64>, ist: Option<f64>) -> Entry {
        Entry {
            kind: kind.into(),
            title: title.into(),
            note: String::new(),
            x: Some(512_020.0),
            y: Some(5_403_020.0),
            z,
            at: 1_795_000_000_000,
            ist,
            photo: None,
            line: None,
            direction: None,
        }
    }

    #[test]
    fn compass_directions() {
        assert_eq!(compass(135.377), "135° (SO)");
        assert_eq!(compass(359.6), "0° (N)");
        assert_eq!(compass(-90.0), "270° (W)");
    }

    #[test]
    fn signed_centimetres() {
        assert_eq!(signed_cm(0.05), "+5,0 cm");
        assert_eq!(signed_cm(-0.02), "−2,0 cm");
        assert_eq!(signed_cm(0.0001), "0,0 cm");
    }

    #[test]
    fn documentation_checks_constraints() {
        // Sohle 298,20: Kanal soll 298,15 (+5 cm verletzt), Schacht 298,19 (+1 cm ok), einer ohne Gelände
        let entries = [
            entry("constraint", "Kanal", Some(298.15), Some(298.2)),
            entry("constraint", "Schacht", Some(298.19), Some(298.2)),
            entry("constraint", "draußen", Some(300.0), None),
            entry("note", "Bodenprobe", None, None),
            entry("photo", "Böschung Nord", None, None),
            Entry {
                line: Some((11.0, 3)),
                ..entry("line", "Kanal DN 300", None, None)
            },
        ];
        let d = documentation_doc(&ctx(), &entries);
        assert_eq!(d.kpis[0].value, "1 / 3");
        assert_eq!(d.kpis[0].tone, Tone::Removed);
        let t = d.texts();
        for s in [
            "Kanal DN 300",
            "11,00 m",
            "+5,0 cm",
            "+1,0 cm",
            "verletzt",
            "eingehalten",
            "nicht geprüft",
            "Bodenprobe",
        ] {
            assert!(t.iter().any(|x| x == s), "{s} fehlt");
        }
        assert!(
            t.iter().any(|x| x.starts_with("Böschung Nord:")),
            "Foto ohne Bild als Text"
        );
        assert!(t.iter().any(|x| x == "E 512.020,00 · N 5.403.020,00"));
    }

    #[test]
    fn plan_ist_statistics() {
        let spots = [
            Spot {
                layer: "Höhen".into(),
                at: [0.0, 0.0, 298.20],
                ist: Some(298.20),
            },
            Spot {
                layer: "Höhen".into(),
                at: [1.0, 0.0, 298.30],
                ist: Some(298.20),
            },
            Spot {
                layer: "Höhen".into(),
                at: [9.0, 9.0, 300.00],
                ist: None,
            },
        ];
        let d = plan_doc(&ctx(), "Lageplan.dxf", &spots);
        // Abweichungen 0 und −10 cm: eine in Toleranz, Mittel −5 cm, größte −10 cm
        assert_eq!(d.kpis[0].value, "1 / 2");
        assert_eq!(d.kpis[1].value, "−5,0 cm");
        assert_eq!(d.kpis[2].value, "−10,0 cm");
        assert!(matches!(&d.blocks[0], Block::Note(n) if n.starts_with("1 Höhenpunkt")));
        assert!(plan_doc(&ctx(), "leer", &[])
            .texts()
            .iter()
            .any(|t| t.contains("keine Höhenpunkte")));
    }
}
