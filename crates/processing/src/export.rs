//! Exporte für CAD/Vermessung: Höhenlinien, DXF, LandXML (TIN), OBJ/PLY (Mesh), CSV.

use crate::geom::Polygon;
use crate::grid::Grid;
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::Path;

fn create(path: &Path) -> Result<BufWriter<std::fs::File>> {
    Ok(BufWriter::new(std::fs::File::create(path).with_context(|| format!("{} nicht schreibbar", path.display()))?))
}

// ---------------------------------------------------------------- Höhenlinien

/// Eine Höhenlinie (offen oder geschlossen).
#[derive(Debug, Clone)]
pub struct Contour {
    pub z: f64,
    pub points: Vec<[f64; 2]>,
    pub closed: bool,
}

/// Höhenlinien im Abstand `interval` (Marching Squares über die Zellmitten).
/// Höhen werden auf Vielfache des Intervalls gelegt (z. B. 0,5 m → 312,0 / 312,5 …).
pub fn contours(g: &Grid, interval: f64) -> Result<Vec<Contour>> {
    if !(interval > 0.0) {
        bail!("Intervall muss > 0 sein");
    }
    let vals: Vec<f64> = g.data.iter().filter(|v| !v.is_nan()).map(|&v| v as f64).collect();
    if vals.is_empty() {
        return Ok(Vec::new());
    }
    let lo = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut out = Vec::new();
    let mut k = (lo / interval).ceil() as i64;
    while (k as f64) * interval <= hi {
        out.extend(contour_level(g, k as f64 * interval));
        k += 1;
    }
    Ok(out)
}

fn contour_level(g: &Grid, level: f64) -> Vec<Contour> {
    let s = g.spec;
    let (cols, rows) = (s.cols, s.rows);
    if cols < 2 || rows < 2 {
        return Vec::new();
    }
    let node = |c: usize, r: usize| r * cols + c;
    let val = |i: usize| g.data[i] as f64;
    // Kante zwischen zwei Gitterknoten → Schnittpunkt
    let cross = |a: usize, b: usize| -> [f64; 2] {
        let (ca, ra) = (a % cols, a / cols);
        let (cb, rb) = (b % cols, b / cols);
        let (za, zb) = (val(a), val(b));
        let t = ((level - za) / (zb - za)).clamp(0.0, 1.0);
        let (xa, ya) = s.center(ca, ra);
        let (xb, yb) = s.center(cb, rb);
        [xa + t * (xb - xa), ya + t * (yb - ya)]
    };
    let key = |a: usize, b: usize| if a < b { (a, b) } else { (b, a) };

    let mut segs: Vec<((usize, usize), (usize, usize))> = Vec::new();
    for r in 0..rows - 1 {
        for c in 0..cols - 1 {
            // Ecken im Uhrzeigersinn: oben links, oben rechts, unten rechts, unten links
            let n = [node(c, r), node(c + 1, r), node(c + 1, r + 1), node(c, r + 1)];
            if n.iter().any(|&i| g.data[i].is_nan()) {
                continue;
            }
            let above: Vec<bool> = n.iter().map(|&i| val(i) >= level).collect();
            let e = [key(n[0], n[1]), key(n[1], n[2]), key(n[2], n[3]), key(n[3], n[0])];
            let idx = above.iter().enumerate().fold(0, |acc, (i, &b)| acc | ((b as u8) << i));
            let pairs: &[(usize, usize)] = match idx {
                0 | 15 => &[],
                1 | 14 => &[(3, 0)],
                2 | 13 => &[(0, 1)],
                3 | 12 => &[(3, 1)],
                4 | 11 => &[(1, 2)],
                6 | 9 => &[(0, 2)],
                7 | 8 => &[(2, 3)],
                5 | 10 => {
                    let center = n.iter().map(|&i| val(i)).sum::<f64>() / 4.0;
                    // Sattel: Mittelwert entscheidet, welche Ecken verbunden sind
                    if (center >= level) == (idx == 5) { &[(3, 2), (0, 1)] } else { &[(3, 0), (1, 2)] }
                }
                _ => unreachable!(),
            };
            for &(a, b) in pairs {
                segs.push((e[a], e[b]));
            }
        }
    }

    // Segmente zu Linien verketten
    let mut by_edge: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (i, (a, b)) in segs.iter().enumerate() {
        by_edge.entry(*a).or_default().push(i);
        by_edge.entry(*b).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let mut lines = Vec::new();
    for start in 0..segs.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut chain = std::collections::VecDeque::from([segs[start].0, segs[start].1]);
        for forward in [true, false] {
            loop {
                let end = if forward { *chain.back().unwrap() } else { *chain.front().unwrap() };
                let next = by_edge.get(&end).and_then(|v| v.iter().copied().find(|&j| !used[j]));
                let Some(j) = next else { break };
                used[j] = true;
                let other = if segs[j].0 == end { segs[j].1 } else { segs[j].0 };
                if forward { chain.push_back(other) } else { chain.push_front(other) }
            }
        }
        let closed = chain.len() > 2 && chain.front() == chain.back();
        if closed {
            chain.pop_back();
        }
        lines.push(Contour { z: level, points: chain.iter().map(|&(a, b)| cross(a, b)).collect(), closed });
    }
    lines
}

