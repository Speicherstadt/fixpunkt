<div align="center">

<img src="docs/assets/icon.svg" alt="Fixpunkt logo" width="72">

<h1>Fixpunkt: earthwork volumes from drone surveys</h1>

[Deutsch](README.md) · **English**

**Self-hosted volume calculation for construction sites: upload a point cloud or elevation model, draw a billing boundary, get a quantity report in m³.**<br>
Compares original terrain and excavation per billing area, filters out excavators and vegetation, and hands over the report as PDF, CSV or GeoTIFF.<br>
One binary with a web UI, ETRS89/UTM32 and DHHN2016 – no cloud subscription, on your own server.

[![CI](https://github.com/Speicherstadt/fixpunkt/actions/workflows/ci.yml/badge.svg)](https://github.com/Speicherstadt/fixpunkt/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Speicherstadt/fixpunkt?logo=github&label=release)](https://github.com/Speicherstadt/fixpunkt/releases/latest)
[![Docs](https://img.shields.io/badge/docs-speicherstadt.github.io%2Ffixpunkt-4f46e5?logo=zensical&logoColor=white)](https://speicherstadt.github.io/fixpunkt/)
[![Docker Pulls](https://img.shields.io/docker/pulls/tristanteu/fixpunkt?logo=docker&logoColor=white)](https://hub.docker.com/r/tristanteu/fixpunkt)
[![Docker Image](https://img.shields.io/docker/v/tristanteu/fixpunkt?sort=semver&logo=docker&logoColor=white&label=image)](https://hub.docker.com/r/tristanteu/fixpunkt/tags)
[![Platforms](https://img.shields.io/badge/platform-amd64%20%7C%20arm64-lightgrey)](https://hub.docker.com/r/tristanteu/fixpunkt/tags)
[![Rust](https://img.shields.io/badge/Rust-axum%20%7C%20SQLite-b7410e?logo=rust&logoColor=white)](https://speicherstadt.github.io/fixpunkt/architektur.html)
[![EPSG:25832](https://img.shields.io/badge/ETRS89-UTM32%20%C2%B7%20DHHN2016-1f6f5c)](https://speicherstadt.github.io/fixpunkt/daten.html)

```bash
docker run -d -p 8080:8080 -v fixpunkt:/data tristanteu/fixpunkt
```

[Why?](#-why-fixpunkt) •
[Features](#-features) •
[Installation](#-installation) •
[FAQ](#-faq) •
[Documentation](https://speicherstadt.github.io/fixpunkt/) •
[Development](#️-development)

<sub>The app is available in German and English; the documentation website is in German for now.</sub>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshot-bericht-dunkel.png">
  <img src="docs/screenshot-bericht.png" alt="Fixpunkt quantity report of a construction pit: cut 800.1 m³, fill 0.0 m³, table per billing area pit and excavator" width="900">
</picture>

</div>

## 💡 Why Fixpunkt?

How much soil came out of a construction pit is usually estimated from delivery notes or measured by a surveying
office – days later, and again for every date. An RTK drone captures the pit in a quarter of an hour; what's
missing is the step from point cloud to a billable quantity.

- **Quantities on the day of the flight:** upload a capture, compare two states, cut and fill per billing area.
- **The excavator doesn't count:** a ground filter removes machines, containers and vegetation and fills in the
  terrain below.
- **Your data stays with you:** one binary with SQLite, no cloud subscription, no third-party service.
- **Official coordinates:** ETRS89/UTM32 (EPSG:25832), heights in DHHN2016 via the GCG2016 quasigeoid.

Runs on any Linux server, NAS or mini PC – with Docker, Podman or as a systemd service. The drone photos are
processed by [OpenDroneMap](https://opendronemap.org/), directly from Fixpunkt via NodeODM if you like.

## ✨ Features

<table>
<tr>
<td width="50%" valign="top">

### 📐 Volumes
- Cut and fill from two captures, per area
- Stockpiles against a base surface, time series across the project
- Plan vs. actual against a formation level, an inclined plane or a design surface
- Ground filter for excavators, containers and vegetation
- Report as PDF with map, as CSV or GeoTIFF; documentation, plan vs. actual and a custom site report as PDF
- Billing: statement per period with item number and unit price, in bank and loose m³, tonnes and truck loads;
  delivery tickets imported as CSV and checked against the measurement
- Site overview, arranged by each user: excavation per flight, quantity targets, stock and history per material,
  plan vs. actual, constraint points

</td>
<td width="50%" valign="top">

### 🛰️ Data
- Point clouds: LAS, LAZ, E57 (laser scanners), XYZ, CSV, PTS
- Elevation models: GeoTIFF (OpenDroneMap), ASC
- Flight planning: mapping flight as KMZ for DJI Pilot 2
- Process drone photos directly via NodeODM
- Billing boundaries as GeoJSON from QGIS, several areas per file
- Boundaries in latitude/longitude are converted to UTM32

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 🗺️ View and measure
- Elevation image or orthophoto of each capture, with difference or deviation on top
- Terrain or point cloud in 3D, in colour or by intensity, with vertical exaggeration
- Measure: point, distance, slope, area, profile
- Draw billing areas directly in the browser
- Map of all sites
- Constraint points with plan vs. actual, traced utility lines, photos with GPS position and viewing direction,
  notes – as PDF and DXF
- DXF plans as background, heights from the plan against the terrain
- Works with mouse and keyboard

</td>
<td width="50%" valign="top">

### 👷 As a team
- Users with roles: admin, edit, read-only
- Sign in with password or passkey, API tokens for programs
- Live status: checks and calculations update by themselves
- Installable on tablet and phone, light and dark
- Interface in German and English

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 🧮 Exports and command line
- GeoTIFF, DXF with contour lines, LandXML, CSV, OBJ, orthophoto as GeoTIFF – from the web UI
- Control points (Helmert) with residuals in the web UI, transform coordinates and heights
- Ground filter, outliers, profiles on the command line
- [All commands](https://speicherstadt.github.io/fixpunkt/kommandozeile.html) (German)

</td>
<td width="50%" valign="top">

### 🔒 Self-hosted
- One binary with SQLite, no cloud subscription
- Docker image for amd64 and arm64, or systemd without a container
- Login with argon2id, CSRF protection
- [Installation](https://speicherstadt.github.io/fixpunkt/installation.html) (German)

</td>
</tr>
</table>

<p align="center">
  <img src="docs/screenshot-ansicht.png" alt="View of a construction pit in Fixpunkt: elevation image, plan vs. actual deviation of the pit floor, billing areas and a profile across the pit" width="49%">
  <img src="docs/screenshot-baustelle.png" alt="Construction site in Fixpunkt with three captures from original terrain to finished excavation, two billing areas and the calculation" width="49%">
</p>

## 🚀 Installation

**Docker or Podman** (amd64, arm64):

```bash
docker run -d --name fixpunkt -p 8080:8080 -v fixpunkt:/data --restart unless-stopped tristanteu/fixpunkt
```

**Without a container** – one binary as a systemd service:

```bash
curl -fsSL https://raw.githubusercontent.com/Speicherstadt/fixpunkt/main/scripts/install.sh | sudo sh
```

Then open `http://<server>:8080` and create the first account. Compose, Podman Quadlet, settings and reverse proxy:
[Installation](https://speicherstadt.github.io/fixpunkt/installation.html) ·
[Configuration](https://speicherstadt.github.io/fixpunkt/konfiguration.html) (German).

## ❓ FAQ

**How accurate is the volume calculation from drone data?**<br>
The calculation is exact down to the grid size (checked against the cone and frustum formulas, 0.004 % with
4 million points). The accuracy comes from the data: an RTK drone measures heights to 2–5 cm. What holds is the
balance; more under [Method and accuracy](https://speicherstadt.github.io/fixpunkt/methode.html) (German).

**Do I need a specific drone?**<br>
No. Fixpunkt works with point clouds and elevation models from any source. For results without ground control
points the drone needs RTK (e.g. DJI Matrice 4E with SAPOS correction data). Laser scans (LAS/LAZ, E57) work too.

**Does Fixpunkt process the drone photos itself?**<br>
Via [NodeODM](https://github.com/OpenDroneMap/NodeODM): upload the photos in Fixpunkt, NodeODM computes the point
cloud, and it appears as a capture – in the UTM zone of the site
([Processing drone photos](https://speicherstadt.github.io/fixpunkt/drohnenfotos.html), German). Without NodeODM,
upload the point cloud from WebODM.

**What can Fixpunkt do, and what's next?**<br>
Everything at a glance in the [feature overview](https://speicherstadt.github.io/fixpunkt/funktionen.html); the
iPhone app with RTK rover for utility trenches and field testing on real sites are on the
[roadmap](https://speicherstadt.github.io/fixpunkt/roadmap.html), the hardware under
[devices](https://speicherstadt.github.io/fixpunkt/geraete.html) (all German).

**Can I bill with Fixpunkt?**<br>
Yes. In Germany there is no certification for the software or method; the civil code (BGB) and VOB/B require a
verifiable, traceable quantity calculation. What it takes – agreement with the client, drone regulations, legal
metrology, data protection – is described under
[Billing and law](https://speicherstadt.github.io/fixpunkt/recht.html) (German).

**Which coordinate systems are supported?**<br>
ETRS89/UTM zone 32 (EPSG:25832) by default, other UTM zones on the command line. Heights DHHN2016 via GCG2016.

## 🛠️ Development

Rust server (axum, SQLite) with an embedded React UI (Bun, Vite, TanStack, Paraglide) and the processing core as a
library. Tests on every layer, from results against known formulas to Playwright against the finished binary.

```bash
bun install
bun run dev    # server on :8080, UI with hot reload on http://localhost:5173 (admin / adminadmin)
```

[Architecture](https://speicherstadt.github.io/fixpunkt/architektur.html) ·
[Development and tests](https://speicherstadt.github.io/fixpunkt/entwicklung.html) ·
[Roadmap](https://speicherstadt.github.io/fixpunkt/roadmap.html) (German)

> [!NOTE]
> Fixpunkt does not replace an official survey. Until the accuracy is proven on your sites, check results against
> delivery notes or a survey before billing.
