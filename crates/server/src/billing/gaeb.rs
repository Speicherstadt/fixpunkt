//! Aufmaß als GAEB DA XML 3.3, Datenaustauschphase X31 (Mengenermittlung), Schema-Stand 2023-01.
//!
//! Aufbau nach `GAEB_DA_XML_31_3.3_2023-01.xsd` und der Fachdokumentation GAEB DA XML 3.3 (2023-01),
//! Kapitel 7 (beide von www.gaeb.de): `GAEB` → `GAEBInfo`, `QtyDeterm` → `PrjInfo`, `QtyDetermInfo`, `DP` 31,
//! `BoQ` mit `BoQBkdn` (Gliederung der Ordnungszahl) und `BoQBody` aus `BoQCtgy` und `Itemlist`/`Item`.
//!
//! Je Position steht nur die Gesamtmenge (`Item/QtyDeterm/Qty`); Aufmaßzeilen nach REB-VB 23.003 entfallen,
//! weil die Menge aus dem Vergleich zweier Geländemodelle stammt, nicht aus Rechenansätzen. Das ist nach
//! Kapitel 7.3 der Fachdokumentation zulässig („Die Gesamtsumme einer Position kann auch ohne dazugehörende
//! Aufmaßzeilen übertragen werden.“). Positionen mit derselben Ordnungszahl werden addiert.

use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write;

pub const NAMESPACE: &str = "http://www.gaeb.de/GAEB_DA_XML/DA31/3.3";

/// Ordnungszahl in Teile: „02.01.0010“ → ["02", "01", "0010"]
fn parts(oz: &str) -> Vec<&str> {
    oz.split(['.', ' ']).filter(|p| !p.is_empty()).collect()
}

fn valid_part(p: &str) -> bool {
    !p.is_empty() && p.len() <= 14 && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Gliederung aus der ersten Ordnungszahl; alle müssen ihr folgen (gleich viele Teile, gleiche Längen)
fn breakdown(ozs: &[&str]) -> Result<Vec<usize>, String> {
    let first = ozs.first().ok_or("keine Position mit LV-Position")?;
    let mask: Vec<usize> = parts(first).iter().map(|p| p.len()).collect();
    if mask.is_empty() || mask.len() > 6 || mask.iter().sum::<usize>() > 14 {
        return Err(format!(
            "Ordnungszahl „{first}“: höchstens 6 Stufen und 14 Stellen"
        ));
    }
    for oz in ozs {
        let p = parts(oz);
        if p.len() != mask.len()
            || p.iter().zip(&mask).any(|(x, l)| x.len() != *l)
            || !p.iter().all(|x| valid_part(x))
        {
            return Err(format!(
                "Ordnungszahl „{oz}“ passt nicht zur Gliederung von „{first}“"
            ));
        }
    }
    Ok(mask)
}

/// Menge einer Position in ihrer Einheit; ohne Preiseinheit m³ fest
fn qty(p: &Value) -> Option<f64> {
    match p["unit"].as_str() {
        Some(_) => p["quantity"].as_f64(),
        None => p["period"]["bankM3"].as_f64(),
    }
}

fn esc(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            c => c.to_string(),
        })
        .collect()
}

/// höchstens `n` Zeichen (normalizedString-Felder des Schemas haben feste Längen)
fn cut(s: &str, n: usize) -> String {
    esc(&s
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(n)
        .collect::<String>())
}

/// Gruppen und Positionen als Baum: Teil-OZ → Untergruppe oder Menge
#[derive(Default)]
struct Node {
    children: BTreeMap<String, Node>,
    qty: f64,
}

fn body(out: &mut String, node: &Node, path: &[String], depth: usize, levels: usize) {
    out.push_str("<BoQBody>");
    if depth + 1 == levels {
        out.push_str("<Itemlist>");
        for (rno, item) in &node.children {
            let id = format!("I_{}_{rno}", path.join("_"));
            let _ = write!(
                out,
                r#"<Item ID="{}" RNoPart="{rno}"><QtyDeterm><Qty>{:.3}</Qty></QtyDeterm></Item>"#,
                id.replace("I__", "I_"),
                item.qty
            );
        }
        out.push_str("</Itemlist>");
    } else {
        for (rno, child) in &node.children {
            let mut p = path.to_vec();
            p.push(rno.clone());
            let _ = write!(out, r#"<BoQCtgy ID="C_{}" RNoPart="{rno}">"#, p.join("_"));
            body(out, child, &p, depth + 1, levels);
            out.push_str("</BoQCtgy>");
        }
    }
    out.push_str("</BoQBody>");
}

/// Kopfdaten der Datei
pub struct Head<'a> {
    pub site: &'a str,
    /// `2026-11-17`
    pub date: String,
    /// `21:05:00`
    pub time: String,
    pub from: String,
    pub to: String,
}

