---
name: checks
description: "Prüfungen von Fixpunkt ausführen (cargo fmt, clippy, cargo test, Biome und checks, Typen, Vitest, Build, Playwright, Doku). Vor jedem Push, im Review und wenn Rust, Bun, ein Browser oder ein Werkzeug für die Tests fehlt."
---

# Prüfungen

Vor jedem Push grün, danach die CI prüfen. Dieselben Befehle wie in `.github/workflows/ci.yml`; Voraussetzungen:
Rust (stable), Bun in der Version aus `packageManager` (`package.json`).

```sh
bun install --frozen-lockfile
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bun run lint           # Biome, dann checks (checks.json): Dateigröße, Seiten im axe-Test, Texte de/en, Importkreise
bun run typecheck      # ui und e2e
bun run test           # Vitest in ui/
bun run build          # UI nach ui/dist; der Server bettet sie ein
bun run e2e            # baut UI und Server, startet Playwright (setup, desktop, phone)
shellcheck scripts/install.sh   # wenn install.sh geändert
```

Wenn sich `docs/`, `mkdocs.yml`, `overrides/` oder `scripts/doku-*` geändert haben
(`pip install -r requirements-docs.txt`):

```sh
python -m unittest discover -s scripts/doku-aenderungsdatum
python -m unittest discover -s scripts/doku-hinweise
zensical build --strict
test -z "$(grep -L '^description:' docs/*.md)"   # jede Seite mit description im Front Matter
```

## Was übersprungen wird

- **GAEB-Schema:** Der Test des GAEB-Exports gegen das offizielle Schema X31 läuft nur mit `GAEB_X31_XSD` und
  `xmllint`, sonst wird er übersprungen. Die CI lädt das Schema von gaeb.de; lokal so:
  `GAEB_X31_XSD=/pfad/GAEB_DA_XML_31_3.3_2023-01.xsd cargo test -p fixpunkt-server gaeb`. Ohne Schema im Pull
  Request als übersprungen nennen.
- **Docker-Image:** baut und startet nur die CI (Job „Docker-Image“, wenn sich Dockerfile, Abhängigkeiten oder
  Cargo-Dateien ändern).
- **Browser:** Ohne passenden Playwright-Browser `PW_CHROMIUM_PATH=/pfad/zu/chrome bun run e2e`. Gezielt:
  `bun --bun playwright test e2e/<spec>.ts --project=setup --project=desktop` (nach `bun run build` und
  `cargo build -p fixpunkt-server`); die volle Suite läuft in der CI.
- **Entwürfe:** Die CI baut Entwurfs-Pull-Requests nicht; „Ready for review“ startet sie. Die Regeln am Pull Request
  (Titel, `Umgesetzt`/`Refs`, neue Abhängigkeiten, lange Funktionen) prüft die Werkbank trotzdem, vor dem Push
  siehe Skill `beitrag`.

## Testumgebung

E2E läuft auf Port 28080 (Server) und 28081 (NodeODM-Nachbau) mit frischem Datenverzeichnis `.e2e-data/`
(`playwright.config.ts`); der Entwicklungsserver (`bun run dev`) nutzt 8080 und 5173. Laufende Prozesse nicht
stoppen; ist ein Port belegt, die Nutzerin oder den Nutzer fragen. Was übersprungen wurde, gehört mit Grund in die
Beschreibung des Pull Requests, nie als bestanden.
