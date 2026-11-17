//! Kommandozeile der Fixpunkt-Engine. Jeder Befehl gibt sein Ergebnis als JSON auf stdout aus
//! (Fehler: Meldung auf stderr, Exit-Code ≠ 0) – so kann der Worker der Web-App ihn direkt aufrufen.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use fixpunkt::compare::{self, Design};
use fixpunkt::crs::{Geoid, Utm};
use fixpunkt::export::{self, Dxf, Mesh};
use fixpunkt::filter::{self, GroundParams};
use fixpunkt::geom::{self, BBox, Polygon, Region};
use fixpunkt::grid::{Aggregate, Grid, GridSpec};
use fixpunkt::pointcloud::{self, Point3, PointCloud, ReadOptions};
use fixpunkt::surface::{self, SurfaceOptions};
use fixpunkt::transform::{self, Mode};
use fixpunkt::volume::{self, Base};
use fixpunkt::{geotiff, measure};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// Fixpunkt-Engine: Punktwolken aufbereiten, georeferenzieren, messen, Volumen berechnen, exportieren.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// EPSG-Code des Lagesystems für Ausgaben (GeoTIFF, LandXML)
    #[arg(long, global = true, default_value_t = 25832)]
    epsg: u16,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Args, Clone)]
struct SurfaceArgs {
    /// Zellgröße des Höhenrasters in m
    #[arg(long, default_value_t = 0.25)]
    cell: f64,
    /// nur Bodenpunkte (LAS-Klasse 2)
    #[arg(long)]
    ground_only: bool,
    /// Bodenfilter vorher selbst rechnen (für unklassifizierte Wolken)
    #[arg(long)]
    classify_ground: bool,
    #[arg(long, value_enum, default_value_t = Aggregate::Mean)]
    aggregate: Aggregate,
    /// Lücken bis zu so vielen Zellen auffüllen
    #[arg(long, default_value_t = 4)]
    fill: usize,
}

