//! Koordinaten: ETRS89 geographisch (Breite/Länge) ↔ ETRS89/UTM (z. B. Zone 32 = EPSG:25832)
//! und Höhen: ellipsoidisch ↔ DHHN2016 (Normalhöhe) über ein Quasigeoid-Raster wie GCG2016.
//!
//! UTM nach Krüger-Reihen 6. Ordnung (Karney 2011) – Genauigkeit weit unter 1 mm innerhalb der Zone,
//! ganz ohne PROJ-Abhängigkeit.
//!
//! Hinweis: GNSS mit SAPOS-Korrekturen liefert ETRS89. Koordinaten aus Web-Karten (WGS84/ITRF)
//! weichen in Deutschland inzwischen um ca. 0,8–1 m davon ab – für Abrechnungsgrenzen meist egal,
//! für Messungen nicht.

use anyhow::{bail, Context, Result};
use std::io::{BufRead, BufReader};
use std::path::Path;

/// GRS80 (Ellipsoid von ETRS89)
const A: f64 = 6_378_137.0;
const F: f64 = 1.0 / 298.257_222_101;
const K0: f64 = 0.9996;
const FALSE_EASTING: f64 = 500_000.0;

/// Eine UTM-Zone auf der Nordhalbkugel (ETRS89/UTM: EPSG:258zz).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utm {
    pub zone: u8,
}

impl Default for Utm {
    /// Zone 32 (EPSG:25832) – Standard in den meisten Bundesländern.
    fn default() -> Self {
        Utm { zone: 32 }
    }
}

struct Series {
    a_rect: f64,
    e: f64,
    alpha: [f64; 6],
    beta: [f64; 6],
}

fn series() -> Series {
    let n = F / (2.0 - F);
    let (n2, n3, n4, n5, n6) = (n * n, n.powi(3), n.powi(4), n.powi(5), n.powi(6));
    let a_rect = A / (1.0 + n) * (1.0 + n2 / 4.0 + n4 / 64.0 + n6 / 256.0);
    let alpha = [
        n / 2.0 - 2.0 / 3.0 * n2 + 5.0 / 16.0 * n3 + 41.0 / 180.0 * n4 - 127.0 / 288.0 * n5
            + 7891.0 / 37800.0 * n6,
        13.0 / 48.0 * n2 - 3.0 / 5.0 * n3 + 557.0 / 1440.0 * n4 + 281.0 / 630.0 * n5
            - 1983433.0 / 1935360.0 * n6,
        61.0 / 240.0 * n3 - 103.0 / 140.0 * n4 + 15061.0 / 26880.0 * n5 + 167603.0 / 181440.0 * n6,
        49561.0 / 161280.0 * n4 - 179.0 / 168.0 * n5 + 6601661.0 / 7257600.0 * n6,
        34729.0 / 80640.0 * n5 - 3418889.0 / 1995840.0 * n6,
        212378941.0 / 319334400.0 * n6,
    ];
    let beta = [
        n / 2.0 - 2.0 / 3.0 * n2 + 37.0 / 96.0 * n3 - 1.0 / 360.0 * n4 - 81.0 / 512.0 * n5
            + 96199.0 / 604800.0 * n6,
        1.0 / 48.0 * n2 + 1.0 / 15.0 * n3 - 437.0 / 1440.0 * n4 + 46.0 / 105.0 * n5
            - 1118711.0 / 3870720.0 * n6,
        17.0 / 480.0 * n3 - 37.0 / 840.0 * n4 - 209.0 / 4480.0 * n5 + 5569.0 / 90720.0 * n6,
        4397.0 / 161280.0 * n4 - 11.0 / 504.0 * n5 - 830251.0 / 7257600.0 * n6,
        4583.0 / 161280.0 * n5 - 108847.0 / 3991680.0 * n6,
        20648693.0 / 638668800.0 * n6,
    ];
    Series {
        a_rect,
        e: (F * (2.0 - F)).sqrt(),
        alpha,
        beta,
    }
}

impl Utm {
    pub fn new(zone: u8) -> Result<Self> {
        if !(1..=60).contains(&zone) {
            bail!("UTM-Zone muss zwischen 1 und 60 liegen");
        }
        Ok(Utm { zone })
    }

