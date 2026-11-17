//! `fixpunkt` – startet den Server; `fixpunkt <befehl> …` führt einen Rechenbefehl aus
//! (`diff`, `stockpile`, `export` …, siehe `fixpunkt help`).

use tracing_subscriber::EnvFilter;

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let first = args.get(1).and_then(|a| a.to_str()).unwrap_or("serve");
    match first {
        "serve" => std::process::exit(match run_server() {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("Fehler: {e:#}");
                1
            }
        }),
        "--version" | "-V" => println!("fixpunkt {}", env!("CARGO_PKG_VERSION")),
        _ => std::process::exit(fixpunkt::cli::main_from(args)),
    }
}

fn run_server() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")),
        )
        .init();
    let cfg = fixpunkt_server::config::Config::from_env()?;
    tokio::runtime::Runtime::new()?.block_on(fixpunkt_server::serve(cfg))
}
