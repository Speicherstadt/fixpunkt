//! End-to-End: synthetische Baustelle erzeugen und jeden Befehl der Kommandozeile ausführen.

use fixpunkt::pointcloud::{Point3, PointCloud};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const X0: f64 = 512_000.0;
const Y0: f64 = 5_403_000.0;

struct Site {
    dir: PathBuf,
}

impl Drop for Site {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

impl Site {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("fixpunkt-cli-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        Site { dir }
    }

    fn p(&self, f: &str) -> PathBuf {
        self.dir.join(f)
    }
}

/// Urgelände: 60 × 60 m, 3 % Gefälle nach Osten
fn terrain(dx: f64, _dy: f64) -> f64 {
    310.0 - 0.03 * dx
}

/// Baugrube 20 × 20 m, 3 m tief, senkrechte Wände, Mitte bei (30, 30)
fn in_pit(dx: f64, dy: f64) -> bool {
    (20.0..40.0).contains(&dx) && (20.0..40.0).contains(&dy)
}

fn cloud(f: impl Fn(f64, f64) -> f64, step: f64) -> Vec<Point3> {
    let n = (60.0 / step) as usize;
    let mut pts = Vec::new();
    for j in 0..n {
        for i in 0..n {
            // halbe Schrittweite versetzt → Punkte nicht auf Zellgrenzen
            let (dx, dy) = ((i as f64 + 0.5) * step, (j as f64 + 0.5) * step);
            pts.push(Point3 { x: X0 + dx, y: Y0 + dy, z: f(dx, dy) });
        }
    }
    pts
}

fn run(args: &[&str]) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_fixpunkt")).args(args).output().unwrap();
    assert!(out.status.success(), "{args:?}\n{}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&out.stdout).into()))
}

fn run_fail(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_fixpunkt")).args(args).output().unwrap();
    assert!(!out.status.success(), "{args:?} sollte fehlschlagen");
    String::from_utf8_lossy(&out.stderr).into()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn f(v: &Value, path: &str) -> f64 {
    v.pointer(path).and_then(Value::as_f64).unwrap_or_else(|| panic!("{path} fehlt in {v}"))
}

fn write_boundary(site: &Site) -> PathBuf {
    let p = site.p("grenze.geojson");
    let ring = |x0: f64, y0: f64, x1: f64, y1: f64| {
        format!("[[[{x0},{y0}],[{x1},{y0}],[{x1},{y1}],[{x0},{y1}],[{x0},{y0}]]]", x0 = X0 + x0, y0 = Y0 + y0, x1 = X0 + x1, y1 = Y0 + y1)
    };
    std::fs::write(
        &p,
        format!(
            r#"{{"type":"FeatureCollection","features":[
              {{"type":"Feature","properties":{{"name":"Grube"}},"geometry":{{"type":"Polygon","coordinates":{}}}}},
              {{"type":"Feature","properties":{{"name":"Rand"}},"geometry":{{"type":"Polygon","coordinates":{}}}}}]}}"#,
            ring(15.0, 15.0, 45.0, 45.0),
            ring(46.0, 5.0, 55.0, 55.0)
        ),
    )
    .unwrap();
    p
}

