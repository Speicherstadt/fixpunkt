<div align="center">

<img src="docs/assets/icon.svg" alt="Fixpunkt-Logo" width="72">

<h1>Fixpunkt: Aushub und Haufwerke aus Drohnenflügen berechnen</h1>

**Selbst gehostete Massenermittlung für Baustellen: Punktwolke oder Höhenmodell hochladen, Abrechnungsgrenze setzen, Mengenbericht in m³.**<br>
Vergleicht Urgelände und Aushub je Abrechnungsbereich, rechnet Bagger und Bewuchs heraus und gibt den Bericht als PDF, CSV oder GeoTIFF weiter.<br>
Eine Programmdatei mit Web-UI, ETRS89/UTM32 und DHHN2016 – ohne Cloud-Abo, auf deinem Server.

[![CI](https://github.com/firsttris/fixpunkt/actions/workflows/ci.yml/badge.svg)](https://github.com/firsttris/fixpunkt/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/firsttris/fixpunkt?logo=github&label=release)](https://github.com/firsttris/fixpunkt/releases/latest)
[![Doku](https://img.shields.io/badge/Doku-firsttris.github.io%2Ffixpunkt-4f46e5?logo=zensical&logoColor=white)](https://firsttris.github.io/fixpunkt/)
[![Docker Pulls](https://img.shields.io/docker/pulls/tristanteu/fixpunkt?logo=docker&logoColor=white)](https://hub.docker.com/r/tristanteu/fixpunkt)
[![Plattformen](https://img.shields.io/badge/platform-amd64%20%7C%20arm64-lightgrey)](https://hub.docker.com/r/tristanteu/fixpunkt/tags)
[![Rust](https://img.shields.io/badge/Rust-axum%20%7C%20SQLite-b7410e?logo=rust&logoColor=white)](https://firsttris.github.io/fixpunkt/architektur.html)
[![EPSG:25832](https://img.shields.io/badge/ETRS89-UTM32%20%C2%B7%20DHHN2016-1f6f5c)](https://firsttris.github.io/fixpunkt/daten.html)

```bash
docker run -d -p 8080:8080 -v fixpunkt:/data tristanteu/fixpunkt
```

[Warum?](#-warum-fixpunkt) •
[Funktionen](#-funktionen) •
[Installation](#-installation) •
[FAQ](#-faq) •
[Dokumentation](https://firsttris.github.io/fixpunkt/) •
[Entwicklung](#️-entwicklung)

<sub>English: self-hosted earthwork volume calculation from drone surveys – upload point clouds (LAS/LAZ) or elevation models (GeoTIFF, e.g. from OpenDroneMap), set billing boundaries, get cut and fill per area. One binary with web UI. The app is in German and English, the documentation in German.</sub>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshot-bericht-dunkel.png">
  <img src="docs/screenshot-bericht.png" alt="Mengenbericht der Baugrube Kirchstraße 12 in Fixpunkt: Abtrag 800,1 m³, Auftrag 0,0 m³, Tabelle je Abrechnungsbereich Grube und Bagger" width="900">
</picture>

</div>

## 💡 Warum Fixpunkt?

Wie viel Boden aus der Baugrube gekommen ist, wird heute mit Lieferscheinen geschätzt oder vom Vermessungsbüro
aufgemessen – Tage später und für jeden Termin neu. Eine RTK-Drohne erfasst die Baugrube in einer Viertelstunde;
was fehlt, ist der Schritt von der Punktwolke zur abrechenbaren Menge.

- **Massen am Tag des Flugs:** Aufnahme hochladen, zwei Stände vergleichen, Abtrag und Auftrag je Abrechnungsbereich.
- **Der Bagger zählt nicht mit:** ein Bodenfilter entfernt Maschinen, Container und Bewuchs und ergänzt das Gelände darunter.
- **Deine Daten bleiben bei dir:** eine Programmdatei mit SQLite, kein Cloud-Abo, kein fremder Dienst.
- **Amtliche Koordinaten:** ETRS89/UTM32 (EPSG:25832), Höhen in DHHN2016 über das Quasigeoid GCG2016.

Läuft auf jedem Linux-Server, NAS oder Mini-PC – mit Docker, Podman oder als systemd-Dienst. Die Fotos der Drohne
wertet [OpenDroneMap](https://opendronemap.org/) aus, auf Wunsch direkt aus Fixpunkt über NodeODM.

## ✨ Funktionen

<table>
<tr>
<td width="50%" valign="top">

### 📐 Massenermittlung
- Aushub und Auftrag aus zwei Aufnahmen, je Bereich
- Haufwerke gegen Basisfläche, Zeitreihe über die Bauphase
- Soll/Ist gegen Sohlhöhe, geneigte Ebene oder Soll-DGM
- Bodenfilter für Bagger, Container und Bewuchs
- Bericht als PDF mit Karte, als CSV oder GeoTIFF; Dokumentation und Plan/Ist als PDF
- Überblick je Baustelle: Aushub je Befliegung, Massenziele und Lagerbestand je Material, Soll/Ist, Zwangspunkte

</td>
<td width="50%" valign="top">

### 🛰️ Daten
- Punktwolken: LAS, LAZ, E57 (Laserscanner wie Leica BLK2GO), XYZ, CSV
- Höhenmodelle: GeoTIFF (OpenDroneMap), ASC
- Flugplanung: Kartierflug als KMZ für DJI Pilot 2
- Drohnenfotos direkt auswerten über NodeODM
- Abrechnungsgrenzen als GeoJSON aus QGIS, mehrere Bereiche je Datei
- Grenzen in Länge/Breite werden nach UTM32 umgerechnet

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 🗺️ Ansicht und Messen
- Höhenbild oder Orthofoto jeder Aufnahme, Differenz oder Abweichung darübergelegt
- Gelände oder Punktwolke in 3D, in Farbe oder nach Intensität, mit Überhöhung
- Messen: Punkt, Strecke, Gefälle, Fläche, Profil
- Abrechnungsbereiche direkt im Browser zeichnen
- Karte aller Baustellen
- Zwangspunkte mit Soll/Ist, Leitungen nachzeichnen, Fotos mit GPS-Position und Blickrichtung, Notizen – als PDF und DXF
- DXF-Pläne als Hintergrund, Höhen aus dem Plan gegen das Gelände
- Bedienbar mit Maus und Tastatur

</td>
<td width="50%" valign="top">

### 👷 Im Team
- Benutzer mit Rollen: Admin, Bearbeiten, Nur lesen
- Anmelden mit Passwort oder Passkey, API-Tokens für Programme
- Live-Status: Prüfung und Berechnung aktualisieren sich von selbst
- Auf Tablet und Handy installierbar, hell und dunkel
- Oberfläche auf Deutsch und Englisch

</td>
</tr>
<tr>
<td width="50%" valign="top">

### 🧮 Exporte und Kommandozeile
- GeoTIFF, DXF mit Höhenlinien, LandXML, CSV, OBJ – aus der Web-UI
- Passpunkte (Helmert) mit Restklaffen in der Web-UI, Koordinaten und Höhen umrechnen
- Bodenfilter, Ausreißer, Profile auf der Kommandozeile
- [Alle Befehle](https://firsttris.github.io/fixpunkt/kommandozeile.html)

</td>
<td width="50%" valign="top">

### 🔒 Selbst gehostet
- Eine Programmdatei mit SQLite, kein Cloud-Abo
- Docker-Image für amd64 und arm64, oder systemd ohne Container
- Login mit argon2id, Schutz gegen CSRF
- [Installation](https://firsttris.github.io/fixpunkt/installation.html)

</td>
</tr>
</table>

<p align="center">
  <img src="docs/screenshot-ansicht.png" alt="Ansicht einer Baugrube in Fixpunkt: Höhenbild, Soll/Ist-Abweichung der Sohle, Abrechnungsbereiche und ein Profil quer durch die Grube" width="49%">
  <img src="docs/screenshot-baustelle.png" alt="Baustelle in Fixpunkt mit drei Aufnahmen vom Urgelände bis zum fertigen Aushub, zwei Abrechnungsbereichen und der Berechnung" width="49%">
</p>

## 🚀 Installation

**Docker oder Podman** (amd64, arm64):

```bash
docker run -d --name fixpunkt -p 8080:8080 -v fixpunkt:/data --restart unless-stopped tristanteu/fixpunkt
```

**Ohne Container** – eine Programmdatei als systemd-Dienst:

```bash
curl -fsSL https://raw.githubusercontent.com/firsttris/fixpunkt/main/scripts/install.sh | sudo sh
```

Dann `http://<server>:8080` öffnen und den ersten Zugang anlegen. Compose, Podman Quadlet, Einstellungen und
Reverse-Proxy: [Installation](https://firsttris.github.io/fixpunkt/installation.html) ·
[Konfiguration](https://firsttris.github.io/fixpunkt/konfiguration.html).

## ❓ FAQ

**Wie genau ist die Massenberechnung aus Drohnendaten?**<br>
Die Rechnung ist bis auf die Rasterweite exakt (geprüft gegen Kegel- und Pyramidenstumpfformel, 0,004 % bei
4 Mio. Punkten). Die Genauigkeit kommt aus den Daten: eine RTK-Drohne misst Höhen auf 2–5 cm. Belastbar ist die
Bilanz; mehr unter [Methode und Genauigkeit](https://firsttris.github.io/fixpunkt/methode.html).

**Brauche ich eine bestimmte Drohne?**<br>
Nein. Fixpunkt rechnet mit Punktwolken und Höhenmodellen, egal woher. Für Ergebnisse ohne Passpunkte braucht die
Drohne RTK (z. B. DJI Matrice 4E mit SAPOS). Auch Laserscanner (LAS/LAZ) und iPhone-LiDAR-Scans passen.

**Rechnet Fixpunkt die Drohnenfotos selbst?**<br>
Über [NodeODM](https://github.com/OpenDroneMap/NodeODM): Fotos in Fixpunkt hochladen, NodeODM rechnet die
Punktwolke, sie erscheint als Aufnahme – in der Zone der Baustelle
([Drohnenfotos auswerten](https://firsttris.github.io/fixpunkt/drohnenfotos.html)). Ohne NodeODM lädt man die
Punktwolke aus WebODM hoch.

**Ist Fixpunkt eine Alternative zu SDX-Cloud, Propeller oder DroneDeploy?**<br>
Für die Massenermittlung von Baugruben und Haufwerken: ja, selbst gehostet und ohne Abo – mit 2D- und 3D-Ansicht,
Messen, PDF-Bericht, Passpunkten, Plänen und Drohnenfotos über NodeODM. Die iPhone-App mit RTK-Rover
folgt ([Projektplan](https://firsttris.github.io/fixpunkt/PLAN.html)).

**Welche Koordinatensysteme werden unterstützt?**<br>
ETRS89/UTM Zone 32 (EPSG:25832) als Standard, andere UTM-Zonen auf der Kommandozeile. Höhen DHHN2016 über GCG2016.

## 🛠️ Entwicklung

Rust-Server (axum, SQLite) mit eingebetteter React-UI (Bun, Vite, TanStack, Paraglide) und Rechenkern als
Bibliothek. Tests auf jeder Schicht, von Rechenergebnissen gegen bekannte Formeln bis Playwright gegen die
fertige Programmdatei.

```bash
bun install
bun run dev    # Server auf :8080, UI mit Hot Reload auf http://localhost:5173 (admin / adminadmin)
```

[Architektur](https://firsttris.github.io/fixpunkt/architektur.html) ·
[Entwicklung und Tests](https://firsttris.github.io/fixpunkt/entwicklung.html) ·
[Projektplan](https://firsttris.github.io/fixpunkt/PLAN.html)

> [!NOTE]
> Fixpunkt ersetzt kein amtliches Aufmaß. Bis die Genauigkeit auf euren Baustellen belegt ist, Ergebnisse vor der
> Abrechnung mit Lieferscheinen oder einem Aufmaß plausibilisieren.
