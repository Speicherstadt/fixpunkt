//! Aufmaßblatt eines Abrechnungszeitraums: Positionen mit Menge, Einheitspreis und Betrag, Mengen je Bereich in
//! allen Einheiten, Abgleich mit Lieferscheinen und die Kennwerte der Umrechnung. Daten aus `billing::statement`.

use super::doc::{col, Block, Doc, Kpi, Table, Tone};
use super::site::{head, SiteContext};
use super::text::{date, number, percent, volume};
use serde_json::Value;

/// Preiseinheit als Text
pub fn unit_label(u: &str) -> &'static str {
    match u {
        "m3" => "m³",
        "m3_loose" => "m³ lose",
        "t" => "t",
        "loads" => "Ladungen",
        _ => "",
    }
}

pub fn euro(v: f64) -> String {
    format!("{} €", number(v, 2))
}

fn opt(v: &Value, f: impl Fn(f64) -> String) -> String {
    v.as_f64().map(f).unwrap_or_else(|| "–".into())
}

fn capture(v: &Value) -> String {
    format!(
        "{} ({})",
        v["name"].as_str().unwrap_or(""),
        v["at"].as_i64().map(date).unwrap_or_default()
    )
}

fn positions(s: &Value) -> Table {
    let ps = s["positions"].as_array().cloned().unwrap_or_default();
    let rows = ps
        .iter()
        .map(|p| {
            let unit = p["unit"].as_str().unwrap_or("m3");
            vec![
                p["lvPosition"].as_str().unwrap_or("–").into(),
                p["name"].as_str().unwrap_or("").into(),
                p["material"].as_str().unwrap_or("–").into(),
                match p["quantity"].as_f64() {
                    Some(q) => format!("{} {}", number(q, 1), unit_label(unit)),
                    None => opt(&p["period"]["bankM3"], volume),
                },
                opt(&p["unitPrice"], |v| {
                    format!("{}/{}", euro(v), unit_label(unit))
                }),
                opt(&p["amount"], euro),
            ]
        })
        .collect();
    Table {
        columns: vec![
            col("Position", false, 0.14),
            col("Bereich", false, 0.2),
            col("Material", false, 0.18),
            col("Menge", true, 0.16),
            col("Einheitspreis", true, 0.16),
            col("Betrag", true, 0.16),
        ],
        rows,
        total: Some(vec![
            "Summe netto".into(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            opt(&s["amount"], euro),
        ]),
    }
}

fn quantities(s: &Value) -> Table {
    let ps = s["positions"].as_array().cloned().unwrap_or_default();
    let n = |v: &Value, d| opt(v, |x| number(x, d));
    Table {
        columns: vec![
            col("Bereich", false, 0.22),
            col("bisher m³", true, 0.13),
            col("gesamt m³", true, 0.13),
            col("Zeitraum m³", true, 0.13),
            col("m³ lose", true, 0.13),
            col("t", true, 0.13),
            col("LKW", true, 0.13),
        ],
        rows: ps
            .iter()
            .map(|p| {
                let q = &p["period"];
                vec![
                    p["name"].as_str().unwrap_or("").into(),
                    n(&p["previous"], 1),
                    n(&p["total"], 1),
                    n(&q["bankM3"], 1),
                    n(&q["looseM3"], 1),
                    n(&q["tonnes"], 1),
                    n(&q["loads"], 0),
                ]
            })
            .collect(),
        total: None,
    }
}

fn checks(s: &Value) -> Vec<Block> {
    let cs = s["checks"].as_array().cloned().unwrap_or_default();
    if cs.is_empty() {
        return Vec::new();
    }
    let rows = cs
        .iter()
        .map(|c| {
            let delivered = c["delivered"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|d| {
                    format!(
                        "{} {}",
                        number(d[1].as_f64().unwrap_or(0.0), 1),
                        unit_label(d[0].as_str().unwrap_or(""))
                    )
                })
                .collect::<Vec<_>>()
                .join(" + ");
            vec![
                c["material"].as_str().unwrap_or("ohne Material").into(),
                c["count"].as_u64().unwrap_or(0).to_string(),
                delivered,
                opt(&c["deliveredBank"], volume),
                opt(&c["measuredBank"], volume),
                opt(&c["deviationPercent"], |p| {
                    format!("{}{}", if p > 0.0 { "+" } else { "" }, percent(p))
                }),
            ]
        })
        .collect();
    vec![
        Block::Heading("Abgleich mit Lieferscheinen".into()),
        Block::Text(
            "Lieferscheine im Zeitraum, umgerechnet in m³ fest (Dichte, Auflockerung, Nutzlast), gegen die \
             gemessene Menge. Eine Abweichung über etwa 10 % lohnt einen zweiten Blick."
                .into(),
        ),
        Block::Table(Table {
            columns: vec![
                col("Material", false, 0.2),
                col("Scheine", true, 0.1),
                col("geliefert", false, 0.22),
                col("in m³ fest", true, 0.16),
                col("gemessen", true, 0.16),
                col("Abweichung", true, 0.16),
            ],
            rows,
            total: None,
        }),
    ]
}

fn factors(settings: &[(String, Option<f64>, Option<f64>)], truck: Option<f64>) -> String {
    let mut parts: Vec<String> = settings
        .iter()
        .map(|(n, d, b)| {
            let d = d.map(|d| format!("Dichte {} t/m³", number(d, 2)));
            let b = b.map(|b| format!("Auflockerung {}", number(b, 2)));
            format!(
                "{n}: {}",
                [d, b].into_iter().flatten().collect::<Vec<_>>().join(", ")
            )
        })
        .collect();
    if let Some(t) = truck {
        parts.push(format!("LKW-Nutzlast {} t", number(t, 1)));
    }
    if parts.is_empty() {
        "Keine Kennwerte hinterlegt; Mengen in m³ fest.".into()
    } else {
        format!("Umrechnung: {}.", parts.join("; "))
    }
}

pub fn billing_doc(
    ctx: &SiteContext,
    s: &Value,
    materials: &[(String, Option<f64>, Option<f64>)],
) -> Doc {
    let period = format!("{} bis {}", capture(&s["from"]), capture(&s["to"]));
    let mut doc = head(ctx, "Aufmaß", period.clone());
    doc.meta.insert(1, ("Zeitraum".into(), period));
    doc.meta.push(("Urgelände".into(), capture(&s["base"])));
    let bank: f64 = s["positions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p["period"]["bankM3"].as_f64())
        .sum();
    doc.kpis = vec![
        Kpi::new("Menge im Zeitraum (fest)", volume(bank), Tone::Removed),
        Kpi::new("Betrag netto", opt(&s["amount"], euro), Tone::Neutral),
        Kpi::new(
            "Positionen",
            s["positions"].as_array().map_or(0, Vec::len).to_string(),
            Tone::Neutral,
        ),
    ];
    let missing: Vec<&str> = s["missing"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if !missing.is_empty() {
        doc.blocks.push(Block::Note(format!(
            "Für {} fehlt die Aushub-Berechnung gegen das Urgelände; die Mengen sind unvollständig.",
            missing.join(" und ")
        )));
    }
    if s["unconverted"].as_u64().is_some_and(|n| n > 0) {
        doc.blocks.push(Block::Note(
            "Für Preise je t oder m³ lose fehlen Dichte oder Auflockerung des Materials; diese Positionen haben \
             keinen Betrag."
                .into(),
        ));
    }
    doc.blocks.extend([
        Block::Heading("Positionen".into()),
        Block::Table(positions(s)),
        Block::Heading("Mengen je Bereich".into()),
        Block::Text(
            "Menge im Zeitraum = Aushub gegen das Urgelände bis zum Ende des Zeitraums minus bis zum Anfang; \
             gemessen im gewachsenen Zustand (fest)."
                .into(),
        ),
        Block::Table(quantities(s)),
    ]);
    doc.blocks.extend(checks(s));
    doc.blocks
        .push(Block::Text(factors(materials, s["truckT"].as_f64())));
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn statement_as_document() {
        let s = json!({
            "base": {"id": 1, "name": "Urgelände", "at": 0},
            "from": {"id": 2, "name": "Januar", "at": 2_592_000_000_i64},
            "to": {"id": 3, "name": "Februar", "at": 5_011_200_000_i64},
            "missing": [], "truckT": 15.0, "amount": 12740.0, "unpriced": 0,
            "positions": [
                {"name": "Grube", "lvPosition": "01.02.0010", "material": "Lehm", "unit": "t", "unitPrice": 12.5,
                 "quantity": 1000.0, "amount": 12500.0, "previous": 300.0, "total": 800.0,
                 "period": {"bankM3": 500.0, "looseM3": 625.0, "tonnes": 1000.0, "loads": 67.0}},
                {"name": "Rampe", "lvPosition": "01.02.0020", "material": "Lehm", "unit": "m3", "unitPrice": 8.0,
                 "quantity": 30.0, "amount": 240.0, "previous": 20.0, "total": 50.0,
                 "period": {"bankM3": 30.0, "looseM3": 37.5, "tonnes": 60.0, "loads": 4.0}},
            ],
            "checks": [{"material": "Lehm", "count": 3, "delivered": [["t", 600.0], ["loads", 10.0]],
                        "deliveredBank": 575.0, "measuredBank": 500.0, "deviationPercent": 15.0}],
        });
        let ctx = SiteContext {
            site: "Kirchstraße 12".into(),
            epsg: 25832,
            now: 0,
            against: Some("Februar".into()),
        };
        let doc = billing_doc(&ctx, &s, &[("Lehm".into(), Some(2.0), Some(1.25))]);
        let t = doc.texts().join("\n");
        for want in [
            "Aufmaß",
            "Januar (31.01.1970) bis Februar (28.02.1970)",
            "530,0 m³",
            "12.740,00 €",
            "1.000,0 t",
            "12,50 €/t",
            "8,00 €/m³",
            "12.500,00 €",
            "600,0 t + 10,0 Ladungen",
            "+15,0 %",
            "Lehm: Dichte 2,00 t/m³, Auflockerung 1,25; LKW-Nutzlast 15,0 t",
            "67",
        ] {
            assert!(t.contains(want), "fehlt: {want}\n{t}");
        }
        assert!(!t.contains("fehlt die Aushub-Berechnung"));
        let mut gap = s.clone();
        gap["missing"] = json!(["Februar"]);
        gap["amount"] = Value::Null;
        gap["truckT"] = Value::Null;
        let t = billing_doc(&ctx, &gap, &[]).texts().join("\n");
        assert!(t.contains("Für Februar fehlt die Aushub-Berechnung"));
        assert!(t.contains("Keine Kennwerte hinterlegt"));
        assert!(!t.contains("fehlen Dichte"));
        gap["unconverted"] = json!(1);
        let t = billing_doc(&ctx, &gap, &[]).texts().join("\n");
        assert!(t.contains("fehlen Dichte oder Auflockerung"));
    }
}
