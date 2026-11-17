// rust-embed braucht `ui/dist` schon beim Kompilieren. Ist die UI nicht gebaut (z. B. `cargo test`
// ohne Bun), wird ein leerer Ordner angelegt, damit der Server trotzdem baut.
// Vorbild: firsttris/haul crates/haul/build.rs
fn main() {
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/dist");
    if !dist.exists() {
        let _ = std::fs::create_dir_all(&dist);
    }
    println!("cargo:rerun-if-changed=../../ui/dist");
    // sqlx::migrate! bettet die Migrationen beim Kompilieren ein; neue Dateien bemerkt Cargo sonst nicht
    println!("cargo:rerun-if-changed=migrations");
}