impl SurfaceArgs {
    fn options(&self) -> SurfaceOptions {
        SurfaceOptions {
            cell: self.cell,
            ground_only: self.ground_only,
            classify_ground: self.classify_ground.then(GroundParams::default),
            aggregate: self.aggregate,
            fill: self.fill,
        }
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// Überblick über eine Punktwolke oder ein Raster
    Info { input: PathBuf },
    /// Höhenraster (DGM/DOM) erzeugen → .tif oder .asc
    Dem {
        input: PathBuf,
        #[arg(long, short)]
        out: PathBuf,
        #[command(flatten)]
        s: SurfaceArgs,
        /// nur innerhalb dieser Grenze (GeoJSON)
        #[arg(long)]
        boundary: Option<PathBuf>,
    },
    /// Aushub/Auftrag: Differenz zweier Aufnahmen (Punktwolke oder Raster), je Bereich
    Diff {
        before: PathBuf,
        after: PathBuf,
        #[command(flatten)]
        s: SurfaceArgs,
        /// Abrechnungsgrenze(n) als GeoJSON; mehrere Polygone = mehrere Bereiche
        #[arg(long)]
        boundary: Option<PathBuf>,
        /// Differenzraster schreiben (.tif oder .asc)
        #[arg(long)]
        diff_out: Option<PathBuf>,
    },
    /// Haufwerk gegen Basisfläche (plane | lowest | <Höhe>)
    Stockpile {
        surface: PathBuf,
        #[command(flatten)]
        s: SurfaceArgs,
        #[arg(long)]
        boundary: PathBuf,
        #[arg(long, default_value = "plane")]
        base: String,
    },
    /// Zeitreihe: jede Aufnahme gegen Urgelände und gegen die vorherige
    Series {
        base: PathBuf,
        #[arg(required = true)]
        epochs: Vec<PathBuf>,
        #[command(flatten)]
        s: SurfaceArgs,
        #[arg(long)]
        boundary: Option<PathBuf>,
        /// Beschriftungen, kommagetrennt (Standard: Dateinamen)
        #[arg(long, value_delimiter = ',')]
        labels: Vec<String>,
    },
    /// Bodenpunkte klassifizieren (SMRF) und als LAS/LAZ schreiben (Attribute bleiben erhalten)
    Ground {
        input: PathBuf,
        output: PathBuf,
        #[arg(long, default_value_t = 1.0)]
        cell: f64,
        /// maximale Geländeneigung (Höhe/Strecke)
        #[arg(long, default_value_t = 0.7)]
        slope: f64,
        /// größtes Objekt in m
        #[arg(long, default_value_t = 10.0)]
        max_window: f64,
        /// Höhentoleranz in m
        #[arg(long, default_value_t = 0.3)]
        threshold: f64,
        /// Gelände-Raster zusätzlich schreiben (.tif/.asc)
        #[arg(long)]
        dtm_out: Option<PathBuf>,
    },
    /// Ausreißer entfernen und/oder ausdünnen
    Clean {
        input: PathBuf,
        output: PathBuf,
        /// Voxelgröße in m fürs Ausdünnen
        #[arg(long)]
        voxel: Option<f64>,
        /// Nachbarn für den Ausreißerfilter (0 = aus)
        #[arg(long, default_value_t = 8)]
        neighbors: usize,
        #[arg(long, default_value_t = 2.5)]
        std_ratio: f64,
    },
    /// Transformation aus Passpunkten bestimmen und optional auf eine Punktwolke anwenden
    Georef {
        /// CSV: name, x_quelle, y_quelle, z_quelle, x_ziel, y_ziel, z_ziel
        #[arg(long)]
        gcp: PathBuf,
        #[arg(long, value_enum, default_value_t = Mode::Similarity)]
        mode: Mode,
        input: Option<PathBuf>,
        output: Option<PathBuf>,
    },
    /// Koordinaten umrechnen: Zeilen `lat lon [h]` → `E N [H]` (oder umgekehrt mit --inverse)
    Coords {
        /// Datei oder `-` für stdin
        input: PathBuf,
        #[arg(long, default_value_t = 32)]
        zone: u8,
        /// Quasigeoid (z. B. GCG2016) für Höhen h ↔ H (DHHN2016)
        #[arg(long)]
        geoid: Option<PathBuf>,
        #[arg(long)]
        inverse: bool,
    },
    /// Messen: Punkte, Strecke/Polylinie, Fläche
    Measure {
        surface: PathBuf,
        #[command(flatten)]
        s: SurfaceArgs,
        /// Punkt `x,y` (mehrfach möglich)
        #[arg(long = "at", value_parser = parse_xy)]
        at: Vec<[f64; 2]>,
        /// Linie (GeoJSON-LineString oder `x y` je Zeile)
        #[arg(long)]
        line: Option<PathBuf>,
        /// Fläche (GeoJSON-Polygon)
        #[arg(long)]
        area: Option<PathBuf>,
    },
    /// Längs-/Querprofil entlang einer Linie
    Profile {
        surface: PathBuf,
        #[command(flatten)]
        s: SurfaceArgs,
        #[arg(long)]
        line: PathBuf,
        #[arg(long, default_value_t = 0.5)]
        step: f64,
        /// als CSV schreiben (sonst JSON)
        #[arg(long)]
        csv: Option<PathBuf>,
    },
    /// Soll/Ist-Vergleich gegen Soll-Höhe, Soll-Ebene oder Soll-DGM
    Compare {
        scan: PathBuf,
        #[command(flatten)]
        s: SurfaceArgs,
        /// waagrechte Soll-Höhe
        #[arg(long)]
        height: Option<f64>,
        /// geneigte Ebene: `x,y,z,gefälle_prozent,richtung_grad` (0° = Nord)
        #[arg(long, value_parser = parse_slope)]
        slope: Option<[f64; 5]>,
        /// Soll-DGM oder zweite Aufnahme
        #[arg(long)]
        design: Option<PathBuf>,
        #[arg(long)]
        boundary: Option<PathBuf>,
        /// Toleranz in m
        #[arg(long, default_value_t = 0.03)]
        tolerance: f64,
        /// Abweichungsraster (Ist − Soll) schreiben
        #[arg(long)]
        diff_out: Option<PathBuf>,
    },
    /// Oberfläche exportieren: GeoTIFF, ASC, DXF (Höhenlinien/DGM), LandXML, OBJ, PLY, CSV
    Export {
        surface: PathBuf,
        #[command(flatten)]
        s: SurfaceArgs,
        #[arg(long)]
        boundary: Option<PathBuf>,
        #[arg(long)]
        tif: Option<PathBuf>,
        #[arg(long)]
        asc: Option<PathBuf>,
        #[arg(long)]
        dxf: Option<PathBuf>,
        #[arg(long)]
        landxml: Option<PathBuf>,
        #[arg(long)]
        obj: Option<PathBuf>,
        #[arg(long)]
        ply: Option<PathBuf>,
        #[arg(long)]
        csv: Option<PathBuf>,
        /// Höhenlinien-Abstand in m (für DXF)
        #[arg(long, default_value_t = 0.5)]
        contours: f64,
        /// jede n-te Höhenlinie als Hauptlinie (in m)
        #[arg(long, default_value_t = 2.5)]
        major: f64,
        /// Dreiecksnetz in DXF mit ausgeben
        #[arg(long)]
        dxf_mesh: bool,
        /// Ausdünnung für Mesh/CSV (jede n-te Zelle)
        #[arg(long, default_value_t = 1)]
        stride: usize,
        /// OBJ/PLY relativ zur linken unteren Ecke schreiben (Offset steht in der Datei)
        #[arg(long)]
        local: bool,
    },
}

fn parse_xy(s: &str) -> Result<[f64; 2], String> {
    let v: Vec<f64> = s
        .split(',')
        .map(|t| t.trim().parse::<f64>())
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    if v.len() != 2 {
        return Err("erwartet x,y".into());
    }
    Ok([v[0], v[1]])
}

fn parse_slope(s: &str) -> Result<[f64; 5], String> {
    let v: Vec<f64> = s
        .split(',')
        .map(|t| t.trim().parse::<f64>())
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    <[f64; 5]>::try_from(v).map_err(|_| "erwartet x,y,z,gefälle_prozent,richtung_grad".to_string())
}

fn print(v: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(v)?);
    Ok(())
}

