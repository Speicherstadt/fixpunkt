---
description: >-
  An Fixpunkt entwickeln: Voraussetzungen, bun run dev, Prüfungen wie in der CI, Testschichten von Rechenkern bis
  E2E, Screenshots und Doku erneuern, Release.
---

# Entwicklung und Tests

## Loslegen

Voraussetzungen: Rust (stable), [Bun](https://bun.sh/) in der Version aus `packageManager` (`package.json`).

```bash
bun install
bun run dev    # Server auf :8080, UI mit Hot Reload auf http://localhost:5173 (admin / adminadmin)
```

## Prüfungen

Dieselben wie in der CI, grün vor jedem Push:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bun run lint && bun run typecheck && bun run test && bun run build
bun run e2e
zensical build --strict   # wenn docs/ oder mkdocs.yml geändert
```

`bun run lint` führt nach Biome auch [`checks`](https://github.com/firsttris/workflows#checks) aus (Einstellungen in
`checks.json`): Dateien über 400 Zeilen, Seiten, die `e2e/accessibility.spec.ts` nicht öffnet, und die Texte der
Oberfläche (beide Sprachen vollständig, alle genutzt, keiner im Code) und Kreise von Importen zwischen Modulen. Was es schon vorher gab, steht in `.checks/`;
im Pull Request nennt `checks pr` neue oder gewachsene lange Funktionen.

Der GAEB-Export wird gegen das offizielle Schema geprüft, sobald `GAEB_X31_XSD` darauf zeigt (braucht `xmllint`;
die CI lädt das Schema von gaeb.de, lokal wird die Prüfung sonst übersprungen):

```bash
GAEB_X31_XSD=/pfad/GAEB_DA_XML_31_3.3_2023-01.xsd cargo test -p fixpunkt-server gaeb
```

## Testschichten

| Schicht | Ort | Was |
|---|---|---|
| Rechenkern | `crates/processing` | Unit-Tests je Modul gegen bekannte Werte; `tests/cli.rs` ruft jeden Befehl an einer synthetischen Baustelle auf |
| Server | `crates/server` | Unit-Tests je Modul; `tests/api.rs` mit echtem Router, SQLite und Rechenkern |
| UI | `ui/src/**/*.test.ts(x)` | Vitest und Testing Library; Server-Fehlertexte übersetzt, Gestaltungsregeln |
| E2E | `e2e/` | Playwright gegen die Programmdatei: kompletter Aushub mit Prüfung der m³, Rollen, axe (WCAG 2.2 AA) hell und dunkel, Tastatur, Handy |
| Docker | CI | Image bauen, als Nicht-root starten, Health, UI, erster Admin |

## Screenshots und Doku

```bash
bun run docs:screenshots          # docs/screenshot-*.png, social-preview.png und die Beispiel-PDFs neu
pip install -r requirements-docs.txt
zensical serve                    # Doku unter http://localhost:8000
```

In GitHub: **Actions → Update screenshots → Run workflow** nimmt die Bilder auf und committet, was sich geändert
hat. Das Vorschaubild des PDF-Berichts (`screenshot-pdf.png`) rendert `pdftoppm` aus poppler-utils; fehlt es
lokal, bleibt das alte Bild. Ohne passenden Playwright-Browser: `PW_CHROMIUM_PATH=/pfad/zu/chrome bun run docs:screenshots`. Die Doku baut [Zensical](https://zensical.org/) bei jedem Push auf `main` nach GitHub Pages.

## Release

**Actions → Bump version** hebt die Version in `Cargo.toml` und taggt `vX.Y.Z`. Der Tag startet `release.yml`:
Docker-Image `tristanteu/fixpunkt` (amd64 + arm64), GitHub-Release, statische Programmdateien und `install.sh`.
Voraussetzung: das Secret `DOCKER_PAT` (Docker-Hub-Token) im Repository.
