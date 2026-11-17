//! Rechenkern als eigenständige Kommandozeile (`fixpunkt-engine`); im Server-Binary `fixpunkt`
//! sind dieselben Befehle eingebaut.

fn main() {
    std::process::exit(fixpunkt::cli::main_from(std::env::args_os()));
}