// ---------------------------------------------------------------- Mesh / TIN

/// Dreiecksnetz aus dem Raster: ein Knoten je Zelle mit Daten, zwei Dreiecke je vollständigem Quadrat
/// (eins, wenn nur drei Ecken Daten haben). Dreiecke sind von oben gesehen gegen den Uhrzeigersinn.
#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[u32; 3]>,
}

impl Mesh {
    /// `stride` > 1 dünnt das Raster aus (jede n-te Zelle), `boundary` begrenzt auf Zellen im Polygon.
    pub fn from_grid(g: &Grid, stride: usize, boundary: Option<&Polygon>) -> Self {
        let s = g.spec;
        let st = stride.max(1);
        let mask = s.mask(boundary);
        let (nc, nr) = ((s.cols - 1) / st + 1, (s.rows - 1) / st + 1);
        let mut id = vec![u32::MAX; nc * nr];
        let mut m = Mesh::default();
        for rr in 0..nr {
            for cc in 0..nc {
                let (c, r) = (cc * st, rr * st);
                let i = r * s.cols + c;
                let z = g.data[i];
                if z.is_nan() || !mask[i] {
                    continue;
                }
                let (x, y) = s.center(c, r);
                id[rr * nc + cc] = m.vertices.len() as u32;
                m.vertices.push([x, y, z as f64]);
            }
        }
        for rr in 0..nr.saturating_sub(1) {
            for cc in 0..nc.saturating_sub(1) {
                let tl = id[rr * nc + cc];
                let tr = id[rr * nc + cc + 1];
                let bl = id[(rr + 1) * nc + cc];
                let br = id[(rr + 1) * nc + cc + 1];
                let ok = |v: u32| v != u32::MAX;
                match (ok(tl), ok(tr), ok(bl), ok(br)) {
                    (true, true, true, true) => {
                        m.faces.push([bl, br, tr]);
                        m.faces.push([bl, tr, tl]);
                    }
                    (false, true, true, true) => m.faces.push([bl, br, tr]),
                    (true, false, true, true) => m.faces.push([bl, br, tl]),
                    (true, true, false, true) => m.faces.push([br, tr, tl]),
                    (true, true, true, false) => m.faces.push([bl, tr, tl]),
                    _ => {}
                }
            }
        }
        m
    }