    /// EPSG-Code von ETRS89/UTM in dieser Zone (nur Zonen 28–38 sind als 258zz definiert).
    pub fn epsg(&self) -> u32 {
        25800 + self.zone as u32
    }

    pub fn central_meridian_deg(&self) -> f64 {
        self.zone as f64 * 6.0 - 183.0
    }

    /// Breite/Länge in Grad → (Ostwert, Nordwert) in m.
    pub fn forward(&self, lat_deg: f64, lon_deg: f64) -> (f64, f64) {
        let s = series();
        let phi = lat_deg.to_radians();
        let lam = (lon_deg - self.central_meridian_deg()).to_radians();
        let tau = phi.tan();
        let sigma = (s.e * (s.e * tau / (1.0 + tau * tau).sqrt()).atanh()).sinh();
        let tau_p = tau * (1.0 + sigma * sigma).sqrt() - sigma * (1.0 + tau * tau).sqrt();
        let xi_p = tau_p.atan2(lam.cos());
        let eta_p = (lam.sin() / (tau_p * tau_p + lam.cos().powi(2)).sqrt()).asinh();
        let (mut xi, mut eta) = (xi_p, eta_p);
        for (j, a) in s.alpha.iter().enumerate() {
            let k = 2.0 * (j + 1) as f64;
            xi += a * (k * xi_p).sin() * (k * eta_p).cosh();
            eta += a * (k * xi_p).cos() * (k * eta_p).sinh();
        }
        (FALSE_EASTING + K0 * s.a_rect * eta, K0 * s.a_rect * xi)
    }

    /// (Ostwert, Nordwert) in m → Breite/Länge in Grad.
    pub fn inverse(&self, easting: f64, northing: f64) -> (f64, f64) {
        let s = series();
        let xi = northing / (K0 * s.a_rect);
        let eta = (easting - FALSE_EASTING) / (K0 * s.a_rect);
        let (mut xi_p, mut eta_p) = (xi, eta);
        for (j, b) in s.beta.iter().enumerate() {
            let k = 2.0 * (j + 1) as f64;
            xi_p -= b * (k * xi).sin() * (k * eta).cosh();
            eta_p -= b * (k * xi).cos() * (k * eta).sinh();
        }
        let tau_p = xi_p.sin() / (eta_p.sinh().powi(2) + xi_p.cos().powi(2)).sqrt();
        let e2 = s.e * s.e;
        let mut tau = tau_p;
        for _ in 0..20 {
            let sigma = (s.e * (s.e * tau / (1.0 + tau * tau).sqrt()).atanh()).sinh();
            let ti = tau * (1.0 + sigma * sigma).sqrt() - sigma * (1.0 + tau * tau).sqrt();
            let d = (tau_p - ti) / (1.0 + ti * ti).sqrt() * (1.0 + (1.0 - e2) * tau * tau)
                / ((1.0 - e2) * (1.0 + tau * tau).sqrt());
            tau += d;
            if d.abs() < 1e-14 {
                break;
            }
        }
        let lat = tau.atan().to_degrees();
        let lon = eta_p.sinh().atan2(xi_p.cos()).to_degrees() + self.central_meridian_deg();
        (lat, lon)
    }
}

/// UTM-Zone eines EPSG-Codes für UTM Nord: ETRS89 (258zz) oder WGS84 (326zz); sonst `None`.
/// ETRS89 und WGS84 trennen in Europa einige Dezimeter; RTK-Korrekturen (SAPOS) liefern ETRS89, daher
/// werden beide hier gleich behandelt.
pub fn utm_zone(epsg: u32) -> Option<u8> {
    match epsg {
        25801..=25860 | 32601..=32660 => Some((epsg % 100) as u8),
        _ => None,
    }
}

impl Utm {
    /// Ost-/Nordwert aus dieser Zone in die Zone `to` (über Breite/Länge, mm-genau)
    pub fn to_zone(&self, to: &Utm, easting: f64, northing: f64) -> (f64, f64) {
        if self.zone == to.zone {
            return (easting, northing);
        }
        let (lat, lon) = self.inverse(easting, northing);
        to.forward(lat, lon)
    }
}

/// Quasigeoid-/Geoidraster (z. B. GCG2016 des BKG): Höhenanomalie N je Breite/Länge.
/// Normalhöhe DHHN2016 = ellipsoidische Höhe (ETRS89) − N.
#[derive(Debug, Clone)]
pub struct Geoid {
    lat_min: f64,
    lon_min: f64,
    dlat: f64,
    dlon: f64,
    rows: usize,
    cols: usize,
    /// Zeile 0 = südlichste Breite
    n: Vec<f64>,
}

