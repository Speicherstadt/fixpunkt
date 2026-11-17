# Notes for Claude

## Documentation

Die Doku ist in `docs/` (<https://speicherstadt.github.io/fixpunkt/>), gebaut mit Zensical wie firsttris/haben
(`mkdocs.yml`, `requirements-docs.txt`, `.github/workflows/docs.yml`). Vor einer Änderung die betroffenen Seiten
lesen: `architektur.md`, `methode.md`, `aushub.md`, Projektplan `PLAN.md` (Architektur: Abschnitt 5).
Rechenkern: `crates/processing/README.md`. Links in README und Doku zeigen auf die Website, nicht auf `docs/*.md`.
Jede Seite braucht `description:` im Front Matter (docs.yml prüft das). Doku auf Deutsch; Englisch, sobald
Zensical mehrsprachig kann.

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
- die Doku in `docs/` und die README, wenn sich Funktionen, Architektur oder Stand ändern;
- die Screenshots nach einer Änderung am Aussehen: **Actions → Update screenshots → Run workflow**
  (lokal `bun run docs:screenshots`), dann die neuen Bilder ansehen.

## Checks

Grün vor jedem Push, danach die CI ansehen:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bun run lint && bun run typecheck && bun run test && bun run build
bun run e2e    # baut UI + Server und startet Playwright; lokal ohne passenden Browser:
               # PW_CHROMIUM_PATH=/pfad/zu/chrome bun run e2e
zensical build --strict   # wenn docs/ oder mkdocs.yml geändert
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

## Tests (Schichten)

- **Rechenkern** (`crates/processing`): Unit-Tests je Modul gegen bekannte Werte, `tests/cli.rs` End-to-End.
- **Server** (`crates/server`): Unit-Tests je Modul, `tests/api.rs` mit echtem Router, SQLite und Rechenkern;
  Aufträge deterministisch mit `jobs::run_pending`.
- **UI** (`ui/src/**/*.test.ts(x)`): Vitest + Testing Library; Texte (de/en vollständig, alle genutzt, alle
  Server-Fehler übersetzt) und Gestaltungsregeln (`design-rules.test.ts`, wie Kontext).
- **E2E** (`e2e/`): Playwright gegen `target/debug/fixpunkt` mit eingebetteter UI, frisches Datenverzeichnis;
  axe hell/dunkel, Tastatur, Handy. Elemente nach Rolle und Name suchen.
- **Docker** (CI): Image bauen, als Nicht-root starten, Health, UI und erster Admin.

## UI

Gestaltung wie **Kontext** und **Belegt**: Zink-Tokens als CSS-Variablen (`ui/src/styles.css`), Geist,
eigene Komponenten im shadcn/ui-Stil auf Radix mit `cva` (`ui/src/components/ui`), lucide mit einer
Linienstärke, WCAG 2.2 AA. Farben nur als Token, Schrift- und Symbolgrößen auf der Skala.

## Project specifics

- Koordinaten: ETRS89/UTM32 (EPSG:25832), Höhen DHHN2016 (Quasigeoid GCG2016). Testdaten liegen
  bei x ≈ 500 000–512 000, y ≈ 5 400 000 – Rechnungen relativ zu einem Bezugspunkt, sonst
  frisst f32/f64-Auslöschung die Millimeter.
- Erster Anwendungsfall: Aushub von Baugruben aus Drohnenflügen (DJI Matrice 4E, NodeODM).
