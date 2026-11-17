# Notes for Claude

## Documentation

Projektplan, Hardware, Kosten und Phasen: `docs/PLAN.md` – vor jeder Änderung die betroffenen
Abschnitte lesen (Architektur: Abschnitt 5). Rechenkern: `crates/processing/README.md`.

## Reference implementations

Technik und Abläufe wie in den anderen Projekten, nicht aus dem Gedächtnis:

- `firsttris/workflows`: `tech-stack.md` (Technologiewahl), wiederverwendbare Workflows, Vorlagen
- `firsttris/haul`: Rust-Server mit eingebetteter UI (`crates/haul/src/ui.rs`, `build.rs`, `auth.rs`,
  `events.rs`, `db.rs`), `docker/Dockerfile`, `release.yml` (statische musl-Binärdateien), `scripts/install.sh`
- `firsttris/snapraid-ui`: UI mit shadcn/ui, Tailwind 4, Paraglide JS (`apps/web`)

Flach klonen (`git clone --depth 1`) in den Scratchpad und die Quelldatei im Commit nennen, wo sie
ein Detail entscheidet.

## With every change

- Tests für das Geänderte (Unit-Tests im Modul, End-to-End in `crates/processing/tests/cli.rs`);
  Rechenergebnisse gegen unabhängig bekannte Werte prüfen (Formel, numerische Kontrolle).
- `docs/PLAN.md` und die READMEs, wenn sich Funktionen, Architektur oder Stand ändern.

## Checks

Grün vor jedem Push, danach die CI ansehen:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
# sobald es ui/ gibt: bun run typecheck && bun run test && bun run build
```

## Working rules

- Commits direkt auf `main` sind erlaubt.
- Sprache: Doku, Kommentare und Commits auf Deutsch; die Oberfläche auf Deutsch, Englisch später
  (Paraglide, `ui/messages/{de,en}.json`).

## Code style

Technik nach `tech-stack.md` in firsttris/workflows. Abweichungen und warum:

- **Backend in Rust statt TypeScript:** Punktwolken mit Millionen Punkten, Rasterrechnung,
  Bodenfilter – Performance zählt. axum/tokio, SQLite/sqlx, `tracing`, `anyhow` im Binary.
- **SQLite trotz mehrerer Nutzer:** ein Server, eine Programmdatei, keine weiteren Dienste
  (wie haul). PostgreSQL erst, wenn mehrere Dienste dieselbe Datenbank brauchen.
- **Single-Page-App statt TanStack Start:** der Rust-Server liefert die gebaute UI per
  `rust-embed` aus; Start bräuchte einen eigenen JavaScript-Server.
- **Login im Rust-Server** (argon2id, Sitzungs-Cookie, CSRF, Passkeys über `webauthn-rs`) statt
  Better Auth, das ein TypeScript-Backend voraussetzt.

Clean Code, funktional: klare Namen, kleine Funktionen, Abhängigkeiten übergeben, keine
Duplikate; reine Funktionen, Seiteneffekte an den Rändern, Fehler als Werte. Eine Datei macht eine
Sache: ab ~400 Zeilen teilen, Funktionen ab ~80 Zeilen.

## Project specifics

- Koordinaten: ETRS89/UTM32 (EPSG:25832), Höhen DHHN2016 (Quasigeoid GCG2016). Testdaten liegen
  bei x ≈ 500 000–512 000, y ≈ 5 400 000 – Rechnungen relativ zu einem Bezugspunkt, sonst
  frisst f32/f64-Auslöschung die Millimeter.
- Erster Anwendungsfall: Aushub von Baugruben aus Drohnenflügen (DJI Matrice 4E, NodeODM).
