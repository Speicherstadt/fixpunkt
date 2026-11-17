//! Karte für den Bericht: Differenz oder Soll/Ist-Abweichung als Farbbild mit den Abrechnungsbereichen.
//! Farben wie in der 2D-Ansicht (`ui/src/lib/view/colors.ts`): Orange heißt Material weg, Blau Material dazu.

use super::doc::Map;
use super::text;
use fixpunkt::geom::Region;
use fixpunkt::grid::Grid;

/// längste Bildseite in Pixeln; mehr zeigt A4 bei 500 pt Breite nicht
pub const MAX_PIXELS: usize = 600;

/// Blau – hell – Orange, wie `DIVERGING` in der UI
const DIVERGING: [[f64; 3]; 5] = [
    [29.0, 78.0, 216.0],
    [147.0, 197.0, 253.0],
    [245.0, 245.0, 245.0],
    [253.0, 186.0, 116.0],
    [180.0, 83.0, 9.0],
];
const NODATA: [u8; 3] = [255, 255, 255];

/// Was das Raster zeigt: bei einer Differenz (nachher − vorher) heißt negativ „abgetragen“,
/// bei Soll/Ist (Ist − Soll) heißt positiv „zu hoch, muss weg“
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Difference,
    Deviation,
}

fn ramp(t: f64) -> [u8; 3] {
    let u = t.clamp(0.0, 1.0) * (DIVERGING.len() - 1) as f64;
    let i = (u.floor() as usize).min(DIVERGING.len() - 2);
    let f = u - i as f64;
    let (a, b) = (DIVERGING[i], DIVERGING[i + 1]);
    [0, 1, 2].map(|k| (a[k] + (b[k] - a[k]) * f).round() as u8)
}

/// Farbe eines Werts; `limit` ist der Betrag, ab dem die Farbe satt ist
pub fn color(v: f64, limit: f64, kind: Kind) -> [u8; 3] {
    let removed = match kind {
        Kind::Difference => -v,
        Kind::Deviation => v,
    };
    ramp(0.5 + removed / (2.0 * limit))
}

/// Grenze der Farbskala: 98 % der Beträge darunter, mindestens 5 cm, aufgerundet auf eine runde Zahl
pub fn limit(values: &[f32]) -> f64 {
    let mut a: Vec<f64> = values
        .iter()
        .filter(|v| !v.is_nan())
        .map(|v| f64::from(v.abs()))
        .collect();
    if a.is_empty() {
        return 0.05;
    }
    a.sort_by(f64::total_cmp);
    let q = a[((a.len() - 1) as f64 * 0.98).floor() as usize].max(0.05);
    let pow = 10f64.powf(q.log10().floor());
    [1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 7.5, 10.0]
        .into_iter()
        .map(|s| s * pow)
        .find(|s| *s >= q - 1e-9)
        .unwrap_or(10.0 * pow)
}

/// Betrag der Skala, „50 cm“ oder „2 m“
pub fn magnitude(lim: f64) -> String {
    let (v, unit) = if lim >= 1.0 {
        (lim, "m")
    } else {
        (lim * 100.0, "cm")
    };
    let digits = if (v - v.round()).abs() < 1e-6 { 0 } else { 1 };
    format!("{} {unit}", text::number(v, digits))
}

/// Mittel der Zellen mit Daten je Block von `k × k` Zellen
fn downsample(g: &Grid, k: usize) -> (usize, usize, Vec<f32>) {
    let s = g.spec;
    let (cols, rows) = (s.cols.div_ceil(k), s.rows.div_ceil(k));
    let mut out = vec![f32::NAN; cols * rows];
    for r in 0..rows {
        for c in 0..cols {
            let (mut sum, mut n) = (0.0f64, 0u32);
            for rr in r * k..((r + 1) * k).min(s.rows) {
                for cc in c * k..((c + 1) * k).min(s.cols) {
                    let v = g.data[rr * s.cols + cc];
                    if !v.is_nan() {
                        sum += f64::from(v);
                        n += 1;
                    }
                }
            }
            if n > 0 {
                out[r * cols + c] = (sum / f64::from(n)) as f32;
            }
        }
    }
    (cols, rows, out)
}