impl Geoid {
    /// Liest ein Raster in einem von zwei Textformaten:
    /// - **GRAVSOFT**: erste Zeile `lat_min lat_max lon_min lon_max dlat dlon`, dann die Werte
    ///   zeilenweise von Nord nach Süd, je Zeile von West nach Ost.
    /// - **Spalten** `lat lon N` (eine Zeile pro Gitterpunkt, regelmäßiges Gitter, beliebige Reihenfolge).
    pub fn read(path: &Path) -> Result<Self> {
        let f = std::fs::File::open(path)
            .with_context(|| format!("Geoid {} nicht lesbar", path.display()))?;
        let mut rows: Vec<Vec<f64>> = Vec::new();
        for line in BufReader::new(f).lines() {
            let line = line?;
            let vals: Vec<f64> = line
                .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
                .filter(|s| !s.is_empty())
                .map_while(|s| s.parse().ok())
                .collect();
            if !vals.is_empty() {
                rows.push(vals);
            }
        }
        if rows.is_empty() {
            bail!("Geoid {}: keine Werte", path.display());
        }
        let gravsoft_count = |h: &[f64]| {
            let r = ((h[1] - h[0]) / h[4]).round() + 1.0;
            let c = ((h[3] - h[2]) / h[5]).round() + 1.0;
            (r * c) as usize
        };
        let is_gravsoft = rows[0].len() == 6
            && rows[0][4] > 0.0
            && rows[0][5] > 0.0
            && rows[1..].iter().map(Vec::len).sum::<usize>() == gravsoft_count(&rows[0]);
        if is_gravsoft {
            let h = &rows[0];
            let flat: Vec<f64> = rows[1..].iter().flatten().copied().collect();
            Self::from_gravsoft(h[0], h[1], h[2], h[3], h[4], h[5], flat)
        } else if rows.iter().all(|r| r.len() >= 3) {
            Self::from_columns(rows.iter().map(|r| [r[0], r[1], r[2]]).collect())
        } else {
            bail!(
                "Geoid {}: Format nicht erkannt (GRAVSOFT oder 'lat lon N')",
                path.display()
            )
        }
    }

    pub fn from_gravsoft(
        lat_min: f64,
        lat_max: f64,
        lon_min: f64,
        lon_max: f64,
        dlat: f64,
        dlon: f64,
        values_north_first: Vec<f64>,
    ) -> Result<Self> {
        let rows = ((lat_max - lat_min) / dlat).round() as usize + 1;
        let cols = ((lon_max - lon_min) / dlon).round() as usize + 1;
        if values_north_first.len() != rows * cols {
            bail!(
                "Geoid: {} Werte, erwartet {}×{}",
                values_north_first.len(),
                rows,
                cols
            );
        }
        let mut n = vec![0.0; rows * cols];
        for r in 0..rows {
            let src = rows - 1 - r;
            n[r * cols..(r + 1) * cols]
                .copy_from_slice(&values_north_first[src * cols..(src + 1) * cols]);
        }
        Ok(Geoid {
            lat_min,
            lon_min,
            dlat,
            dlon,
            rows,
            cols,
            n,
        })
    }

    pub fn from_columns(pts: Vec<[f64; 3]>) -> Result<Self> {
        let uniq = |i: usize| {
            let mut v: Vec<f64> = pts.iter().map(|p| p[i]).collect();
            v.sort_by(f64::total_cmp);
            v.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
            v
        };
        let (lats, lons) = (uniq(0), uniq(1));
        if lats.len() < 2 || lons.len() < 2 {
            bail!("Geoid: mindestens 2×2 Gitterpunkte nötig");
        }
        let dlat = (lats[lats.len() - 1] - lats[0]) / (lats.len() - 1) as f64;
        let dlon = (lons[lons.len() - 1] - lons[0]) / (lons.len() - 1) as f64;
        let (rows, cols) = (lats.len(), lons.len());
        let mut n = vec![f64::NAN; rows * cols];
        for p in &pts {
            let r = ((p[0] - lats[0]) / dlat).round() as usize;
            let c = ((p[1] - lons[0]) / dlon).round() as usize;
            n[r * cols + c] = p[2];
        }
        if n.iter().any(|v| v.is_nan()) {
            bail!("Geoid: Gitter ist nicht vollständig bzw. nicht regelmäßig");
        }
        Ok(Geoid {
            lat_min: lats[0],
            lon_min: lons[0],
            dlat,
            dlon,
            rows,
            cols,
            n,
        })
    }

