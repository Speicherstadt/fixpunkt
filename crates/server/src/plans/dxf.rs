//! DXF-Plan → einfache Geometrie für die Ansicht: Linienzüge (Bögen und Kreise in Stücken), Texte und
//! Höhenpunkte. Pläne in Deutschland tragen den Ostwert oft mit Zonenkennung (32 512 000); die Zone wird
//! abgezogen. Blockreferenzen (Symbole) bleiben außen vor.

use anyhow::{bail, Context, Result};
use dxf::entities::{Entity, EntityType};
use dxf::Drawing;
use serde::Serialize;
use std::collections::BTreeSet;

/// Obergrenze für Stützpunkte: mehr zeichnet der Browser nicht flüssig
pub const MAX_VERTICES: usize = 300_000;
/// Winkelschritt für Bögen und Kreise
const ARC_STEP_DEG: f64 = 10.0;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Path {
    pub layer: String,
    pub points: Vec<[f64; 2]>,
    pub closed: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Label {
    pub layer: String,
    pub at: [f64; 2],
    pub text: String,
    pub height: f64,
}

/// Höhe aus dem Plan: Punkt mit z, Stützpunkt einer 3D-Polylinie oder Höhenkote (Text, der eine Zahl ist)
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Spot {
    pub layer: String,
    pub at: [f64; 3],
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Geometry {
    pub paths: Vec<Path>,
    pub labels: Vec<Label>,
    pub spots: Vec<Spot>,
    pub layers: Vec<String>,
    /// Ostwert-Verschiebung durch die Zonenkennung (z. B. 32 000 000); 0, wenn keine
    pub zone_offset: f64,
    /// [minX, minY, maxX, maxY] nach Abzug der Zone
    pub bbox: Option<[f64; 4]>,
}

/// Kreisbogen um `c` von `a0` bis `a1` (Grad, gegen den Uhrzeigersinn)
fn arc(c: [f64; 2], r: f64, a0: f64, a1: f64) -> Vec<[f64; 2]> {
    let mut end = a1;
    while end <= a0 {
        end += 360.0;
    }
    let n = ((end - a0) / ARC_STEP_DEG).ceil().max(1.0) as usize;
    (0..=n)
        .map(|i| {
            let a = (a0 + (end - a0) * i as f64 / n as f64).to_radians();
            [c[0] + r * a.cos(), c[1] + r * a.sin()]
        })
        .collect()
}

/// Zwischenpunkte eines Polylinien-Segments mit Bulge (tan(Öffnungswinkel/4)); ohne Startpunkt
pub fn bulge_segment(p0: [f64; 2], p1: [f64; 2], bulge: f64) -> Vec<[f64; 2]> {
    if bulge.abs() < 1e-9 {
        return vec![p1];
    }
    let (dx, dy) = (p1[0] - p0[0], p1[1] - p0[1]);
    let chord = (dx * dx + dy * dy).sqrt();
    let theta = 4.0 * bulge.atan(); // Öffnungswinkel, Vorzeichen = Richtung
    let r = chord / (2.0 * (theta / 2.0).sin());
    // Mittelpunkt: von der Sehnenmitte senkrecht um r·cos(θ/2)
    let (mx, my) = ((p0[0] + p1[0]) / 2.0, (p0[1] + p1[1]) / 2.0);
    let h = r * (theta / 2.0).cos();
    let (nx, ny) = (-dy / chord, dx / chord);
    let c = [mx + nx * h, my + ny * h];
    let a0 = (p0[1] - c[1]).atan2(p0[0] - c[0]);
    let n = (theta.abs().to_degrees() / ARC_STEP_DEG).ceil().max(1.0) as usize;
    // der letzte Punkt genau auf dem Endpunkt, sonst bleiben winzige Lücken im Linienzug
    (1..n)
        .map(|i| {
            let a = a0 + theta * i as f64 / n as f64;
            [c[0] + r.abs() * a.cos(), c[1] + r.abs() * a.sin()]
        })
        .chain([p1])
        .collect()
}

/// Kote aus einem Text: „298.15“, „298,15“, „+298.15 m“, „OK 298,15“, „H=298.2“ → Zahl. Nur mit
/// Nachkommastellen und höchstens einem kurzen Wort davor – „Schacht 4“ ist keine Höhe.
fn spot_height(text: &str) -> Option<f64> {
    let t = text.trim().trim_end_matches('m').trim_end();
    let prefix_len = t.find(|c: char| c.is_ascii_digit() || c == '+' || c == '-')?;
    let (prefix, number) = t.split_at(prefix_len);
    let word = prefix.trim().trim_end_matches(['=', ':']).trim();
    if word.chars().count() > 3 || !word.chars().all(char::is_alphabetic) {
        return None;
    }
    let (int, frac) = number.trim_start_matches('+').split_once(['.', ','])?;
    let valid =
        |s: &str, n: usize| !s.is_empty() && s.len() <= n && s.chars().all(|c| c.is_ascii_digit());
    if !valid(int.trim_start_matches('-'), 4) || !valid(frac, 3) {
        return None;
    }
    let v: f64 = format!("{int}.{frac}").parse().ok()?;
    (-500.0..5000.0).contains(&v).then_some(v)
}

/// Zonenkennung vor dem Ostwert: 32 512 000 → (32 000 000); normale UTM-Ostwerte liegen unter 1 000 000
fn zone_offset(xs: impl Iterator<Item = f64>) -> f64 {
    let mut min = f64::INFINITY;
    for x in xs {
        min = min.min(x);
    }
    if min.is_finite() && min >= 1.0e7 {
        (min / 1.0e6).floor() * 1.0e6
    } else {
        0.0
    }
}

/// Alles aus den Entities einsammeln, noch ohne Zonenabzug
fn collect(entities: impl Iterator<Item = Entity>) -> (Vec<Path>, Vec<Label>, Vec<Spot>) {
    let (mut paths, mut labels, mut spots) = (Vec::new(), Vec::new(), Vec::new());
    for e in entities {
        let layer = e.common.layer.clone();
        let path = |points: Vec<[f64; 2]>, closed: bool| Path {
            layer: layer.clone(),
            points,
            closed,
        };
        match e.specific {
            EntityType::Line(l) => {
                paths.push(path(vec![[l.p1.x, l.p1.y], [l.p2.x, l.p2.y]], false))
            }
            EntityType::LwPolyline(p) => {
                let v = &p.vertices;
                let mut pts: Vec<[f64; 2]> =
                    v.first().map(|f| vec![[f.x, f.y]]).unwrap_or_default();
                let n = v.len();
                let segs = if p.is_closed() {
                    n
                } else {
                    n.saturating_sub(1)
                };
                for i in 0..segs {
                    let (a, b) = (v[i], v[(i + 1) % n]);
                    pts.extend(bulge_segment([a.x, a.y], [b.x, b.y], a.bulge));
                }
                if p.is_closed() && pts.len() > 1 {
                    pts.pop(); // letzter Punkt = erster
                }
                paths.push(path(pts, p.is_closed()));
            }
            EntityType::Polyline(p) => {
                let vs: Vec<_> = p.vertices().map(|v| v.location.clone()).collect();
                if p.is_3d_polyline() {
                    spots.extend(vs.iter().filter(|v| v.z != 0.0).map(|v| Spot {
                        layer: layer.clone(),
                        at: [v.x, v.y, v.z],
                    }));
                }
                paths.push(path(vs.iter().map(|v| [v.x, v.y]).collect(), p.is_closed()));
            }
            EntityType::Circle(c) => {
                let mut pts = arc([c.center.x, c.center.y], c.radius, 0.0, 360.0);
                pts.pop();
                paths.push(path(pts, true));
            }
            EntityType::Arc(a) => paths.push(path(
                arc(
                    [a.center.x, a.center.y],
                    a.radius,
                    a.start_angle,
                    a.end_angle,
                ),
                false,
            )),
            EntityType::ModelPoint(p) if p.location.z != 0.0 => spots.push(Spot {
                layer,
                at: [p.location.x, p.location.y, p.location.z],
            }),
            EntityType::Text(t) => {
                if let Some(h) = spot_height(&t.value) {
                    spots.push(Spot {
                        layer: layer.clone(),
                        at: [t.location.x, t.location.y, h],
                    });
                }
                labels.push(Label {
                    layer,
                    at: [t.location.x, t.location.y],
                    text: t.value,
                    height: t.text_height,
                });
            }
            EntityType::MText(t) => labels.push(Label {
                layer,
                at: [t.insertion_point.x, t.insertion_point.y],
                // Formatierung wie \\P (Absatz) grob entfernen
                text: t.text.replace("\\P", " "),
                height: t.initial_text_height,
            }),
            _ => {}
        }
    }
    paths.retain(|p| p.points.len() >= 2);
    (paths, labels, spots)
}

pub fn from_drawing(d: &Drawing) -> Result<Geometry> {
    let (mut paths, mut labels, mut spots) = collect(d.entities().cloned());
    let vertices: usize = paths.iter().map(|p| p.points.len()).sum::<usize>() + spots.len();
    if vertices > MAX_VERTICES {
        bail!("Plan zu groß: {vertices} Punkte, höchstens {MAX_VERTICES}");
    }
    let xs = paths
        .iter()
        .flat_map(|p| p.points.iter().map(|q| q[0]))
        .chain(labels.iter().map(|l| l.at[0]))
        .chain(spots.iter().map(|s| s.at[0]));
    let off = zone_offset(xs);
    let mut bb = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut grow = |x: f64, y: f64| {
        bb = [bb[0].min(x), bb[1].min(y), bb[2].max(x), bb[3].max(y)];
    };
    for p in &mut paths {
        for q in &mut p.points {
            q[0] -= off;
            grow(q[0], q[1]);
        }
    }
    for l in &mut labels {
        l.at[0] -= off;
        grow(l.at[0], l.at[1]);
    }
    for s in &mut spots {
        s.at[0] -= off;
        grow(s.at[0], s.at[1]);
    }
    let layers: BTreeSet<String> = paths
        .iter()
        .map(|p| p.layer.clone())
        .chain(labels.iter().map(|l| l.layer.clone()))
        .chain(spots.iter().map(|s| s.layer.clone()))
        .collect();
    Ok(Geometry {
        paths,
        labels,
        spots,
        layers: layers.into_iter().collect(),
        zone_offset: off,
        bbox: bb[0].is_finite().then_some(bb),
    })
}

/// DXF-Datei (ASCII oder binär) lesen
pub fn read(bytes: &[u8]) -> Result<Geometry> {
    let d = Drawing::load(&mut std::io::Cursor::new(bytes)).context("keine lesbare DXF-Datei")?;
    let g = from_drawing(&d)?;
    if g.bbox.is_none() {
        bail!("der Plan enthält nichts Darstellbares");
    }
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Von Hand geschriebene DXF (R12, wie aus vielen CAD-Programmen): Linie, geschlossene Polylinie mit
    /// einem Halbkreis (Bulge 1), Kreis, Höhenpunkt, Höhenkote als Text; Ostwerte mit Zonenkennung 32
    const R12: &str = "0\nSECTION\n2\nENTITIES\n\
0\nLINE\n8\nBaugrube\n10\n32512000.0\n20\n5403000.0\n30\n0.0\n11\n32512010.0\n21\n5403000.0\n31\n0.0\n\
0\nLWPOLYLINE\n8\nGrenze\n90\n4\n70\n1\n10\n32512000.0\n20\n5403010.0\n10\n32512020.0\n20\n5403010.0\n42\n1.0\n10\n32512020.0\n20\n5403020.0\n10\n32512000.0\n20\n5403020.0\n\
0\nCIRCLE\n8\nSchacht\n10\n32512005.0\n20\n5403005.0\n30\n0.0\n40\n1.0\n\
0\nPOINT\n8\nHoehen\n10\n32512002.0\n20\n5403002.0\n30\n298.15\n\
0\nTEXT\n8\nHoehen\n10\n32512004.0\n20\n5403004.0\n30\n0.0\n40\n0.5\n1\n298,40\n\
0\nENDSEC\n0\nEOF\n";

    #[test]
    fn hand_written_r12_plan() {
        let g = read(R12.as_bytes()).unwrap();
        assert_eq!(g.zone_offset, 32_000_000.0);
        assert_eq!(g.layers, ["Baugrube", "Grenze", "Hoehen", "Schacht"]);
        let line = &g.paths[0];
        assert_eq!(
            line.points,
            vec![[512_000.0, 5_403_000.0], [512_010.0, 5_403_000.0]]
        );
        // Polylinie: Halbkreis mit Durchmesser 10 m zwischen (20|10) und (20|20), nach Osten gewölbt
        let poly = &g.paths[1];
        assert!(poly.closed);
        let max_x = poly.points.iter().map(|p| p[0]).fold(f64::MIN, f64::max);
        assert!(
            (max_x - 512_025.0).abs() < 1e-6,
            "Scheitel 5 m östlich: {max_x}"
        );
        for p in &poly.points[2..poly.points.len() - 2] {
            let r = ((p[0] - 512_020.0).powi(2) + (p[1] - 5_403_015.0).powi(2)).sqrt();
            assert!((r - 5.0).abs() < 1e-6, "auf dem Kreis: {r}");
        }
        // Kreis: 36 Stücke, alle mit Radius 1
        let circle = &g.paths[2];
        assert_eq!(circle.points.len(), 36);
        assert!(circle.closed);
        // Höhen: Punkt mit z und Kote aus Text
        assert_eq!(
            g.spots,
            vec![
                Spot {
                    layer: "Hoehen".into(),
                    at: [512_002.0, 5_403_002.0, 298.15]
                },
                Spot {
                    layer: "Hoehen".into(),
                    at: [512_004.0, 5_403_004.0, 298.40]
                },
            ]
        );
        assert_eq!(g.labels[0].text, "298,40");
        assert_eq!(
            g.bbox,
            Some([512_000.0, 5_403_000.0, 512_025.0, 5_403_020.0])
        );
    }

    #[test]
    fn arcs_bulges_and_kotes() {
        // Viertelkreis gegen den Uhrzeigersinn: Bulge tan(90°/4)
        let pts = bulge_segment([1.0, 0.0], [0.0, 1.0], (22.5f64).to_radians().tan());
        assert!(pts
            .iter()
            .all(|p| ((p[0].powi(2) + p[1].powi(2)).sqrt() - 1.0).abs() < 1e-9));
        assert_eq!(*pts.last().unwrap(), [0.0, 1.0]);
        assert_eq!(bulge_segment([0.0, 0.0], [1.0, 0.0], 0.0), vec![[1.0, 0.0]]);
        let a = arc([0.0, 0.0], 2.0, 350.0, 10.0);
        assert_eq!(a.len(), 3, "über 0° hinweg: 350° → 10°");
        assert_eq!(spot_height("OK 298,15"), Some(298.15));
        assert_eq!(spot_height("+300.5"), Some(300.5));
        assert_eq!(spot_height("Schacht 4"), None);
        assert_eq!(spot_height("Schacht 4,5"), None);
        assert_eq!(spot_height("H=298.2"), Some(298.2));
        assert_eq!(spot_height("GOK 300,10 m"), Some(300.1));
        assert_eq!(spot_height("DN 300"), None);
        assert_eq!(spot_height("2026"), None);
        assert_eq!(zone_offset([512_000.0, 600_000.0].into_iter()), 0.0);
        assert_eq!(zone_offset([33_400_000.0].into_iter()), 33_000_000.0);
    }

    #[test]
    fn bad_files() {
        assert!(read(b"kein DXF").is_err());
        assert!(read(b"0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF\n").is_err());
    }
}
