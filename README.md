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
| `apps/web` | TanStack Start (TypeScript), Potree / CesiumJS |
| `crates/processing` | Rust-Worker: `las`/`laz`, `nalgebra`, `proj`, `rayon` (+ PDAL-/PotreeConverter-CLI) |
| `infra` | Postgres, S3-kompatibler Storage, NodeODM, Podman/Docker |

Details, Einkaufsliste und Phasen: **[docs/PLAN.md](docs/PLAN.md)**

## Status

🟡 Planung