    /// Höhenanomalie N in m an (Breite, Länge) – bilinear interpoliert.
    pub fn undulation(&self, lat: f64, lon: f64) -> Result<f64> {
        let fr = (lat - self.lat_min) / self.dlat;
        let fc = (lon - self.lon_min) / self.dlon;
        if fr < 0.0 || fc < 0.0 || fr > (self.rows - 1) as f64 || fc > (self.cols - 1) as f64 {
            bail!("Punkt {lat:.5}/{lon:.5} liegt außerhalb des Geoidrasters");
        }
        let r0 = (fr.floor() as usize).min(self.rows - 2);
        let c0 = (fc.floor() as usize).min(self.cols - 2);
        let (ty, tx) = (fr - r0 as f64, fc - c0 as f64);
        let g = |r: usize, c: usize| self.n[r * self.cols + c];
        Ok(g(r0, c0) * (1.0 - tx) * (1.0 - ty)
            + g(r0, c0 + 1) * tx * (1.0 - ty)
            + g(r0 + 1, c0) * (1.0 - tx) * ty
            + g(r0 + 1, c0 + 1) * tx * ty)
    }

    /// Ellipsoidische Höhe → Normalhöhe (DHHN2016 bei GCG2016).
    pub fn to_normal_height(&self, lat: f64, lon: f64, h_ellipsoid: f64) -> Result<f64> {
        Ok(h_ellipsoid - self.undulation(lat, lon)?)
    }