/// Karte aus dem Raster; die Bereiche werden in Bildkoordinaten umgerechnet
pub fn build(g: &Grid, regions: &[Region], kind: Kind, caption: String) -> Map {
    let s = g.spec;
    let k = s.cols.max(s.rows).div_ceil(MAX_PIXELS).max(1);
    let (cols, rows, values) = downsample(g, k);
    let lim = limit(&values);
    let rgb = values
        .iter()
        .flat_map(|v| {
            if v.is_nan() {
                NODATA
            } else {
                color(f64::from(*v), lim, kind)
            }
        })
        .collect();
    let (w, h) = (s.cols as f64 * s.cell, s.rows as f64 * s.cell);
    let outlines = regions
        .iter()
        .map(|r| {
            let ring = r
                .polygon
                .exterior
                .iter()
                .map(|[x, y]| [(x - s.min_x) / w, (s.max_y - y) / h])
                .collect();
            (r.name.clone(), ring)
        })
        .collect();
    let (removed, added) = match kind {
        Kind::Difference => ("Abtrag", "Auftrag"),
        Kind::Deviation => ("zu hoch", "zu tief"),
    };
    Map {
        cols,
        rows,
        rgb,
        width_m: w,
        outlines,
        legend: (0..=32).map(|i| ramp(f64::from(i) / 32.0)).collect(),
        legend_labels: [
            format!("{added} {}", magnitude(lim)),
            "0".into(),
            format!("{removed} {}", magnitude(lim)),
        ],
        caption,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixpunkt::geom::Polygon;
    use fixpunkt::grid::GridSpec;

    fn grid(cols: usize, rows: usize, f: impl Fn(usize, usize) -> f32) -> Grid {
        let spec = GridSpec {
            min_x: 500_000.0,
            max_y: 5_400_000.0,
            cell: 0.5,
            cols,
            rows,
        };
        let mut data = Vec::with_capacity(cols * rows);
        for r in 0..rows {
            for c in 0..cols {
                data.push(f(c, r));
            }
        }
        Grid { spec, data }
    }

    #[test]
    fn colors_follow_the_ui() {
        // abgetragen (Differenz negativ) und zu hoch (Abweichung positiv) sind beide orange
        assert_eq!(color(-1.0, 1.0, Kind::Difference), [180, 83, 9]);
        assert_eq!(color(1.0, 1.0, Kind::Deviation), [180, 83, 9]);
        assert_eq!(color(1.0, 1.0, Kind::Difference), [29, 78, 216]);
        assert_eq!(color(0.0, 1.0, Kind::Deviation), [245, 245, 245]);
        // über die Grenze hinaus bleibt die Farbe satt
        assert_eq!(color(-5.0, 1.0, Kind::Difference), [180, 83, 9]);
    }

    #[test]
    fn limit_is_round_and_robust() {
        let mut v = vec![0.25f32; 99];
        v.push(40.0); // ein Ausreißer verschiebt die Skala nicht
        assert_eq!(limit(&v), 0.25);
        assert_eq!(limit(&[2.35; 3]), 2.5);
        assert_eq!(limit(&[0.01, f32::NAN]), 0.05);
        assert_eq!(limit(&[-1.7; 10]), 2.0);
        assert_eq!(limit(&[]), 0.05);
    }

    #[test]
    fn large_grids_are_averaged_down() {
        // 1500 × 900 Zellen → Blöcke von 3 × 3 → 500 × 300 Pixel
        let g = grid(1500, 900, |c, _| if c < 750 { -2.0 } else { f32::NAN });
        let m = build(&g, &[], Kind::Difference, String::new());
        assert_eq!((m.cols, m.rows), (500, 300));
        assert_eq!(m.rgb.len(), 500 * 300 * 3);
        assert_eq!(&m.rgb[0..3], &[180, 83, 9]);
        let last = m.rgb.len() - 3;
        assert_eq!(&m.rgb[last..], &NODATA);
        assert!((m.width_m - 750.0).abs() < 1e-9);
        assert_eq!(
            m.legend_labels,
            ["Auftrag 2 m", "0", "Abtrag 2 m"].map(String::from)
        );
        assert_eq!(magnitude(0.05), "5 cm");
        assert_eq!(magnitude(0.075), "7,5 cm");
        assert_eq!(magnitude(2.5), "2,5 m");
    }

    #[test]
    fn outlines_in_image_coordinates() {
        let g = grid(40, 20, |_, _| 0.1);
        let r = Region {
            name: "Grube".into(),
            polygon: Polygon::rect(500_005.0, 5_399_995.0, 500_015.0, 5_400_000.0),
        };
        let m = build(&g, &[r], Kind::Deviation, String::new());
        let ring = &m.outlines[0].1;
        // 40 × 0,5 = 20 m breit, 10 m hoch; Ecke links unten bei x = 5 m, y = 5 m unter der Oberkante
        assert!(ring.contains(&[0.25, 0.5]));
        assert!(ring.contains(&[0.75, 0.0]));
    }
}
