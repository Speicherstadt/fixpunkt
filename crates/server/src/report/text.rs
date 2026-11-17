//! Text für den PDF-Bericht: Kodierung in WinAnsi, Breite in Helvetica, Zahlen deutsch formatiert.

use super::metrics;

/// Schriftschnitt der Standardschrift Helvetica
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    Regular,
    Bold,
}

/// Zeichen nach WinAnsi (cp1252); was dort fehlt, wird ersetzt (− als –, sonst ?)
pub fn encode(s: &str) -> Vec<u8> {
    s.chars().map(win_ansi).collect()
}

fn win_ansi(c: char) -> u8 {
    match c {
        ' '..='~' => c as u8,
        '\u{a0}'..='\u{ff}' => c as u32 as u8,
        '€' => 0x80,
        '‚' => 0x82,
        '„' => 0x84,
        '…' => 0x85,
        '‰' => 0x89,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' | '−' => 0x96,
        // Pfeil fehlt in WinAnsi; » liest sich in „Urgelände » Aushub“ genauso
        '→' => 0xbb,
        '—' => 0x97,
        '\u{202f}' | '\u{2009}' => b' ',
        _ => b'?',
    }
}

/// Breite in pt bei Schriftgröße `size`
pub fn width(s: &str, weight: Weight, size: f32) -> f32 {
    let table = match weight {
        Weight::Regular => &metrics::REGULAR,
        Weight::Bold => &metrics::BOLD,
    };
    let units: u32 = encode(s)
        .iter()
        .map(|&b| u32::from(table[usize::from(b.max(32)) - 32]))
        .sum();
    units as f32 * size / 1000.0
}

/// Kürzt mit „…“, bis der Text in `max` pt passt
pub fn fit(s: &str, weight: Weight, size: f32, max: f32) -> String {
    if width(s, weight, size) <= max {
        return s.to_string();
    }
    let mut chars: Vec<char> = s.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let t: String = chars.iter().collect::<String>() + "…";
        if width(&t, weight, size) <= max {
            return t;
        }
    }
    "…".to_string()
}

/// Bricht an Leerzeichen um, sodass jede Zeile in `max` pt passt
pub fn wrap(s: &str, weight: Weight, size: f32, max: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        let next = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if width(&next, weight, size) > max && !line.is_empty() {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        } else {
            line = next;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Zahl mit `digits` Nachkommastellen, Tausenderpunkt und Dezimalkomma; „−0,0“ wird „0,0“
pub fn number(v: f64, digits: usize) -> String {
    if !v.is_finite() {
        return "–".into();
    }
    // kaufmännisch runden wie Intl.NumberFormat in der UI (81,25 → 81,3), nicht halb-gerade
    let scale = 10f64.powi(digits as i32);
    let s = format!("{:.*}", digits, (v.abs() * scale).round() / scale);
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let mut grouped = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(c);
    }
    let zero = s.chars().all(|c| c == '0' || c == '.');
    let sign = if v < 0.0 && !zero { "−" } else { "" };
    if frac.is_empty() {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped},{frac}")
    }
}

pub fn volume(m3: f64) -> String {
    format!("{} m³", number(m3, 1))
}

pub fn area(m2: f64) -> String {
    format!("{} m²", number(m2, 1))
}

pub fn height(m: f64) -> String {
    format!("{} m", number(m, 2))
}

pub fn cm(m: f64) -> String {
    format!("{} cm", number(m * 100.0, 1))
}

pub fn percent(p: f64) -> String {
    format!("{} %", number(p, 1))
}

/// Datum (UTC) aus Millisekunden seit 1970, „17.11.2026“
pub fn date(ms: i64) -> String {
    let t = time::OffsetDateTime::from_unix_timestamp(ms / 1000)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    format!("{:02}.{:02}.{}", t.day(), u8::from(t.month()), t.year())
}

/// Datum und Uhrzeit (UTC) aus Millisekunden seit 1970, „17.11.2026, 09:00 UTC“
pub fn date_time(ms: i64) -> String {
    let t = time::OffsetDateTime::from_unix_timestamp(ms / 1000)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    format!(
        "{:02}.{:02}.{}, {:02}:{:02} UTC",
        t.day(),
        u8::from(t.month()),
        t.year(),
        t.hour(),
        t.minute()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn german_numbers() {
        assert_eq!(number(1234567.891, 1), "1.234.567,9");
        assert_eq!(number(800.05, 1), "800,1");
        assert_eq!(number(-12.0, 1), "−12,0");
        assert_eq!(number(-0.04, 1), "0,0");
        assert_eq!(number(999.96, 1), "1.000,0");
        assert_eq!(number(42.0, 0), "42");
        assert_eq!(number(81.25, 1), "81,3");
        assert_eq!(number(0.125, 2), "0,13");
        assert_eq!(volume(800.1), "800,1 m³");
        assert_eq!(cm(-0.031), "−3,1 cm");
        assert_eq!(number(f64::NAN, 1), "–");
    }

    #[test]
    fn win_ansi_covers_german_and_units() {
        assert_eq!(
            encode("Aä ß³±°"),
            vec![b'A', 0xe4, b' ', 0xdf, 0xb3, 0xb1, 0xb0]
        );
        assert_eq!(encode("−–„“€"), vec![0x96, 0x96, 0x84, 0x93, 0x80]);
        assert_eq!(encode("✓"), vec![b'?']);
        assert_eq!(encode("a → b"), vec![b'a', b' ', 0xbb, b' ', b'b']);
    }

    #[test]
    fn widths_match_helvetica_afm() {
        // Helvetica-AFM: H 722, e 556, l 222, o 556 → „Hello“ = 2278 Einheiten
        assert!((width("Hello", Weight::Regular, 10.0) - 22.78).abs() < 1e-4);
        // Ziffern sind gleich breit (556), daher stehen rechtsbündige Zahlen sauber untereinander
        assert_eq!(
            width("0123456789", Weight::Bold, 1.0),
            width("8888888888", Weight::Bold, 1.0)
        );
        assert!((width("m³", Weight::Regular, 1000.0) - (833.0 + 333.0)).abs() < 1e-3);
    }

    #[test]
    fn fit_and_wrap() {
        let long = "Abrechnungsbereich Nordböschung mit Zufahrt";
        let short = fit(long, Weight::Regular, 10.0, 80.0);
        assert!(short.ends_with('…'));
        assert!(width(&short, Weight::Regular, 10.0) <= 80.0);
        assert_eq!(fit("Grube", Weight::Regular, 10.0, 80.0), "Grube");
        let lines = wrap(long, Weight::Regular, 10.0, 100.0);
        assert!(lines.len() > 1);
        assert!(lines
            .iter()
            .all(|l| width(l, Weight::Regular, 10.0) <= 100.0));
        assert_eq!(lines.join(" "), long);
    }

    #[test]
    fn dates_in_utc() {
        // 2026-11-17T09:00:00Z
        assert_eq!(date_time(1_794_906_000_000), "17.11.2026, 09:00 UTC");
    }
}
