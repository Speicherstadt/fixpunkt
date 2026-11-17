//! Eigener Bericht einer Baustelle: Bausteine aus Überblick und Dokumentation in gewählter Reihenfolge, mit
//! eigenem Titel und optional Firmenname. `GET /api/sites/{id}/report?parts=…&title=…&company=…`

use super::doc::{Block, Doc};
use super::entries::{self, Entry};
use super::overview;
use super::site::{head, SiteContext};
use anyhow::{bail, Result};
use serde_json::Value;

/// Bausteine in der Grundreihenfolge
pub const PARTS: &[&str] = &[
    "kpis",
    "progress",
    "targets",
    "stock",
    "history",
    "compare",
    "attention",
    "constraints",
    "lines",
    "notes",
    "photos",
];

/// Bausteine aus der Anfrage: bekannte, jeder einmal, in der gewünschten Reihenfolge
pub fn parse_parts(s: &str) -> Result<Vec<&'static str>> {
    let mut out: Vec<&'static str> = Vec::new();
    for p in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let Some(known) = PARTS.iter().find(|k| **k == p) else {
            bail!("unbekannter Baustein: {p}");
        };
        if !out.contains(known) {
            out.push(known);
        }
    }
    if out.is_empty() {
        bail!("kein Baustein gewählt");
    }
    Ok(out)
}

fn blocks(part: &str, d: &Value, e: &[Entry]) -> Vec<Block> {
    match part {
        "progress" => overview::progress(d),
        "targets" => overview::targets(d),
        "stock" => overview::stock(d),
        "history" => overview::history(d),
        "compare" => overview::compare(d),
        "attention" => overview::attention(d),
        "constraints" => entries::constraints(e),
        "lines" => entries::lines(e),
        "notes" => entries::notes(e),
        "photos" => entries::photos(e),
        _ => Vec::new(),
    }
}

pub fn custom_doc(
    ctx: &SiteContext,
    title: &str,
    company: Option<&str>,
    parts: &[&str],
    d: &Value,
    e: &[Entry],
) -> Doc {
    let title = match title.trim() {
        "" => "Baustellenbericht",
        t => t,
    };
    let mut doc = head(ctx, title, ctx.site.clone());
    if let Some(c) = company.map(str::trim).filter(|c| !c.is_empty()) {
        doc.meta.insert(0, ("Erstellt von".into(), c.into()));
    }
    for p in parts {
        if *p == "kpis" {
            doc.kpis = overview::kpis(d);
        } else {
            doc.blocks.extend(blocks(p, d, e));
        }
    }
    if doc.blocks.is_empty() && doc.kpis.is_empty() {
        doc.blocks.push(Block::Text(
            "Für die gewählten Bausteine gibt es auf dieser Baustelle noch nichts.".into(),
        ));
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx() -> SiteContext {
        SiteContext {
            site: "Kirchstraße 12".into(),
            epsg: 25832,
            now: 1_795_000_000_000,
            against: Some("Aushub fertig".into()),
        }
    }

    fn data() -> Value {
        json!({
            "captures": {"count": 3},
            "progress": [
                {"name": "Zwischenstand", "at": 1_794_300_000_000i64, "cut": 400.0, "fill": 0.5, "net": -399.5},
                {"name": "Aushub fertig", "at": 1_794_900_000_000i64, "cut": 800.1, "fill": 1.0, "net": -799.1}
            ],
            "targets": [{"name": "Grube", "material": "Boden Klasse 4", "kind": "cut", "target": 1000.0,
                         "done": 800.1, "percent": 80.01, "open": 199.9}],
            "materials": [{"material": "Boden Klasse 4", "target": 1000.0, "done": 800.1}],
            "stock": [{"material": "Oberboden", "current": 420.0, "change": 40.0, "at": 1_794_900_000_000i64}],
            "stockHistory": [{"material": "Oberboden", "points": [
                {"at": 1_794_300_000_000i64, "value": 380.0}, {"at": 1_794_900_000_000i64, "value": 420.0}]}],
            "latestCompare": {"within": 63.4, "tolerance": 0.05, "at": 1_794_900_000_000i64},
            "constraints": {"total": 2, "checked": 2, "against": "Aushub fertig",
                            "violated": [{"id": 1, "title": "Kanal", "diff": 0.05}]},
            "jobs": {"failed": 1},
        })
    }

    #[test]
    fn parts_in_the_chosen_order() {
        assert_eq!(parse_parts("stock, kpis,stock").unwrap(), ["stock", "kpis"]);
        assert!(parse_parts("kpis,gibts-nicht").is_err());
        assert!(parse_parts(" , ").is_err());
    }

    #[test]
    fn report_from_parts() {
        let doc = custom_doc(
            &ctx(),
            "Wochenbericht KW 47",
            Some("Muster Tiefbau GmbH"),
            &[
                "stock",
                "kpis",
                "targets",
                "attention",
                "history",
                "compare",
            ],
            &data(),
            &[],
        );
        assert_eq!(doc.title, "Wochenbericht KW 47");
        assert_eq!(
            doc.meta[0],
            ("Erstellt von".into(), "Muster Tiefbau GmbH".into())
        );
        assert_eq!(doc.kpis[1].value, "800,1 m³");
        assert_eq!(doc.kpis[3].value, "1 / 2");
        // Reihenfolge wie gewählt: Bestand vor Massenzielen
        let headings: Vec<&str> = doc
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Heading(h) => Some(h.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            headings,
            [
                "Bestand je Material",
                "Massenziele",
                "Braucht Aufmerksamkeit",
                "Verlauf je Material",
                "Letzter Soll/Ist-Vergleich"
            ]
        );
        let t = doc.texts();
        for s in [
            "420 m³",
            "+40 m³",
            "80 %",
            "200 m³",
            "Je Material: Boden Klasse 4 800 m³ von 1.000 m³.",
            "63,4 %",
            "± 5,0 cm",
            "+5,0 cm",
            "1 Berechnung(en) fehlgeschlagen",
            "380 m³",
        ] {
            assert!(t.iter().any(|x| x.contains(s)), "{s} fehlt in {t:?}");
        }
        // Grundtitel, und ohne Inhalt ein Hinweis
        let empty = custom_doc(&ctx(), " ", None, &["photos"], &json!({}), &[]);
        assert_eq!(empty.title, "Baustellenbericht");
        assert!(empty.texts().iter().any(|x| x.contains("noch nichts")));
    }
}
