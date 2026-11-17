# Fixpunkt

Georeferenzierte 3D-Baustellendokumentation – per iPhone-LiDAR + RTK-Rover und per Drohne, verarbeitet in einer eigenen, selbst gehosteten Cloud.

> **Fix** – der Moment, in dem der RTK-Rover zentimetergenau ist.
> **Festpunkt** – der Referenzpunkt, auf den sich in der Vermessung alles bezieht.

## Was Fixpunkt macht

1. **Erfassen** – Graben, Haufwerk oder Fläche mit dem iPhone (LiDAR + RTK) scannen oder per Drohne nach Flugplan befliegen.
2. **Hochladen** – Rohdaten (Punktwolke/Bilder + RTK-Trajektorie) gehen in die Fixpunkt-Cloud.
3. **Verarbeiten** – automatische Georeferenzierung in ETRS89/UTM32 (EPSG:25832), Höhen in DHHN2016.
4. **Nutzen** – im Browser ansehen, messen, Volumen berechnen, als LAS/LAZ, DXF, LandXML exportieren.

Vorbild: [SDX-PocketScan / SDX-Cloud](https://makineo.de/produkte/baudokumentation-3d/pocket-scan) von Sodex Innovations (vertrieben von Makineo).

## Aufbau

| Bereich | Technik |
|---|---|
| `apps/ios` | Swift, SwiftUI, ARKit (LiDAR), CoreBluetooth + External Accessory (MFi), NTRIP-Client |
| `crates/server` | Rust: tokio, axum, SQLite (sqlx), SSE – liefert die Web-UI eingebettet aus (**eine Programmdatei**) |
| `crates/processing` | Rust-Rechenkern: LAS/LAZ, GeoTIFF, Bodenfilter, Helmert, UTM32/GCG2016, Volumen, Exporte |
| `ui` | Bun, Vite, TypeScript, React, TanStack Router + Query, shadcn/ui-Stil wie Kontext (Radix, Tailwind 4, Zink, Geist), Paraglide JS |
| `e2e` | Playwright gegen die echte Programmdatei, axe (WCAG 2.2 AA) |
| `docker` | Image in drei Stufen (UI → Binärdatei → Debian slim); NodeODM als eigener Container |

Details, Einkaufsliste und Phasen: **[docs/PLAN.md](docs/PLAN.md)**

## Starten

```bash
# Docker (amd64/arm64), Daten im Volume /data
docker run -d -p 8080:8080 -v fixpunkt:/data tristanteu/fixpunkt

# ohne Docker: eine Programmdatei als systemd-Dienst
curl -fsSL https://raw.githubusercontent.com/firsttris/fixpunkt/main/scripts/install.sh | sudo sh
```

Beim ersten Aufruf legt man den Admin-Zugang an. Einstellungen über Umgebungsvariablen:
`FIXPUNKT_DATA_DIR`, `FIXPUNKT_LISTEN`, `FIXPUNKT_USER`/`FIXPUNKT_PASSWORD` (erster Admin),
`FIXPUNKT_MAX_JOBS`, `FIXPUNKT_MAX_UPLOAD_GB`, `FIXPUNKT_SECURE_COOKIES`.

Dieselbe Datei rechnet auch auf der Kommandozeile: `fixpunkt diff vorher.laz nachher.laz --boundary grube.geojson`.

## Entwickeln

```bash
bun install
bun run dev        # Server auf :8080, UI mit Hot Reload auf http://localhost:5173 (admin / adminadmin)
```

Prüfungen (wie die CI): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `bun run lint`, `bun run typecheck`, `bun run test`, `bun run e2e`.

## Status

🟡 Phase 1 · **Server, Web-UI und Rechenkern lauffähig**, Aushubberechnung von Upload bis Mengenbericht · Rechenkern in `crates/processing` (Volumen, Soll/Ist, Bodenfilter, Georeferenzierung, Messen, Exporte – [Details](crates/processing/README.md))
