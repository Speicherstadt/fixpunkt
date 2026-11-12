use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use fixpunkt::geom::Polygon;
use fixpunkt::grid::{Aggregate, Grid, GridSpec};
use fixpunkt::pointcloud::{PointCloud, ReadOptions};
use fixpunkt::volume::{self, Base};
use std::path::PathBuf;

/// Fixpunkt-Engine: Volumen aus Drohnen-Punktwolken (LAS/LAZ, XYZ).
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Args)]
struct Common {
    /// Zellgröße des Höhenrasters in m
    #[arg(long, default_value_t = 0.25)]
    cell: f64,
    /// Abrechnungsgrenze als GeoJSON (gleiches Koordinatensystem wie die Punktwolke)
    #[arg(long)]
    boundary: Option<PathBuf>,
    /// nur Bodenpunkte (LAS-Klasse 2)
    #[arg(long)]
    ground_only: bool,
    #[arg(long, value_enum, default_value_t = Aggregate::Mean)]
    aggregate: Aggregate,
    /// Lücken bis zu so vielen Zellen auffüllen
    #[arg(long, default_value_t = 4)]
    fill: usize,
}

#[derive(Subcommand)]
enum Cmd {
    /// Aushub/Auftrag: Differenz zweier Aufnahmen
    Diff {
        before: PathBuf,
        after: PathBuf,
        #[command(flatten)]
        c: Common,
        /// Differenzraster als ESRI-ASCII-Grid schreiben
        #[arg(long)]
        diff_out: Option<PathBuf>,
    },
    /// Haufwerk gegen Basisfläche (plane | lowest | <Höhe>)
    Stockpile {
        surface: PathBuf,
        #[command(flatten)]
        c: Common,
        #[arg(long, default_value = "plane")]
        base: String,
    },
}

fn load(path: &PathBuf, spec: Option<GridSpec>, c: &Common, mask: Option<&Polygon>) -> Result<Grid> {
    let cloud = PointCloud::read(path, ReadOptions { ground_only: c.ground_only })?;
    let spec = match spec {
        Some(s) => s,
        None => GridSpec::covering(&mask.map(|p| p.bbox()).unwrap_or_else(|| cloud.bbox()), c.cell)?,
    };
    let mut g = Grid::from_points(&cloud, spec, c.aggregate);
    g.fill_gaps(c.fill, None);
    Ok(g)
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Diff { before, after, c, diff_out } => {
            let b = c.boundary.as_deref().map(Polygon::from_geojson_file).transpose()?;
            let spec = match &b {
                Some(p) => GridSpec::covering(&p.bbox(), c.cell)?,
                None => {
                    let o = ReadOptions { ground_only: c.ground_only };
                    let bb = PointCloud::read(&before, o)?.bbox().intersection(&PointCloud::read(&after, o)?.bbox());
                    GridSpec::covering(&bb, c.cell)?
                }
            };
            let g0 = load(&before, Some(spec), &c, None)?;
            let g1 = load(&after, Some(spec), &c, None)?;
            let (rep, diff) = volume::difference(&g0, &g1, b.as_ref())?;
            if let Some(p) = diff_out {
                diff.write_asc(&p)?;
            }
            println!("{}", serde_json::to_string_pretty(&rep)?);
        }
        Cmd::Stockpile { surface, c, base } => {
            let Some(bp) = &c.boundary else { bail!("--boundary ist für stockpile nötig") };
            let poly = Polygon::from_geojson_file(bp)?;
            let base = match base.as_str() {
                "plane" => Base::Plane,
                "lowest" => Base::Lowest,
                h => Base::Height(h.parse().map_err(|_| anyhow::anyhow!("--base: plane, lowest oder Zahl"))?),
            };
            let g = load(&surface, None, &c, Some(&poly))?;
            println!("{}", serde_json::to_string_pretty(&volume::stockpile(&g, &poly, base)?)?);
        }
    }
    Ok(())
}