fn regions(p: &Option<PathBuf>) -> Result<Vec<Region>> {
    match p {
        Some(p) => Polygon::regions_from_geojson_file(p),
        None => Ok(Vec::new()),
    }
}

fn union_bbox(rs: &[Region]) -> Option<BBox> {
    if rs.is_empty() {
        return None;
    }
    let mut b = BBox::empty();
    for r in rs {
        let rb = r.polygon.bbox();
        b.extend(rb.min_x, rb.min_y);
        b.extend(rb.max_x, rb.max_y);
    }
    Some(b)
}

fn write_grid(g: &Grid, path: &Path, epsg: u16) -> Result<()> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("tif" | "tiff") => geotiff::write(g, path, Some(epsg), None),
        Some("asc") => g.write_asc(path),
        _ => bail!("{}: Raster bitte als .tif oder .asc", path.display()),
    }
}

fn is_las(p: &Path) -> bool {
    matches!(
        p.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("las" | "laz")
    )
}

/// Gemeinsames Raster für mehrere Eingaben: Grenze, sonst Schnittmenge der Ausdehnungen.
fn common_spec(inputs: &[&PathBuf], rs: &[Region], o: &SurfaceOptions) -> Result<GridSpec> {
    let cell = o.cell;
    if let Some(b) = union_bbox(rs) {
        return GridSpec::covering(&b, cell);
    }
    let mut bb: Option<BBox> = None;
    for p in inputs {
        let e = surface::extent(p, o)?;
        bb = Some(match bb {
            Some(b) => b.intersection(&e),
            None => e,
        });
    }
    GridSpec::covering(&bb.context("keine Eingabe")?, cell)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Fehler: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let epsg = cli.epsg;
    match cli.cmd {
        Cmd::Info { input } => {
            if surface::is_raster(&input) {
                let (g, e) = if input
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("asc"))
                {
                    (Grid::read_asc(&input)?, None)
                } else {
                    let r = geotiff::read(&input)?;
                    (r.grid, r.epsg)
                };
                let st = g.z_stats();
                print(&json!({
                    "type": "raster", "epsg": e, "cols": g.spec.cols, "rows": g.spec.rows, "cell": g.spec.cell,
                    "bbox": g.bbox_json(), "valid_cells": g.valid_cells(),
                    "z_min": st.map(|s| s.0), "z_max": st.map(|s| s.1), "z_mean": st.map(|s| s.2),
                }))?;
            } else {
                let c = PointCloud::read(&input, ReadOptions::default())?;
                let b = c.bbox();
                let (lo, hi) = c.z_range().unwrap_or((f64::NAN, f64::NAN));
                let area = (b.width() * b.height()).max(1e-9);
                let mut v = json!({
                    "type": "pointcloud", "points": c.len(),
                    "bbox": {"min_x": b.min_x, "min_y": b.min_y, "max_x": b.max_x, "max_y": b.max_y},
                    "z_min": lo, "z_max": hi, "density_per_m2": c.len() as f64 / area,
                });
                if is_las(&input) {
                    let g = PointCloud::read(&input, ReadOptions { ground_only: true })
                        .map(|g| g.len())
                        .unwrap_or(0);
                    v["ground_points"] = json!(g);
                }
                print(&v)?;
            }
        }

        Cmd::Dem {
            input,
            out,
            s,
            boundary,
        } => {
            let rs = regions(&boundary)?;
            let o = s.options();
            let mut g = surface::load(&input, None, union_bbox(&rs), &o)?;
            if let Some(m) = union_poly_mask(&rs, g.spec) {
                g.data.iter_mut().zip(m).for_each(|(v, k)| {
                    if !k {
                        *v = f32::NAN
                    }
                });
            }
            write_grid(&g, &out, epsg)?;
            let st = g.z_stats();
            print(
                &json!({"out": out, "cols": g.spec.cols, "rows": g.spec.rows, "cell": g.spec.cell,
                "valid_cells": g.valid_cells(), "z_min": st.map(|s| s.0), "z_max": st.map(|s| s.1)}),
            )?;
        }

        Cmd::Diff {
            before,
            after,
            s,
            boundary,
            diff_out,
        } => {
            let o = s.options();
            let rs = regions(&boundary)?;
            let spec = common_spec(&[&before, &after], &rs, &o)?;
            let g0 = surface::load(&before, Some(spec), None, &o)?;
            let g1 = surface::load(&after, Some(spec), None, &o)?;
            if let Some(p) = &diff_out {
                let union = union_poly_mask(&rs, spec);
                let (_, mut d) = volume::difference(&g0, &g1, None)?;
                if let Some(m) = union {
                    d.data.iter_mut().zip(m).for_each(|(v, k)| {
                        if !k {
                            *v = f32::NAN
                        }
                    });
                }
                write_grid(&d, p, epsg)?;
            }
            match rs.len() {
                0 => print(&volume::difference(&g0, &g1, None)?.0)?,
                1 => print(&volume::difference(&g0, &g1, Some(&rs[0].polygon))?.0)?,
                _ => {
                    let (per, total) = volume::difference_regions(&g0, &g1, &rs)?;
                    print(&json!({"regions": per, "total": total}))?;
                }
            }
        }

        Cmd::Stockpile {
            surface: path,
            s,
            boundary,
            base,
        } => {
            let rs = Polygon::regions_from_geojson_file(&boundary)?;
            let base = match base.as_str() {
                "plane" => Base::Plane,
                "lowest" => Base::Lowest,
                h => Base::Height(
                    h.parse()
                        .map_err(|_| anyhow::anyhow!("--base: plane, lowest oder Zahl"))?,
                ),
            };
            let o = s.options();
            let g = surface::load(&path, None, union_bbox(&rs), &o)?;
            if rs.len() == 1 {
                print(&volume::stockpile(&g, &rs[0].polygon, base)?)?;
            } else {
                let mut per = Vec::new();
                for r in &rs {
                    per.push(volume::RegionReport {
                        name: r.name.clone(),
                        boundary_area_m2: r.polygon.area(),
                        volume: volume::stockpile(&g, &r.polygon, base)?,
                    });
                }
                let total = volume::sum(per.iter().map(|r| &r.volume));
                print(&json!({"regions": per, "total": total}))?;
            }
        }

        Cmd::Series {
            base,
            epochs,
            s,
            boundary,
            labels,
        } => {
            let o = s.options();
            let rs = regions(&boundary)?;
            if rs.len() > 1 {
                bail!("series: bitte genau eine Grenze angeben");
            }
            let mut all: Vec<&PathBuf> = vec![&base];
            all.extend(epochs.iter());
            let spec = common_spec(&all, &rs, &o)?;
            let g0 = surface::load(&base, Some(spec), None, &o)?;
            let mut eps = Vec::new();
            for (i, p) in epochs.iter().enumerate() {
                let label = labels.get(i).cloned().unwrap_or_else(|| {
                    p.file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| format!("{}", i + 1))
                });
                eps.push((label, surface::load(p, Some(spec), None, &o)?));
            }
            print(&volume::series(&g0, &eps, rs.first().map(|r| &r.polygon))?)?;
        }

        Cmd::Ground {
            input,
            output,
            cell,
            slope,
            max_window,
            threshold,
            dtm_out,
        } => {
            let params = GroundParams {
                cell,
                slope,
                max_window,
                elevation_threshold: threshold,
                ..Default::default()
            };
            let cloud = PointCloud::read(&input, ReadOptions::default())?;
            let (cls, dtm, rep) = filter::classify_ground(&cloud, params)?;
            if is_las(&input) && is_las(&output) {
                pointcloud::rewrite_las(&input, &output, None, |i, p| {
                    if let Some(i) = i {
                        p.classification =
                            las::point::Classification::new(cls[i]).unwrap_or(p.classification);
                    }
                    true
                })?;
            } else {
                cloud.write(&output, Some(&cls))?;
            }
            if let Some(p) = dtm_out {
                write_grid(&dtm, &p, epsg)?;
            }
            print(&rep)?;
        }

        Cmd::Clean {
            input,
            output,
            voxel,
            neighbors,
            std_ratio,
        } => {
            let cloud = PointCloud::read(&input, ReadOptions::default())?;
            let mut keep = vec![true; cloud.len()];
            let mut report = json!({"points_in": cloud.len()});
            if neighbors > 0 {
                let (k, rep) = filter::statistical_outliers(&cloud, neighbors, std_ratio)?;
                keep = k;
                report["outliers"] = serde_json::to_value(rep)?;
            }
            if let Some(v) = voxel {
                let idx: Vec<usize> = (0..cloud.len()).filter(|&i| keep[i]).collect();
                let sub = PointCloud::new(idx.iter().map(|&i| cloud.points[i]).collect());
                let kept = filter::voxel_thin(&sub, v)?;
                keep = vec![false; cloud.len()];
                for k in kept {
                    keep[idx[k]] = true;
                }
            }
            let n = keep.iter().filter(|k| **k).count();
            if is_las(&input) && is_las(&output) {
                pointcloud::rewrite_las(&input, &output, None, |i, _| i.is_some_and(|i| keep[i]))?;
            } else {
                filter::select(&cloud, &keep).write(&output, None)?;
            }
            report["points_out"] = json!(n);
            print(&report)?;
        }

        Cmd::Georef {
            gcp,
            mode,
            input,
            output,
        } => {
            let pts = transform::read_control_points(&gcp)?;
            let rep = transform::fit(&pts, mode)?;
            if let Some(inp) = input {
                let out = output.context("Ausgabedatei fehlt")?;
                let t = rep.transform.clone();
                if is_las(&inp) && is_las(&out) {
                    let f = |p: [f64; 3]| t.apply(p);
                    pointcloud::rewrite_las(&inp, &out, Some(&f), |_, p| {
                        let q = t.apply([p.x, p.y, p.z]);
                        (p.x, p.y, p.z) = (q[0], q[1], q[2]);
                        true
                    })?;
                } else {
                    let c = PointCloud::read(&inp, ReadOptions::default())?;
                    let moved = PointCloud::new(
                        c.points
                            .iter()
                            .map(|p| {
                                let q = t.apply([p.x, p.y, p.z]);
                                Point3 {
                                    x: q[0],
                                    y: q[1],
                                    z: q[2],
                                }
                            })
                            .collect(),
                    );
                    moved.write(&out, None)?;
                }
            }
            print(&rep)?;
        }

        Cmd::Coords {
            input,
            zone,
            geoid,
            inverse,
        } => {
            let utm = Utm::new(zone)?;
            let geoid = geoid.as_deref().map(Geoid::read).transpose()?;
            let reader: Box<dyn BufRead> = if input.as_os_str() == "-" {
                Box::new(std::io::stdin().lock())
            } else {
                Box::new(std::io::BufReader::new(
                    std::fs::File::open(&input)
                        .with_context(|| format!("{} nicht lesbar", input.display()))?,
                ))
            };
            let mut out = std::io::stdout().lock();
            for line in reader.lines() {
                let line = line?;
                let toks: Vec<&str> = line
                    .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
                    .filter(|s| !s.is_empty())
                    .collect();
                // optional führender Punktname
                let (name, nums) = match toks.first().map(|t| t.parse::<f64>().is_ok()) {
                    Some(false) => (Some(toks[0]), &toks[1..]),
                    _ => (None, &toks[..]),
                };
                let v: Vec<f64> = nums.iter().map_while(|s| s.parse().ok()).collect();
                if v.len() < 2 {
                    continue;
                }
                let mut o: Vec<String> = name.map(|n| vec![n.to_string()]).unwrap_or_default();
                if inverse {
                    let (lat, lon) = utm.inverse(v[0], v[1]);
                    o.push(format!("{lat:.9}"));
                    o.push(format!("{lon:.9}"));
                    if let Some(h) = v.get(2) {
                        let h = match &geoid {
                            Some(g) => g.to_ellipsoidal_height(lat, lon, *h)?,
                            None => *h,
                        };
                        o.push(format!("{h:.4}"));
                    }
                } else {
                    let (e, n) = utm.forward(v[0], v[1]);
                    o.push(format!("{e:.4}"));
                    o.push(format!("{n:.4}"));
                    if let Some(h) = v.get(2) {
                        let h = match &geoid {
                            Some(g) => g.to_normal_height(v[0], v[1], *h)?,
                            None => *h,
                        };
                        o.push(format!("{h:.4}"));
                    }
                }
                writeln!(out, "{}", o.join(" "))?;
            }
        }

        Cmd::Measure {
            surface: path,
            s,
            at,
            line,
            area,
        } => {
            let mut bb: Option<BBox> = None;
            let mut grow = |x: f64, y: f64, pad: f64| {
                let b = bb.get_or_insert_with(BBox::empty);
                b.extend(x - pad, y - pad);
                b.extend(x + pad, y + pad);
            };
            let line = line.as_deref().map(geom::line_from_file).transpose()?;
            let area = area
                .as_deref()
                .map(Polygon::from_geojson_file)
                .transpose()?;
            let pad = 4.0 * s.cell;
            at.iter().for_each(|p| grow(p[0], p[1], pad));
            line.iter().flatten().for_each(|p| grow(p[0], p[1], pad));
            area.iter()
                .flat_map(|a| a.exterior.iter())
                .for_each(|p| grow(p[0], p[1], pad));
            if bb.is_none() {
                bail!("nichts zu messen: --at, --line oder --area angeben");
            }
            let g = surface::load(&path, None, bb, &s.options())?;
            let mut v = serde_json::Map::new();
            if !at.is_empty() {
                v.insert(
                    "points".into(),
                    serde_json::to_value(
                        at.iter()
                            .map(|p| measure::point(&g, p[0], p[1]))
                            .collect::<Vec<_>>(),
                    )?,
                );
            }
            if let Some(l) = &line {
                v.insert("line".into(), serde_json::to_value(measure::line(&g, l)?)?);
            }
            if let Some(a) = &area {
                v.insert("area".into(), serde_json::to_value(measure::area(&g, a))?);
            }
            print(&Value::Object(v))?;
        }

        Cmd::Profile {
            surface: path,
            s,
            line,
            step,
            csv,
        } => {
            let l = geom::line_from_file(&line)?;
            let mut b = BBox::empty();
            for p in &l {
                b.extend(p[0] - 4.0 * s.cell, p[1] - 4.0 * s.cell);
                b.extend(p[0] + 4.0 * s.cell, p[1] + 4.0 * s.cell);
            }
            let g = surface::load(&path, None, Some(b), &s.options())?;
            let prof = measure::profile(&g, &l, step)?;
            match csv {
                Some(p) => {
                    export::write_profile_csv(&prof, &p)?;
                    print(
                        &json!({"out": p, "points": prof.len(), "summary": measure::line(&g, &l)?}),
                    )?;
                }
                None => print(&prof)?,
            }
        }

        Cmd::Compare {
            scan,
            s,
            height,
            slope,
            design,
            boundary,
            tolerance,
            diff_out,
        } => {
            let o = s.options();
            let rs = regions(&boundary)?;
            if rs.len() > 1 {
                bail!("compare: bitte genau eine Grenze angeben");
            }
            let g = surface::load(&scan, None, union_bbox(&rs), &o)?;
            let d = match (height, slope, design) {
                (Some(h), None, None) => Design::Height(h),
                (None, Some(v), None) => Design::Slope {
                    x: v[0],
                    y: v[1],
                    z: v[2],
                    percent: v[3],
                    azimuth_deg: v[4],
                },
                (None, None, Some(p)) => {
                    Design::Surface(surface::load(&p, Some(g.spec), None, &o)?)
                }
                _ => bail!("genau eins von --height, --slope oder --design angeben"),
            };
            let (rep, diff) = compare::compare(&g, &d, rs.first().map(|r| &r.polygon), tolerance)?;
            if let Some(p) = diff_out {
                write_grid(&diff, &p, epsg)?;
            }
            print(&rep)?;
        }

        Cmd::Export {
            surface: path,
            s,
            boundary,
            tif,
            asc,
            dxf,
            landxml,
            obj,
            ply,
            csv,
            contours,
            major,
            dxf_mesh,
            stride,
            local,
        } => {
            let rs = regions(&boundary)?;
            let mut g = surface::load(&path, None, union_bbox(&rs), &s.options())?;
            if let Some(m) = union_poly_mask(&rs, g.spec) {
                g.data.iter_mut().zip(m).for_each(|(v, k)| {
                    if !k {
                        *v = f32::NAN
                    }
                });
            }
            let mut written = Vec::new();
            if let Some(p) = &tif {
                geotiff::write(&g, p, Some(epsg), None)?;
                written.push(p.clone());
            }
            if let Some(p) = &asc {
                g.write_asc(p)?;
                written.push(p.clone());
            }
            let needs_mesh = landxml.is_some() || obj.is_some() || ply.is_some() || dxf_mesh;
            let mesh = needs_mesh.then(|| Mesh::from_grid(&g, stride, None));
            let offset = if local {
                let z0 = g.z_stats().map(|s| s.0.floor()).unwrap_or(0.0);
                [g.spec.min_x, g.spec.min_y(), z0]
            } else {
                [0.0; 3]
            };
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "DGM".into());
            if let (Some(p), Some(m)) = (&landxml, &mesh) {
                m.write_landxml(p, &name, Some(epsg as u32))?;
                written.push(p.clone());
            }
            if let (Some(p), Some(m)) = (&obj, &mesh) {
                m.write_obj(p, offset)?;
                written.push(p.clone());
            }
            if let (Some(p), Some(m)) = (&ply, &mesh) {
                m.write_ply(p, offset)?;
                written.push(p.clone());
            }
            let mut n_contours = 0;
            if let Some(p) = &dxf {
                let mut d = Dxf::new();
                let cs = export::contours(&g, contours)?;
                n_contours = cs.len();
                d.contours(&cs, major);
                for r in &rs {
                    let pts: Vec<[f64; 3]> = r
                        .polygon
                        .exterior
                        .iter()
                        .map(|q| [q[0], q[1], g.sample(q[0], q[1]).unwrap_or(0.0)])
                        .collect();
                    d.polyline("GRENZE", &pts, true);
                }
                if let (true, Some(m)) = (dxf_mesh, &mesh) {
                    d.mesh(m, "DGM");
                }
                d.write(p)?;
                written.push(p.clone());
            }
            if let Some(p) = &csv {
                export::write_grid_csv(&g, p, stride)?;
                written.push(p.clone());
            }
            if written.is_empty() {
                bail!("kein Ausgabeformat angegeben (--tif, --asc, --dxf, --landxml, --obj, --ply, --csv)");
            }
            print(&json!({
                "written": written, "cells": g.valid_cells(), "contours": n_contours,
                "mesh": mesh.as_ref().map(|m| json!({"vertices": m.vertices.len(), "faces": m.faces.len()})),
                "offset": offset,
            }))?;
        }
    }
    Ok(())
}

fn union_poly_mask(rs: &[Region], spec: GridSpec) -> Option<Vec<bool>> {
    if rs.is_empty() {
        return None;
    }
    let mut m = vec![false; spec.len()];
    for r in rs {
        for (a, b) in m.iter_mut().zip(spec.mask(Some(&r.polygon))) {
            *a |= b;
        }
    }
    Some(m)
}

trait BBoxJson {
    fn bbox_json(&self) -> Value;
}

impl BBoxJson for Grid {
    fn bbox_json(&self) -> Value {
        let b = self.bbox();
        json!({"min_x": b.min_x, "min_y": b.min_y, "max_x": b.max_x, "max_y": b.max_y})
    }
}