    /// Normalhöhe → ellipsoidische Höhe.
    pub fn to_ellipsoidal_height(&self, lat: f64, lon: f64, h_normal: f64) -> Result<f64> {
        Ok(h_normal + self.undulation(lat, lon)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Meridianbogenlänge numerisch (Simpson) – unabhängige Kontrolle der Reihen.
    fn meridian_arc(lat_deg: f64) -> f64 {
        let e2 = F * (2.0 - F);
        let m = |p: f64| A * (1.0 - e2) / (1.0 - e2 * p.sin().powi(2)).powf(1.5);
        let n = 20_000;
        let h = lat_deg.to_radians() / n as f64;
        let mut s = m(0.0) + m(lat_deg.to_radians());
        for i in 1..n {
            s += m(i as f64 * h) * if i % 2 == 1 { 4.0 } else { 2.0 };
        }
        s * h / 3.0
    }

    #[test]
    fn central_meridian_matches_arc_length() {
        let u = Utm::default();
        for lat in [0.0, 30.0, 47.5, 48.7758, 54.9] {
            let (e, n) = u.forward(lat, 9.0);
            assert!((e - 500_000.0).abs() < 1e-6);
            assert!((n - K0 * meridian_arc(lat)).abs() < 1e-4, "{lat}: {n}");
        }
    }

    #[test]
    fn roundtrip_across_zone() {
        let u = Utm::default();
        for &(lat, lon) in &[
            (48.7758, 9.1829),
            (47.3, 5.9),
            (54.8, 12.1),
            (50.0, 6.0),
            (52.5, 13.4),
        ] {
            let (e, n) = u.forward(lat, lon);
            let (la, lo) = u.inverse(e, n);
            // 1e-9 Grad ≈ 0,1 mm
            assert!(
                (la - lat).abs() < 1e-9 && (lo - lon).abs() < 1e-9,
                "{lat},{lon} → {la},{lo}"
            );
        }
    }

    #[test]
    fn local_scale_is_conformal() {
        // In einer konformen Abbildung ist der Maßstab in Nord- und Ostrichtung gleich.
        let u = Utm::default();
        let (lat, lon) = (49.0_f64, 11.5_f64);
        let d = 1e-5;
        let (e0, n0) = u.forward(lat, lon);
        let (e1, n1) = u.forward(lat + d, lon);
        let (e2, n2) = u.forward(lat, lon + d);
        let a2 = A * A;
        let e2c = F * (2.0 - F);
        let w = (1.0 - e2c * lat.to_radians().sin().powi(2)).sqrt();
        let m = A * (1.0 - e2c) / w.powi(3) * d.to_radians();
        let p = (a2.sqrt() / w) * lat.to_radians().cos() * d.to_radians();
        let k_north = ((e1 - e0).powi(2) + (n1 - n0).powi(2)).sqrt() / m;
        let k_east = ((e2 - e0).powi(2) + (n2 - n0).powi(2)).sqrt() / p;
        assert!((k_north - k_east).abs() < 1e-6, "{k_north} {k_east}");
        assert!(k_north > K0 && k_north < 1.001);
    }

    #[test]
    fn geoid_bilinear_and_formats() {
        // 3×3-Gitter 48..49 / 9..10, N = 47 + (lat-48) + 2(lon-9)
        let f = |la: f64, lo: f64| 47.0 + (la - 48.0) + 2.0 * (lo - 9.0);
        let mut vals = Vec::new();
        for la in [49.0, 48.5, 48.0] {
            for lo in [9.0, 9.5, 10.0] {
                vals.push(f(la, lo));
            }
        }
        let g = Geoid::from_gravsoft(48.0, 49.0, 9.0, 10.0, 0.5, 0.5, vals).unwrap();
        assert!((g.undulation(48.3, 9.7).unwrap() - f(48.3, 9.7)).abs() < 1e-9);
        assert!(
            (g.to_normal_height(48.3, 9.7, 400.0).unwrap() - (400.0 - f(48.3, 9.7))).abs() < 1e-9
        );
        assert!(g.undulation(50.0, 9.5).is_err());

        let mut cols = Vec::new();
        for la in [48.0, 48.5, 49.0] {
            for lo in [10.0, 9.0, 9.5] {
                cols.push([la, lo, f(la, lo)]);
            }
        }
        let dir = std::env::temp_dir().join(format!("fixpunkt-geoid-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // GRAVSOFT mit genau 3 Werten pro Zeile darf nicht als Spaltenformat erkannt werden
        std::fs::write(
            dir.join("g.txt"),
            "48 49 9 10 0.5 0.5\n1 1 1\n2 2 2\n3 3 3\n",
        )
        .unwrap();
        let gf = Geoid::read(&dir.join("g.txt")).unwrap();
        assert!((gf.undulation(48.0, 9.0).unwrap() - 3.0).abs() < 1e-12);
        std::fs::write(dir.join("c.txt"), "48 9 1\n48 10 1\n49 9 2\n49 10 2\n").unwrap();
        assert!(
            (Geoid::read(&dir.join("c.txt"))
                .unwrap()
                .undulation(48.5, 9.5)
                .unwrap()
                - 1.5)
                .abs()
                < 1e-12
        );
        std::fs::remove_dir_all(&dir).ok();
        let g2 = Geoid::from_columns(cols).unwrap();
        assert!((g2.undulation(48.9, 9.1).unwrap() - f(48.9, 9.1)).abs() < 1e-9);
    }

    #[test]
    fn zones_from_epsg() {
        assert_eq!(utm_zone(25832), Some(32));
        assert_eq!(utm_zone(32633), Some(33));
        assert_eq!(utm_zone(4326), None);
        assert_eq!(utm_zone(31467), None);
    }

    #[test]
    fn zone_change_is_symmetric_at_12_degrees() {
        // 12° Ost liegt je 3° von den Mittelmeridianen 9° (Zone 32) und 15° (Zone 33): Ostwerte spiegeln sich
        // um 500 000 m, Nordwerte sind gleich
        let (z32, z33) = (Utm::new(32).unwrap(), Utm::new(33).unwrap());
        let (e33, n33) = z33.forward(50.0, 12.0);
        let (e32, n32) = z33.to_zone(&z32, e33, n33);
        assert!(
            (e32 - 500_000.0 - (500_000.0 - e33)).abs() < 1e-6,
            "{e32} {e33}"
        );
        assert!((n32 - n33).abs() < 1e-6);
        // hin und zurück
        let (e, n) = z32.to_zone(&z33, e32, n32);
        assert!((e - e33).abs() < 1e-6 && (n - n33).abs() < 1e-6);
        assert_eq!(z32.to_zone(&z32, 1.0, 2.0), (1.0, 2.0));
    }
}
