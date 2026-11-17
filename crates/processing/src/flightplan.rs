//! Flugplanung für eine Kartierung: parallele Bahnen über einem Bereich (Rasenmäher-Muster) aus Bodenauflösung
//! (GSD) und Überlappung. Reine Geometrie in UTM; das Schreiben als KMZ für DJI Pilot 2 steht in [`crate::wpml`].
//!
//! Flughöhe H = GSD · f · Bildbreite / Sensorbreite. Die lange Bildseite liegt quer zur Flugrichtung:
//! Bahnabstand = GSD · Bildbreite · (1 − Querüberlappung), Fotoabstand = GSD · Bildhöhe · (1 − Längsüberlappung).

use crate::geom::Polygon;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// Kamera einer Drohne: Sensor in mm, echte (nicht äquivalente) Brennweite, Bild in Pixeln
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Camera {
    pub name: &'static str,
    pub sensor_w_mm: f64,
    pub sensor_h_mm: f64,
    pub focal_mm: f64,
    pub image_w_px: u32,
    pub image_h_px: u32,
    /// kürzester Abstand zweier Fotos in s
    pub min_interval_s: f64,
    /// höchste Geschwindigkeit in Wegpunkt-Missionen in m/s
    pub max_speed: f64,
}

/// Weitwinkelkamera der DJI Matrice 4E: 4/3-CMOS 17,3 × 13 mm, 20 MP (5280 × 3956), 24 mm KB-äquivalent
/// (12,29 mm echt) – GSD ≈ H / 37,5 (cm bei H in m)
pub const M4E: Camera = Camera {
    name: "DJI Matrice 4E",
    sensor_w_mm: 17.3,
    sensor_h_mm: 13.0,
    focal_mm: 12.29,
    image_w_px: 5280,
    image_h_px: 3956,
    min_interval_s: 0.7,
    max_speed: 15.0,
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FlightParams {
    /// Bodenauflösung in m je Pixel
    pub gsd: f64,
    /// Längsüberlappung 0…1 (in Flugrichtung)
    pub front_overlap: f64,
    /// Querüberlappung 0…1 (zwischen den Bahnen)
    pub side_overlap: f64,
    /// Flugrichtung in Grad, im Uhrzeigersinn von Nord (0 = Nord–Süd, 90 = Ost–West)
    pub direction: f64,
    /// gewünschte Geschwindigkeit in m/s; wird durch den Fotoabstand begrenzt
    pub speed: f64,
}

impl Default for FlightParams {
    fn default() -> Self {
        FlightParams {
            gsd: 0.02,
            front_overlap: 0.8,
            side_overlap: 0.7,
            direction: 0.0,
            speed: 10.0,
        }
    }
}

impl FlightParams {
    pub fn validate(&self) -> Result<()> {
        if !(0.005..=0.2).contains(&self.gsd) {
            bail!("Bodenauflösung muss zwischen 0,5 und 20 cm liegen");
        }
        for o in [self.front_overlap, self.side_overlap] {
            if !(0.0..=0.95).contains(&o) {
                bail!("Überlappung muss zwischen 0 und 95 % liegen");
            }
        }
        if !(1.0..=30.0).contains(&self.speed) {
            bail!("Geschwindigkeit muss zwischen 1 und 30 m/s liegen");
        }
        if !self.direction.is_finite() {
            bail!("Flugrichtung fehlt");
        }
        Ok(())
    }
}

/// Eine Bahn: Anfang und Ende in UTM, in Flugreihenfolge
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Line {
    pub start: [f64; 2],
    pub end: [f64; 2],
}

impl Line {
    pub fn length(&self) -> f64 {
        (self.end[0] - self.start[0]).hypot(self.end[1] - self.start[1])
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlightPlan {
    /// Flughöhe über dem Startpunkt in m
    pub altitude: f64,
    pub line_spacing: f64,
    pub photo_spacing: f64,
    /// Geschwindigkeit in m/s, höchstens Fotoabstand / kürzestes Fotointervall
    pub speed: f64,
    pub lines: Vec<Line>,
    /// Weg entlang der Bahnen und zwischen ihnen, ohne An- und Abflug, in m
    pub length: f64,
    /// Flugzeit für `length` in s
    pub duration: f64,
    pub photos: usize,
    /// überflogene Fläche des Bereichs in m²
    pub area: f64,
}

/// Punkt um den Ursprung drehen (Winkel im Bogenmaß, gegen den Uhrzeigersinn)
fn rotate([x, y]: [f64; 2], a: f64) -> [f64; 2] {
    let (s, c) = a.sin_cos();
    [x * c - y * s, x * s + y * c]
}

/// x-Werte, an denen die Waagerechte y den Ring schneidet
fn crossings(ring: &[[f64; 2]], y: f64) -> Vec<f64> {
    let n = ring.len();
    (0..n)
        .filter_map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let between = (a[1] <= y && y < b[1]) || (b[1] <= y && y < a[1]);
            between.then(|| a[0] + (y - a[1]) / (b[1] - a[1]) * (b[0] - a[0]))
        })
        .collect()
}

/// Ausdehnung des Rings längs der Waagerechten y, auch knapp neben der Kante (y ± Halbbreite des Fotos)
fn span(ring: &[[f64; 2]], y: f64, half: f64) -> Option<(f64, f64)> {
    let mut xs: Vec<f64> = [-half, 0.0, half]
        .iter()
        .flat_map(|d| crossings(ring, y + d))
        .collect();
    // Ecken innerhalb des Streifens, den das Foto abdeckt
    xs.extend(
        ring.iter()
            .filter(|p| (p[1] - y).abs() <= half)
            .map(|p| p[0]),
    );
    let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (lo <= hi).then_some((lo, hi))
}

/// Bahnen über dem Bereich; Fotos beginnen und enden einen Fotoabstand vor bzw. hinter der Grenze
pub fn plan(area: &Polygon, cam: &Camera, p: &FlightParams) -> Result<FlightPlan> {
    p.validate()?;
    let altitude = p.gsd * cam.focal_mm * f64::from(cam.image_w_px) / cam.sensor_w_mm;
    let across = p.gsd * f64::from(cam.image_w_px);
    let along = p.gsd * f64::from(cam.image_h_px);
    let line_spacing = across * (1.0 - p.side_overlap);
    let photo_spacing = along * (1.0 - p.front_overlap);
    let speed = p
        .speed
        .min(photo_spacing / cam.min_interval_s)
        .min(cam.max_speed);

    // Flugrichtung auf die x-Achse drehen: Richtung α (von Nord im Uhrzeigersinn) ↔ mathematisch 90° − α
    let heading = (90.0 - p.direction).to_radians();
    let center = {
        let b = area.bbox();
        [(b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0]
    };
    let ring: Vec<[f64; 2]> = area
        .exterior
        .iter()
        .map(|q| rotate([q[0] - center[0], q[1] - center[1]], -heading))
        .collect();
    let (lo, hi) = ring
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), q| {
            (lo.min(q[1]), hi.max(q[1]))
        });
    // Bahnen mittig über die Breite, die äußeren auf oder jenseits der Grenze
    let count = ((hi - lo) / line_spacing).ceil() as usize + 1;
    let first = (lo + hi) / 2.0 - (count - 1) as f64 * line_spacing / 2.0;

    let back = |q: [f64; 2]| {
        let r = rotate(q, heading);
        [r[0] + center[0], r[1] + center[1]]
    };
    let mut lines = Vec::new();
    for i in 0..count {
        let y = first + i as f64 * line_spacing;
        let Some((x0, x1)) = span(&ring, y, line_spacing / 2.0) else {
            continue;
        };
        let (a, b) = ([x0 - photo_spacing, y], [x1 + photo_spacing, y]);
        let (a, b) = if lines.len() % 2 == 0 { (a, b) } else { (b, a) };
        lines.push(Line {
            start: back(a),
            end: back(b),
        });
    }
    if lines.is_empty() {
        bail!("Bereich ist leer");
    }
    let on_lines: f64 = lines.iter().map(Line::length).sum();
    let between: f64 = lines
        .windows(2)
        .map(|w| (w[1].start[0] - w[0].end[0]).hypot(w[1].start[1] - w[0].end[1]))
        .sum();
    let photos = lines
        .iter()
        .map(|l| (l.length() / photo_spacing).floor() as usize + 1)
        .sum();
    Ok(FlightPlan {
        altitude,
        line_spacing,
        photo_spacing,
        speed,
        length: on_lines + between,
        duration: (on_lines + between) / speed,
        photos,
        area: area.area(),
        lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const X0: f64 = 512_000.0;
    const Y0: f64 = 5_403_000.0;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn altitude_and_spacing_from_gsd() {
        let field = Polygon::rect(X0, Y0, X0 + 100.0, Y0 + 60.0);
        let p = plan(&field, &M4E, &FlightParams::default()).unwrap();
        // H = 0,02 · 12,29 · 5280 / 17,3 = 75,02 m – DJI gibt für die M4E GSD ≈ H / 37,5 an: 75 / 37,5 = 2 cm
        assert!(close(p.altitude, 75.02, 0.01), "{}", p.altitude);
        assert!(close(p.altitude / 37.5, 2.0, 0.01));
        // Bahnabstand 0,02 · 5280 · 0,3 = 31,68 m, Fotoabstand 0,02 · 3956 · 0,2 = 15,824 m
        assert!(close(p.line_spacing, 31.68, 1e-9));
        assert!(close(p.photo_spacing, 15.824, 1e-9));
        // 15,824 m alle 0,7 s erlauben 22,6 m/s, also gilt die gewünschte Geschwindigkeit
        assert_eq!(p.speed, 10.0);
        assert_eq!(p.area, 6000.0);
    }

    #[test]
    fn north_south_lines_cover_the_rectangle() {
        // 100 m Ost–West breit, Bahnen Nord–Süd: ⌈100 / 31,68⌉ + 1 = 5 Bahnen, mittig um x = 50, von Ost nach West
        let field = Polygon::rect(X0, Y0, X0 + 100.0, Y0 + 60.0);
        let p = plan(&field, &M4E, &FlightParams::default()).unwrap();
        assert_eq!(p.lines.len(), 5);
        let xs: Vec<f64> = p.lines.iter().map(|l| l.start[0] - X0).collect();
        for (x, want) in xs.iter().zip([113.36, 81.68, 50.0, 18.32, -13.36]) {
            assert!(close(*x, want, 1e-6), "{xs:?}");
        }
        // jede Bahn senkrecht, 60 m + 2 · 15,824 m lang, abwechselnd nach Nord und nach Süd
        for (i, l) in p.lines.iter().enumerate() {
            assert!(close(l.start[0], l.end[0], 1e-6));
            assert!(close(l.length(), 60.0 + 2.0 * 15.824, 1e-6));
            assert_eq!(l.end[1] > l.start[1], i % 2 == 0);
        }
        // Weg: 5 Bahnen à 91,648 m + 4 Wechsel à 31,68 m
        assert!(close(p.length, 5.0 * 91.648 + 4.0 * 31.68, 1e-6));
        assert!(close(p.duration, p.length / 10.0, 1e-9));
        // Fotos: ⌊91,648 / 15,824⌋ + 1 = 6 je Bahn
        assert_eq!(p.photos, 30);
    }

    #[test]
    fn direction_rotates_the_lines() {
        let field = Polygon::rect(X0, Y0, X0 + 100.0, Y0 + 60.0);
        let east = plan(
            &field,
            &M4E,
            &FlightParams {
                direction: 90.0,
                ..Default::default()
            },
        )
        .unwrap();
        // Ost–West über 60 m Breite: ⌈60 / 31,68⌉ + 1 = 3 Bahnen, 100 + 2 · 15,824 m lang
        assert_eq!(east.lines.len(), 3);
        for l in &east.lines {
            assert!(close(l.start[1], l.end[1], 1e-6));
            assert!(close(l.length(), 131.648, 1e-6));
        }
        // erste Bahn von West nach Ost
        assert!(east.lines[0].end[0] > east.lines[0].start[0]);
    }

    #[test]
    fn triangle_lines_follow_the_outline() {
        // Dreieck: unten 100 m breit, Spitze oben – Bahnen Ost–West werden nach oben kürzer
        let tri = Polygon::new(vec![[X0, Y0], [X0 + 100.0, Y0], [X0 + 50.0, Y0 + 100.0]]).unwrap();
        let p = plan(
            &tri,
            &M4E,
            &FlightParams {
                direction: 90.0,
                ..Default::default()
            },
        )
        .unwrap();
        let lens: Vec<f64> = p.lines.iter().map(Line::length).collect();
        assert!(lens.windows(2).all(|w| w[1] < w[0]), "{lens:?}");
        // jede Bahn reicht über den Streifen, den ihr Foto zeigt
        let p0 = &p.lines[0];
        assert!(p0.start[0].min(p0.end[0]) <= X0 - 15.0);
    }

    #[test]
    fn speed_is_limited_by_the_photo_interval() {
        // 1 cm GSD, 90 % längs: Fotoabstand 0,01 · 3956 · 0,1 = 3,956 m → 3,956 / 0,7 = 5,65 m/s
        let field = Polygon::rect(X0, Y0, X0 + 50.0, Y0 + 50.0);
        let p = plan(
            &field,
            &M4E,
            &FlightParams {
                gsd: 0.01,
                front_overlap: 0.9,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(close(p.speed, 3.956 / 0.7, 1e-9));
        assert!(plan(
            &field,
            &M4E,
            &FlightParams {
                gsd: 0.5,
                ..Default::default()
            }
        )
        .is_err());
    }
}