/// X31-Datei aus einem Aufmaß (`statement::statement`). Fehler als Text: keine Positionen, Gliederung passt
/// nicht, Menge fehlt (z. B. Preis je t ohne Dichte).
pub fn x31(s: &Value, h: &Head) -> Result<String, String> {
    let positions: Vec<&Value> = s["positions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["lvPosition"].is_string())
        .collect();
    let ozs: Vec<&str> = positions
        .iter()
        .filter_map(|p| p["lvPosition"].as_str())
        .collect();
    let mask = breakdown(&ozs)?;
    let mut root = Node::default();
    for p in &positions {
        let oz = p["lvPosition"].as_str().unwrap_or_default();
        let q = qty(p).ok_or_else(|| {
            format!(
                "Position {oz} ({}): keine Menge in der Preiseinheit – Kennwerte oder Berechnung fehlen",
                p["name"].as_str().unwrap_or("")
            )
        })?;
        let mut node = &mut root;
        for part in parts(oz) {
            node = node.children.entry(part.to_string()).or_default();
        }
        node.qty += q;
    }
    if positions
        .iter()
        .any(|p| qty(p).is_some_and(|q| q.abs() >= 1e8))
    {
        return Err("Menge über 99.999.999,999".into());
    }

    let mut out = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    let _ = write!(
        out,
        r#"<GAEB xmlns="{NAMESPACE}"><GAEBInfo><Version>3.3</Version><VersDate>2023-01</VersDate><Date>{}</Date><Time>{}</Time><ProgSystem>Fixpunkt</ProgSystem><ProgName>Fixpunkt {}</ProgName></GAEBInfo>"#,
        h.date,
        h.time,
        env!("CARGO_PKG_VERSION")
    );
    let _ = write!(
        out,
        r#"<QtyDeterm><PrjInfo><RefPrjName>{}</RefPrjName></PrjInfo><QtyDetermInfo><MethodDescription>REB23003-2009</MethodDescription><ProjDescr>{}</ProjDescr><ServiceProvisionStartDate>{}</ServiceProvisionStartDate><ServiceProvisionEndDate>{}</ServiceProvisionEndDate></QtyDetermInfo><DP>31</DP><BoQ ID="BoQ1">"#,
        cut(h.site, 60),
        cut(h.site, 80),
        h.from,
        h.to
    );
    for (i, len) in mask.iter().enumerate() {
        let kind = if i + 1 == mask.len() {
            "Item"
        } else {
            "BoQLevel"
        };
        let numeric = ozs
            .iter()
            .all(|oz| parts(oz)[i].chars().all(|c| c.is_ascii_digit()));
        let _ = write!(
            out,
            "<BoQBkdn><Type>{kind}</Type><Length>{len}</Length><Num>{}</Num></BoQBkdn>",
            if numeric { "Yes" } else { "No" }
        );
    }
    body(&mut out, &root, &[], 0, mask.len());
    out.push_str("</BoQ></QtyDeterm></GAEB>\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn head() -> Head<'static> {
        Head {
            site: "Baugrube Kirchstraße 12 & Co",
            date: "2026-11-17".into(),
            time: "21:05:00".into(),
            from: "2026-11-10".into(),
            to: "2026-11-17".into(),
        }
    }

    fn pos(oz: Option<&str>, unit: Option<&str>, quantity: Option<f64>, bank: f64) -> Value {
        json!({"name": "Grube", "lvPosition": oz, "unit": unit, "quantity": quantity,
               "period": {"bankM3": bank}})
    }

    #[test]
    fn items_in_categories_with_summed_quantities() {
        let s = json!({"positions": [
            pos(Some("02.01.0010"), Some("t"), Some(800.1234), 400.0),
            pos(Some("02.01.0020"), None, None, 123.4),
            // gleiche OZ (z. B. zwei Bereiche): addiert
            pos(Some("02.01.0010"), Some("t"), Some(100.0), 50.0),
            pos(Some("03.01.0010"), Some("m3"), Some(5.0), 5.0),
            // ohne LV-Position: nicht im LV
            pos(None, None, None, 9.0),
        ]});
        let x = x31(&s, &head()).unwrap();
        assert!(x.contains(r#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA31/3.3">"#));
        assert!(x.contains("<DP>31</DP>"));
        assert!(x.contains("<RefPrjName>Baugrube Kirchstraße 12 &amp; Co</RefPrjName>"));
        // Gliederung 2 + 2 + 4
        assert!(x.contains("<BoQBkdn><Type>BoQLevel</Type><Length>2</Length><Num>Yes</Num></BoQBkdn><BoQBkdn><Type>BoQLevel</Type><Length>2</Length><Num>Yes</Num></BoQBkdn><BoQBkdn><Type>Item</Type><Length>4</Length><Num>Yes</Num></BoQBkdn>"));
        assert!(x.contains(r#"<BoQCtgy ID="C_02" RNoPart="02"><BoQBody><BoQCtgy ID="C_02_01" RNoPart="01"><BoQBody><Itemlist><Item ID="I_02_01_0010" RNoPart="0010"><QtyDeterm><Qty>900.123</Qty></QtyDeterm></Item><Item ID="I_02_01_0020" RNoPart="0020"><QtyDeterm><Qty>123.400</Qty></QtyDeterm></Item></Itemlist>"#));
        assert!(x.contains(r#"<Item ID="I_03_01_0010" RNoPart="0010"><QtyDeterm><Qty>5.000</Qty>"#));
        assert!(!x.contains("9.000"));
    }

    #[test]
    fn single_level_and_errors() {
        let flat = json!({"positions": [pos(Some("10"), None, None, 1.5)]});
        let x = x31(&flat, &head()).unwrap();
        assert!(x.contains(
            r#"<BoQBody><Itemlist><Item ID="I_10" RNoPart="10"><QtyDeterm><Qty>1.500</Qty>"#
        ));
        let none = json!({"positions": [pos(None, None, None, 1.0)]});
        assert!(x31(&none, &head()).unwrap_err().contains("keine Position"));
        let mixed = json!({"positions": [pos(Some("02.01.0010"), None, None, 1.0), pos(Some("02.0010"), None, None, 1.0)]});
        assert!(x31(&mixed, &head())
            .unwrap_err()
            .contains("passt nicht zur Gliederung"));
        let unconverted = json!({"positions": [pos(Some("01.0010"), Some("t"), None, 1.0)]});
        assert!(x31(&unconverted, &head())
            .unwrap_err()
            .contains("keine Menge"));
        let long = json!({"positions": [pos(Some("123456789.123456"), None, None, 1.0)]});
        assert!(x31(&long, &head()).is_err());
    }

    /// Prüft eine erzeugte Datei gegen das offizielle Schema, wenn es vorliegt:
    /// `GAEB_X31_XSD=/pfad/GAEB_DA_XML_31_3.3_2023-01.xsd cargo test -p fixpunkt-server gaeb` (braucht xmllint).
    /// Das Schema steht nicht im Repo (© DIN); Download: www.gaeb.de → Downloads → GAEB-Datenaustausch.
    #[test]
    fn valid_against_the_official_schema() {
        let Ok(xsd) = std::env::var("GAEB_X31_XSD") else {
            return;
        };
        let s = json!({"positions": [
            pos(Some("02.01.0010"), Some("t"), Some(800.0), 400.0),
            pos(Some("02.02.0010"), None, None, 12.0),
        ]});
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("aufmass.x31");
        std::fs::write(&file, x31(&s, &head()).unwrap()).unwrap();
        let flat = dir.path().join("flach.x31");
        std::fs::write(
            &flat,
            x31(
                &json!({"positions": [pos(Some("10"), None, None, 1.0)]}),
                &head(),
            )
            .unwrap(),
        )
        .unwrap();
        let out = std::process::Command::new("xmllint")
            .args(["--noout", "--schema", &xsd])
            .arg(&file)
            .arg(&flat)
            .output()
            .expect("xmllint");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