    /// Volumen zwischen Netz und waagrechter Ebene `z0` (Prismen), positiv oberhalb.
    pub fn volume_above(&self, z0: f64) -> f64 {
        self.faces
            .iter()
            .map(|f| {
                let [a, b, c] = f.map(|i| self.vertices[i as usize]);
                let area = ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])) / 2.0;
                area * ((a[2] + b[2] + c[2]) / 3.0 - z0)
            })
            .sum()
    }

    /// Wavefront OBJ. `offset` wird abgezogen (viele Programme rechnen nur mit float32).
    pub fn write_obj(&self, path: &Path, offset: [f64; 3]) -> Result<()> {
        let mut w = create(path)?;
        writeln!(w, "# fixpunkt – Offset {} {} {}", offset[0], offset[1], offset[2])?;
        for v in &self.vertices {
            writeln!(w, "v {:.3} {:.3} {:.3}", v[0] - offset[0], v[1] - offset[1], v[2] - offset[2])?;
        }
        for f in &self.faces {
            writeln!(w, "f {} {} {}", f[0] + 1, f[1] + 1, f[2] + 1)?;
        }
        w.flush()?;
        Ok(())
    }

    /// Binäres PLY mit double-Koordinaten (volle UTM-Genauigkeit).
    pub fn write_ply(&self, path: &Path, offset: [f64; 3]) -> Result<()> {
        let mut w = create(path)?;
        write!(
            w,
            "ply\nformat binary_little_endian 1.0\ncomment fixpunkt offset {} {} {}\nelement vertex {}\n\
             property double x\nproperty double y\nproperty double z\nelement face {}\n\
             property list uchar int vertex_indices\nend_header\n",
            offset[0],
            offset[1],
            offset[2],
            self.vertices.len(),
            self.faces.len()
        )?;
        for v in &self.vertices {
            for k in 0..3 {
                w.write_all(&(v[k] - offset[k]).to_le_bytes())?;
            }
        }
        for f in &self.faces {
            w.write_all(&[3u8])?;
            for i in f {
                w.write_all(&(*i as i32).to_le_bytes())?;
            }
        }
        w.flush()?;
        Ok(())
    }

    /// LandXML 1.2 mit einer TIN-Oberfläche (Punkte als Nordwert Ostwert Höhe, wie in LandXML üblich).
    pub fn write_landxml(&self, path: &Path, name: &str, epsg: Option<u32>) -> Result<()> {
        let mut w = create(path)?;
        let (date, time) = now_utc();
        writeln!(w, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
        writeln!(
            w,
            r#"<LandXML xmlns="http://www.landxml.org/schema/LandXML-1.2" version="1.2" date="{date}" time="{time}">"#
        )?;
        writeln!(
            w,
            r#"  <Units><Metric areaUnit="squareMeter" linearUnit="meter" volumeUnit="cubicMeter" temperatureUnit="celsius" pressureUnit="milliBars" angularUnit="decimal degrees" directionUnit="decimal degrees"/></Units>"#
        )?;
        if let Some(e) = epsg {
            writeln!(w, r#"  <CoordinateSystem epsgCode="{e}"/>"#)?;
        }
        writeln!(w, r#"  <Application name="fixpunkt" version="{}"/>"#, env!("CARGO_PKG_VERSION"))?;
        writeln!(w, r#"  <Surfaces>"#)?;
        writeln!(w, r#"    <Surface name="{}">"#, xml_escape(name))?;
        writeln!(w, r#"      <Definition surfType="TIN">"#)?;
        writeln!(w, "        <Pnts>")?;
        for (i, v) in self.vertices.iter().enumerate() {
            writeln!(w, r#"          <P id="{}">{:.3} {:.3} {:.3}</P>"#, i + 1, v[1], v[0], v[2])?;
        }
        writeln!(w, "        </Pnts>\n        <Faces>")?;
        for f in &self.faces {
            writeln!(w, "          <F>{} {} {}</F>", f[0] + 1, f[1] + 1, f[2] + 1)?;
        }
        writeln!(w, "        </Faces>\n      </Definition>\n    </Surface>\n  </Surfaces>\n</LandXML>")?;
        w.flush()?;
        Ok(())
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// (JJJJ-MM-TT, hh:mm:ss) in UTC – ohne zusätzliche Abhängigkeit.
fn now_utc() -> (String, String) {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Hinnant: civil_from_days
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + (m <= 2) as i64;
    (format!("{y:04}-{m:02}-{d:02}"), format!("{:02}:{:02}:{:02}", rem / 3600, rem % 3600 / 60, rem % 60))
}

// ---------------------------------------------------------------- DXF

/// Einfacher DXF-Schreiber (Version R12, ASCII) – öffnet in AutoCAD, BricsCAD, QGIS, Civil-Programmen.
pub struct Dxf {
    body: String,
}

impl Default for Dxf {
    fn default() -> Self {
        Self::new()
    }
}

impl Dxf {
    pub fn new() -> Self {
        Dxf { body: String::new() }
    }

    fn g(&mut self, code: u16, v: impl std::fmt::Display) {
        use std::fmt::Write as _;
        let _ = write!(self.body, "{code}\n{v}\n");
    }

    pub fn point(&mut self, layer: &str, p: [f64; 3]) {
        self.g(0, "POINT");
        self.g(8, layer);
        self.xyz(p, 10);
    }

    pub fn text(&mut self, layer: &str, p: [f64; 3], height: f64, text: &str) {
        self.g(0, "TEXT");
        self.g(8, layer);
        self.xyz(p, 10);
        self.g(40, format!("{height:.3}"));
        self.g(1, text.replace(['\n', '\r'], " "));
    }

    /// 3D-Polylinie
    pub fn polyline(&mut self, layer: &str, pts: &[[f64; 3]], closed: bool) {
        self.g(0, "POLYLINE");
        self.g(8, layer);
        self.g(66, 1);
        self.xyz([0.0; 3], 10);
        self.g(70, 8 | closed as u8);
        for p in pts {
            self.g(0, "VERTEX");
            self.g(8, layer);
            self.xyz(*p, 10);
            self.g(70, 32);
        }
        self.g(0, "SEQEND");
        self.g(8, layer);
    }

    /// Dreiecksfläche (3DFACE)
    pub fn face(&mut self, layer: &str, a: [f64; 3], b: [f64; 3], c: [f64; 3]) {
        self.g(0, "3DFACE");
        self.g(8, layer);
        self.xyz(a, 10);
        self.xyz(b, 11);
        self.xyz(c, 12);
        self.xyz(c, 13);
    }

    fn xyz(&mut self, p: [f64; 3], base: u16) {
        self.g(base, format!("{:.4}", p[0]));
        self.g(base + 10, format!("{:.4}", p[1]));
        self.g(base + 20, format!("{:.4}", p[2]));
    }

    pub fn contours(&mut self, cs: &[Contour], major_every: f64) {
        for c in cs {
            let major = major_every > 0.0 && ((c.z / major_every).round() * major_every - c.z).abs() < 1e-6;
            let layer = if major { "HOEHENLINIEN_HAUPT" } else { "HOEHENLINIEN" };
            let pts: Vec<[f64; 3]> = c.points.iter().map(|p| [p[0], p[1], c.z]).collect();
            self.polyline(layer, &pts, c.closed);
        }
    }

    pub fn mesh(&mut self, m: &Mesh, layer: &str) {
        for f in &m.faces {
            let [a, b, c] = f.map(|i| m.vertices[i as usize]);
            self.face(layer, a, b, c);
        }
    }

    pub fn render(&self) -> String {
        format!(
            "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1009\n9\n$INSUNITS\n70\n6\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n{}0\nENDSEC\n0\nEOF\n",
            self.body
        )
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.render()).with_context(|| format!("{} nicht schreibbar", path.display()))
    }
}

// ---------------------------------------------------------------- CSV

/// Raster als Punktliste `x;y;z` (nur Zellen mit Daten, optional nur jede n-te Zelle).
pub fn write_grid_csv(g: &Grid, path: &Path, stride: usize) -> Result<()> {
    let mut w = create(path)?;
    writeln!(w, "x;y;z")?;
    let st = stride.max(1);
    let s = g.spec;
    for r in (0..s.rows).step_by(st) {
        for c in (0..s.cols).step_by(st) {
            let z = g.get(c, r);
            if !z.is_nan() {
                let (x, y) = s.center(c, r);
                writeln!(w, "{x:.3};{y:.3};{z:.3}")?;
            }
        }
    }
    w.flush()?;
    Ok(())
}

/// Profil als CSV `station;x;y;z`.
pub fn write_profile_csv(p: &[crate::measure::ProfilePoint], path: &Path) -> Result<()> {
    let mut w = create(path)?;
    writeln!(w, "station;x;y;z")?;
    for q in p {
        let z = q.z.map(|z| format!("{z:.3}")).unwrap_or_default();
        writeln!(w, "{:.3};{:.3};{:.3};{z}", q.station, q.x, q.y)?;
    }
    w.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridSpec;

    fn cone() -> Grid {
        // Kegel: Höhe 10 − Abstand zur Mitte
        let spec = GridSpec { min_x: 500_000.0, max_y: 5_400_020.0, cell: 0.25, cols: 80, rows: 80 };
        let mut g = Grid::new_nodata(spec);
        for r in 0..80 {
            for c in 0..80 {
                let (x, y) = spec.center(c, r);
                let d = ((x - 500_010.0).powi(2) + (y - 5_400_010.0).powi(2)).sqrt();
                g.set(c, r, (10.0 - d) as f32);
            }
        }
        g
    }

    #[test]
    fn contours_of_cone_are_closed_circles() {
        let g = cone();
        let cs = contours(&g, 1.0).unwrap();
        let c5: Vec<_> = cs.iter().filter(|c| (c.z - 5.0).abs() < 1e-9).collect();
        assert_eq!(c5.len(), 1);
        let ring = c5[0];
        assert!(ring.closed);
        for p in &ring.points {
            let d = ((p[0] - 500_010.0).powi(2) + (p[1] - 5_400_010.0).powi(2)).sqrt();
            assert!((d - 5.0).abs() < 0.02, "{d}");
        }
        // Umfang ≈ 2π·5
        let n = ring.points.len();
        let len: f64 = (0..n)
            .map(|i| {
                let (a, b) = (ring.points[i], ring.points[(i + 1) % n]);
                ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
            })
            .sum();
        assert!((len - 2.0 * std::f64::consts::PI * 5.0).abs() < 0.1, "{len}");
    }

    #[test]
    fn mesh_volume_and_orientation() {
        let g = cone();
        let m = Mesh::from_grid(&g, 1, None);
        assert_eq!(m.vertices.len(), 6400);
        assert_eq!(m.faces.len(), 2 * 79 * 79);
        // alle Dreiecke gegen den Uhrzeigersinn → positive Fläche
        assert!(m.faces.iter().all(|f| {
            let [a, b, c] = f.map(|i| m.vertices[i as usize]);
            (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]) > 0.0
        }));
        // Kegel über z = 0 (Radius 10, Höhe 10, Ecken außerhalb negativ abgeschnitten → nur Kegel testen)
        let clipped = Mesh {
            vertices: m.vertices.iter().map(|v| [v[0], v[1], v[2].max(0.0)]).collect(),
            faces: m.faces.clone(),
        };
        let exact = std::f64::consts::PI * 100.0 * 10.0 / 3.0;
        assert!((clipped.volume_above(0.0) - exact).abs() / exact < 0.02, "{}", clipped.volume_above(0.0));
        let thin = Mesh::from_grid(&g, 4, None);
        assert_eq!(thin.vertices.len(), 20 * 20);
    }

    #[test]
    fn writers_produce_valid_files() {
        let g = cone();
        let m = Mesh::from_grid(&g, 8, None);
        let dir = std::env::temp_dir().join(format!("fixpunkt-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        m.write_landxml(&dir.join("s.xml"), "Kegel & Co", Some(25832)).unwrap();
        let x = std::fs::read_to_string(dir.join("s.xml")).unwrap();
        assert!(x.contains(r#"epsgCode="25832""#) && x.contains("Kegel &amp; Co"));
        assert_eq!(x.matches("<P id=").count(), m.vertices.len());
        assert_eq!(x.matches("<F>").count(), m.faces.len());
        // LandXML: Nordwert zuerst
        assert!(x.contains(&format!("{:.3} {:.3}", m.vertices[0][1], m.vertices[0][0])));

        m.write_obj(&dir.join("m.obj"), [500_000.0, 5_400_000.0, 0.0]).unwrap();
        let o = std::fs::read_to_string(dir.join("m.obj")).unwrap();
        assert_eq!(o.lines().filter(|l| l.starts_with("v ")).count(), m.vertices.len());

        m.write_ply(&dir.join("m.ply"), [0.0; 3]).unwrap();
        let ply = std::fs::read(dir.join("m.ply")).unwrap();
        let hdr_end = ply.windows(11).position(|w| w == b"end_header\n").unwrap() + 11;
        assert_eq!(ply.len() - hdr_end, m.vertices.len() * 24 + m.faces.len() * 13);

        let mut dxf = Dxf::new();
        dxf.contours(&contours(&g, 1.0).unwrap(), 5.0);
        dxf.point("PUNKTE", [500_001.0, 5_400_001.0, 3.0]);
        dxf.text("PUNKTE", [500_001.0, 5_400_001.0, 3.0], 0.5, "P1");
        dxf.mesh(&Mesh::from_grid(&g, 20, None), "DGM");
        let s = dxf.render();
        assert!(s.starts_with("0\nSECTION\n2\nHEADER") && s.ends_with("0\nEOF\n"));
        assert_eq!(s.matches("\nPOLYLINE\n").count(), s.matches("\nSEQEND\n").count());
        assert!(s.contains("HOEHENLINIEN_HAUPT") && s.contains("3DFACE"));
        // Gruppencodes und Werte wechseln sich ab: gerade Zeilenzahl
        assert_eq!(s.lines().count() % 2, 0);

        write_grid_csv(&g, &dir.join("g.csv"), 10).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("g.csv")).unwrap().lines().count(), 1 + 64);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn date_format() {
        let (d, t) = now_utc();
        assert_eq!(d.len(), 10);
        assert_eq!(t.len(), 8);
        assert!(d.as_str() >= "2024-01-01");
    }
}