#[test]
fn excavation_workflow() {
    let site = Site::new("aushub");
    let before = site.p("vorher.laz");
    let after = site.p("nachher.laz");
    PointCloud::new(cloud(terrain, 0.2)).write(&before, None).unwrap();
    // nachher: Grube + Bagger (5 × 3 m, 3,5 m hoch) am Rand außerhalb der Grube, unklassifiziert
    // (Photogrammetrie sieht nur die Oberseite: unter dem Bagger gibt es keine Bodenpunkte)
    let bagger_at = |dx: f64, dy: f64| (8.0..13.0).contains(&dx) && (25.0..28.0).contains(&dy);
    let pts: Vec<Point3> = cloud(
        |dx, dy| {
            let z = terrain(dx, dy) - if in_pit(dx, dy) { 3.0 } else { 0.0 };
            if bagger_at(dx, dy) { z + 3.5 } else { z }
        },
        0.2,
    );
    PointCloud::new(pts).write(&after, None).unwrap();
    let boundary = write_boundary(&site);

    // info
    let info = run(&["info", s(&after)]);
    assert_eq!(f(&info, "/points") as usize, 90_000);

    // Aushub je Bereich: Grube = 20 × 20 × 3 = 1200 m³, Rand = 0
    let diff_tif = site.p("diff.tif");
    let d = run(&["diff", s(&before), s(&after), "--boundary", s(&boundary), "--cell", "0.25", "--diff-out", s(&diff_tif)]);
    let cut = f(&d, "/regions/0/cut_m3");
    assert!((cut - 1200.0).abs() < 12.0, "Grube: {cut}");
    assert!(f(&d, "/regions/1/cut_m3") < 1.0);
    assert_eq!(d.pointer("/regions/0/name").unwrap(), "Grube");
    assert!((f(&d, "/total/cut_m3") - cut - f(&d, "/regions/1/cut_m3")).abs() < 1e-6);
    let dt = run(&["info", s(&diff_tif)]);
    assert_eq!(dt["epsg"], 25832);

    // Bagger liegt in keinem Bereich → mit Bereich um den Bagger: Auftrag sichtbar, mit Bodenfilter weg
    let bagger = site.p("bagger.geojson");
    std::fs::write(
        &bagger,
        format!(r#"{{"type":"Polygon","coordinates":[[[{a},{b}],[{c},{b}],[{c},{d}],[{a},{d}],[{a},{b}]]]}}"#, a = X0 + 5.0, b = Y0 + 22.0, c = X0 + 16.0, d = Y0 + 31.0),
    )
    .unwrap();
    let raw = run(&["diff", s(&before), s(&after), "--boundary", s(&bagger)]);
    assert!(f(&raw, "/fill_m3") > 40.0, "{raw}");
    let filtered = run(&["diff", s(&before), s(&after), "--boundary", s(&bagger), "--classify-ground"]);
    assert!(f(&filtered, "/fill_m3") < 2.0, "{filtered}");

    // ground: LAZ mit Klassen → ground-only liefert dieselbe Bilanz
    let classified = site.p("nachher_boden.laz");
    let g = run(&["ground", s(&after), s(&classified), "--dtm-out", s(&site.p("dtm.tif"))]);
    assert!(f(&g, "/non_ground") >= 300.0, "{g}");
    let gi = run(&["info", s(&classified)]);
    assert!(f(&gi, "/ground_points") < f(&gi, "/points"));
    let go = run(&["diff", s(&before), s(&classified), "--boundary", s(&bagger), "--ground-only"]);
    assert!(f(&go, "/fill_m3") < 2.0, "{go}");

    // Raster als Eingabe (z. B. ODM-DGM): gleiches Ergebnis
    let dem0 = site.p("vorher.tif");
    let dem1 = site.p("nachher.tif");
    run(&["dem", s(&before), "-o", s(&dem0), "--cell", "0.25"]);
    run(&["dem", s(&after), "-o", s(&dem1), "--cell", "0.25"]);
    let dr = run(&["diff", s(&dem0), s(&dem1), "--boundary", s(&boundary)]);
    assert!((f(&dr, "/regions/0/cut_m3") - cut).abs() < 15.0, "{dr}");

    // Zeitreihe: halbe Grube, dann volle Grube
    let half = site.p("halb.laz");
    PointCloud::new(cloud(|dx, dy| terrain(dx, dy) - if in_pit(dx, dy) && dx < 30.0 { 3.0 } else { 0.0 }, 0.2))
        .write(&half, None)
        .unwrap();
    let grube = site.p("grube.geojson");
    std::fs::write(
        &grube,
        format!(r#"{{"type":"Polygon","coordinates":[[[{a},{a2}],[{b},{a2}],[{b},{b2}],[{a},{b2}],[{a},{a2}]]]}}"#, a = X0 + 15.0, b = X0 + 45.0, a2 = Y0 + 15.0, b2 = Y0 + 45.0),
    )
    .unwrap();
    let ser = run(&["series", s(&before), s(&half), s(&dem1), "--boundary", s(&grube), "--labels", "KW10,KW12"]);
    assert_eq!(ser[0]["label"], "KW10");
    assert!((f(&ser, "/0/since_start/cut_m3") - 600.0).abs() < 10.0, "{ser}");
    assert!((f(&ser, "/1/since_previous/cut_m3") - 600.0).abs() < 10.0, "{ser}");

    // Soll/Ist: Sohle soll auf Gelände − 3 m liegen, als geneigte Ebene (3 % nach Osten)
    let cmp = run(&[
        "compare", s(&after), "--boundary", s(&site.p("sohle.geojson").tap_write(X0 + 22.0, Y0 + 22.0, X0 + 38.0, Y0 + 38.0)),
        "--slope", &format!("{X0},{Y0},307,3,90"), "--tolerance", "0.03",
    ]);
    assert!(f(&cmp, "/within_tolerance_percent") > 99.0, "{cmp}");

    // Messen + Profil quer durch die Grube
    let line = site.p("achse.txt");
    std::fs::write(&line, format!("{} {}\n{} {}\n", X0 + 10.0, Y0 + 30.0, X0 + 50.0, Y0 + 30.0)).unwrap();
    let m = run(&["measure", s(&before), "--line", s(&line), "--at", &format!("{},{}", X0 + 30.0, Y0 + 30.0)]);
    assert!((f(&m, "/line/length_2d") - 40.0).abs() < 1e-6);
    assert!((f(&m, "/line/slope_percent") + 3.0).abs() < 0.1, "{m}");
    assert!((f(&m, "/points/0/z") - terrain(30.0, 30.0)).abs() < 0.02);
    let csv = site.p("profil.csv");
    let pr = run(&["profile", s(&after), "--line", s(&line), "--step", "1", "--csv", s(&csv)]);
    assert_eq!(f(&pr, "/points") as usize, 41);
    let rows: Vec<String> = std::fs::read_to_string(&csv).unwrap().lines().map(String::from).collect();
    let z_mid: f64 = rows[21].split(';').nth(3).unwrap().parse().unwrap(); // Station 20 = Grubenmitte
    assert!((z_mid - (terrain(30.0, 30.0) - 3.0)).abs() < 0.05, "{z_mid}");

    // Exporte
    let e = run(&[
        "export", s(&after), "--boundary", s(&boundary), "--classify-ground", "--cell", "0.5",
        "--tif", s(&site.p("e.tif")), "--dxf", s(&site.p("e.dxf")), "--landxml", s(&site.p("e.xml")),
        "--obj", s(&site.p("e.obj")), "--ply", s(&site.p("e.ply")), "--csv", s(&site.p("e.csv")),
        "--contours", "1", "--stride", "2", "--local",
    ]);
    assert_eq!(e["written"].as_array().unwrap().len(), 6);
    assert!(f(&e, "/contours") > 0.0);
    let dxf = std::fs::read_to_string(site.p("e.dxf")).unwrap();
    assert!(dxf.contains("HOEHENLINIEN") && dxf.contains("GRENZE"));
    let xml = std::fs::read_to_string(site.p("e.xml")).unwrap();
    assert!(xml.contains("<Surface") && xml.contains(r#"epsgCode="25832""#));

    // Fehlerfall: falsches Format
    assert!(run_fail(&["dem", s(&before), "-o", s(&site.p("x.png"))]).contains(".tif"));
}

trait TapWrite {
    fn tap_write(self, x0: f64, y0: f64, x1: f64, y1: f64) -> Self;
}

impl TapWrite for PathBuf {
    fn tap_write(self, x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        std::fs::write(&self, format!(r#"{{"type":"Polygon","coordinates":[[[{x0},{y0}],[{x1},{y0}],[{x1},{y1}],[{x0},{y1}],[{x0},{y0}]]]}}"#)).unwrap();
        self
    }
}

#[test]
fn stockpile_and_cleaning() {
    let site = Site::new("haufen");
    // Kegel r = 6, h = 3 auf ebenem Gelände + zwei Ausreißer
    let mut pts = cloud(|dx, dy| {
        let d = ((dx - 30.0).powi(2) + (dy - 30.0).powi(2)).sqrt();
        300.0 + (3.0 * (1.0 - d / 6.0)).max(0.0)
    }, 0.2);
    pts.push(Point3 { x: X0 + 10.0, y: Y0 + 10.0, z: 330.0 });
    pts.push(Point3 { x: X0 + 50.0, y: Y0 + 40.0, z: 280.0 });
    let raw = site.p("haufen.laz");
    PointCloud::new(pts).write(&raw, None).unwrap();

    let clean = site.p("sauber.laz");
    let c = run(&["clean", s(&raw), s(&clean), "--neighbors", "8"]);
    assert_eq!(f(&c, "/outliers/removed") as usize, 2, "{c}");
    let thin = site.p("duenn.xyz");
    let t = run(&["clean", s(&clean), s(&thin), "--neighbors", "0", "--voxel", "1"]);
    assert!(f(&t, "/points_out") < 5000.0);

    let b = site.p("haufen.geojson");
    std::fs::write(
        &b,
        format!(r#"{{"type":"Polygon","coordinates":[[[{a},{a2}],[{c},{a2}],[{c},{c2}],[{a},{c2}],[{a},{a2}]]]}}"#, a = X0 + 20.0, c = X0 + 40.0, a2 = Y0 + 20.0, c2 = Y0 + 40.0),
    )
    .unwrap();
    let v = run(&["stockpile", s(&clean), "--boundary", s(&b), "--base", "plane"]);
    let exact = std::f64::consts::PI * 36.0 * 3.0 / 3.0;
    assert!((f(&v, "/fill_m3") - exact).abs() / exact < 0.02, "{v} vs {exact}");
    let low = run(&["stockpile", s(&clean), "--boundary", s(&b), "--base", "299"]);
    assert!((f(&low, "/fill_m3") - exact - 400.0).abs() < 4.0, "{low}");
}

#[test]
fn georeferencing_and_coordinates() {
    let site = Site::new("georef");
    // lokales Scan-System: um 30° gedreht, Ursprung irgendwo
    let (a, s_, c_) = (30f64.to_radians(), 1.0, (X0 + 100.0, Y0 + 200.0, 250.0));
    let to_utm = |p: [f64; 3]| {
        [c_.0 + s_ * (a.cos() * p[0] - a.sin() * p[1]), c_.1 + s_ * (a.sin() * p[0] + a.cos() * p[1]), c_.2 + p[2]]
    };
    let gcps = [[0.0, 0.0, 0.0], [20.0, 0.0, 0.5], [0.0, 25.0, -0.3], [22.0, 24.0, 1.0]];
    let mut csv = String::from("name;x;y;z;E;N;H\n");
    for (i, g) in gcps.iter().enumerate() {
        let u = to_utm(*g);
        csv += &format!("GCP{i};{};{};{};{:.4};{:.4};{:.4}\n", g[0], g[1], g[2], u[0], u[1], u[2]);
    }
    let gcp = site.p("gcp.csv");
    std::fs::write(&gcp, csv).unwrap();
    let local = site.p("lokal.las");
    PointCloud::new((0..100).map(|i| Point3 { x: i as f64 * 0.3, y: 5.0, z: 1.0 }).collect()).write(&local, None).unwrap();
    let out = site.p("utm.laz");
    let r = run(&["georef", "--gcp", s(&gcp), "--mode", "rigid", s(&local), s(&out)]);
    assert!(f(&r, "/rms_3d") < 0.001, "{r}");
    assert!((f(&r, "/rotation_deg/2") - 30.0).abs() < 1e-3);
    let back = PointCloud::read(&out, Default::default()).unwrap();
    let e = to_utm([3.0, 5.0, 1.0]);
    assert!((back.points[10].x - e[0]).abs() < 0.002 && (back.points[10].y - e[1]).abs() < 0.002);

    // Koordinaten: Hin und zurück, Höhe über Geoid (konstant 47,5 m)
    let geoid = site.p("geoid.txt");
    std::fs::write(&geoid, "47.0 49.0 8.0 10.0 1.0 1.0\n47.5 47.5 47.5\n47.5 47.5 47.5\n47.5 47.5 47.5\n").unwrap();
    let pts = site.p("punkte.txt");
    std::fs::write(&pts, "P1 48.5 9.0 400.0\n").unwrap();
    let fwd = run(&["coords", s(&pts), "--geoid", s(&geoid)]);
    let line = fwd.as_str().unwrap().trim().to_string();
    let v: Vec<&str> = line.split(' ').collect();
    assert_eq!(v[0], "P1");
    assert!((v[1].parse::<f64>().unwrap() - 500_000.0).abs() < 1e-3);
    assert!((v[3].parse::<f64>().unwrap() - 352.5).abs() < 1e-6);
    let utm = site.p("utm.txt");
    std::fs::write(&utm, &line).unwrap();
    let inv = run(&["coords", s(&utm), "--geoid", s(&geoid), "--inverse"]);
    let v: Vec<f64> = inv.as_str().unwrap().split_whitespace().skip(1).map(|t| t.parse().unwrap()).collect();
    assert!((v[0] - 48.5).abs() < 1e-8 && (v[1] - 9.0).abs() < 1e-8 && (v[2] - 400.0).abs() < 1e-6);
}
