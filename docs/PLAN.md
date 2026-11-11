# Fixpunkt – Projektplan

Stand: 11.11.2026 · Preise sind Richtwerte (Recherche Nov. 2026), vor dem Kauf prüfen.

---

## 1. Worum geht's

Wir bauen eine selbst gehostete Alternative zu **SDX-PocketScan + SDX-Cloud** von Sodex Innovations
(österreichische Firma, seit 2021; Vertrieb in DE über Makineo, Leica-Partner).

Das Vorbild besteht aus zwei Teilen:

1. **Erfassung**
   - *PocketScan*: iPhone/iPad mit eingebautem LiDAR + aufgesteckter RTK-GNSS-Empfänger
     (7 mm + 1 ppm horizontal, 14 mm + 1 ppm vertikal). Scannen auf der Baustelle, ohne Vermessungs-Know-how.
     Optional Bodenpasspunkte (GCPs), die mit einem Lotstab eingemessen werden.
   - *Drohne*: Befliegung mit hunderten bis tausenden überlappenden Fotos (Photogrammetrie).
   - (Bei Sodex zusätzlich: Scanner am Bagger – „KTEG AutoDoku“. Für uns erstmal nicht relevant.)
2. **Cloud** – Upload, automatische Verarbeitung in wenigen Minuten, dann im Browser:
   messen, Volumen/Massen ermitteln, mit Planung vergleichen, dokumentieren, exportieren.

**Erster Nutzer:** Kumpel aus dem Bau – Gräben, Leitungen, Haufwerke, kleine Flächen.

---

## 2. Funktionsumfang (was wir nachbauen)

Abgeleitet aus dem, was SDX-Cloud/PocketScan anbieten:

### Messen
- [ ] Einzelpunkt mit Koordinate (Rechts-/Hochwert, Höhe)
- [ ] **Längen** / Strecken (3D und horizontal), Polylinien
- [ ] Höhenunterschiede, Gefälle (z. B. Leitungsgefälle in %)
- [ ] **Flächen** (horizontal und Oberfläche)
- [ ] Querprofile / Längsschnitte durch einen Graben

### Massenermittlung
- [ ] **Volumen von Haufwerken** (gegen interpolierte Basisfläche)
- [ ] **Aushub-/Auftragsmassen** (Differenz zweier Aufnahmen: Urgelände ↔ Aushub)
- [ ] Volumen innerhalb frei gezeichneter Bereiche (Geofences)
- [ ] Fortlaufende Aktualisierung über die Bauphase (Zeitreihe)

### Vergleichen
- [ ] Plan/Ist-Vergleich mit hinterlegten DXF-Plänen
- [ ] Zwei Zeitpunkte vergleichen (Abweichungs-Heatmap)
- [ ] Soll-Höhe / Sollfläche gegen Scan

### Dokumentieren
- [ ] **Zwangspunkte** digitalisieren: Abzweige, Hausanschlüsse, Schächte, Leitungsverlauf
- [ ] Fotos georeferenziert inkl. Blickrichtung, Notizen dazu
- [ ] Berichte (PDF) und Dashboards pro Baustelle
- [ ] Kartenansicht aller Baustellen/Scans

### Ansichten
- [ ] 2D (Draufsicht, Orthofoto), 3D (Punktwolke, Mesh), Karte
- [ ] Einfärbung nach RGB, Höhe, Intensität, Abweichung

### Export
- [ ] LAS/LAZ (Punktwolke), DXF (Linien/Punkte), LandXML (Flächen), CSV (Punkte), OBJ/PLY (Mesh), GeoTIFF (Orthofoto/DGM)

---

## 3. Hardware & Kosten

**Grundsatz:** Nur Neuware, Apple-Geräte direkt bei Apple, in guter Ausstattung – kein Warten auf Builds, kein Gebastel.
Preise: Apple inkl. MwSt., Emlid und netcup netto (Teufel-IT ist vorsteuerabzugsberechtigt).

### 3.1 Entwicklung

| Gerät | Empfehlung | Preis (ca.) | Wofür |
|---|---|---|---|
| **Mac mini** | **Mac mini M5 Pro, 18-Core CPU / 20-Core GPU, 64 GB, 1 TB SSD** | Einstieg M5 Pro ab 1.999 €; diese Ausstattung **ca. 3.000–3.700 €** (genau im Apple-Konfigurator) | Schnelle Xcode-Builds, Simulator, ARKit-Debugging; 64 GB reichen auch, um Punktwolken und ODM lokal zu testen |
| Alternative Mac mini | M5 Pro 15-Core / 16-Core GPU, 24 GB, 512 GB | ab 1.999 € | Reicht für die iOS-App allein auch flüssig |
| Apple Developer Program | – | **99 $/Jahr** | TestFlight, Verteilung an Kumpel |

### 3.2 Handscan (PocketScan-Variante)

| Gerät | Empfehlung | Preis (ca.) | Notiz |
|---|---|---|---|
| **iPhone mit LiDAR** | **iPhone 18 Pro Max, 512 GB** | ab 1.599 € (256 GB), 512 GB mit Aufpreis | Großes Display und Akku für die Baustelle, LiDAR, viel Speicher für Scans |
| Alternative | iPhone 18 Pro | ab 1.449 € (256 GB) | Kompakter, gleicher LiDAR |
| **RTK-Rover** | **Emlid Reach RX2** (Bluetooth, Neigungskompensation, Multiband) | **1.999 €** netto | Bewährt, funktioniert mit jeder NTRIP-Quelle (SAPOS) |
| Halterung Phone + Rover | Emlid Scanning Handle | 199 € netto | Starr, bekannter Antennen-Offset |
| Lotstab mit Handyhalter | Emlid Survey Pole | 299 € netto | Passpunkte einmessen |
| Passpunkte (GCP-Marken) | 10–20 Zielmarken / Vermessungsnägel | ~50 € | Genauigkeitsnachweis |
| Korrekturdaten | **SAPOS HEPS** (NTRIP) | **0 €** | In Baden-Württemberg kostenlos |

Zum Vergleich: Das fertige *Pix4D & Emlid Scanning Kit* kostet 3.900 € netto – das ist praktisch das Produkt, das wir nachbauen.

### 3.3 Drohne

| Gerät | Empfehlung | Preis (ca.) | Notiz |
|---|---|---|---|
| **Drohne** | **DJI Matrice 4E (EU)** – RTK eingebaut, mechanischer Verschluss, 20-MP-Weitwinkel + Tele, EU-Klasse **C2** | **~4.900 €** | Standard für Baustellen-Mapping; inkl. 1 Jahr DJI Care Enterprise Plus |
| Ersatzakkus | 2× Matrice 4 Intelligent Flight Battery | 2× ~199 € | Ein Akku reicht nicht für einen Arbeitstag |
| Prüfen | Vorhandene Drohne des Kumpels | – | Reicht sie (RTK? mechanischer Verschluss?) |

### 3.4 Drohnen-Betrieb (rechtlich nötig)

| Posten | Preis (ca.) |
|---|---|
| LBA-Betreiberregistrierung | 20 € (natürliche Person) / 50 € (juristische Person) |
| EU-Kompetenznachweis A1/A3 (Online-Test) | ~25 € |
| EU-Fernpilotenzeugnis A2 (für C2 nahe an Personen) | Kurs + Prüfung, Schätzung 150–300 € |
| Drohnen-Haftpflicht (gewerblich) | Schätzung 100–400 €/Jahr – Angebote einholen |
| Kennzeichen mit e-ID | wenige € |

### 3.5 Cloud / Hosting – netcup

Die Fixpunkt-Cloud läuft von Anfang an bei **netcup** (deutsches Rechenzentrum, DSGVO).

**Stufenweise:** Klein anfangen. Die Rechenlast steckt fast nur in der Drohnen-Photogrammetrie (ODM).
Die läuft am Anfang auf dem **Mac mini (64 GB)** als Worker – der netcup-Server muss dann nur Web, API, Datenbank und die leichte Handscan-Verarbeitung stemmen.

| Stufe | Tarif | Preis netto | Wann |
|---|---|---|---|
| **Start** | **netcup RS 1000 G12.5** – 4 dedizierte vCores (AMD EPYC), 8 GB RAM, 128 GB NVMe | **21,73 €/Monat** (12 Monate Laufzeit) | Web, API, Postgres, Handscans; ODM-Jobs laufen auf dem Mac mini |
| Noch kleiner | netcup VPS (geteilte Kerne) | ca. 5–10 €/Monat | Reicht für reines Web/API, wird bei Punktwolken zäh |
| Ausbau | netcup RS 2000 G12.5 – 8 vCores, 16 GB, 256 GB | 40,70 €/Monat | Wenn ODM auf den Server soll (kleine/mittlere Flüge) |
| Viel Drohne | netcup RS 4000 G12.5 – 12 vCores, 32 GB, 512 GB | 77,83 €/Monat | Viele Flüge mit tausenden Bildern |
| **Speicher** | **netcup Storage Space B – 500 GB** | **8,00 €/Monat** | Start; später Tarif C (1 TB) für 15 €/Monat |
| Domain | Subdomain von teufel-it.de (Cloudflare) | 0 € | z. B. `fixpunkt.teufel-it.de` |

Faustregel ODM: Ein paar hundert Fotos brauchen ~16 GB RAM, tausende Fotos 32–64 GB. 24 Monate Laufzeit sind bei netcup nochmals günstiger.

### 3.6 Einmalige Anschaffung

| Paket | Inhalt | Summe (ca.) |
|---|---|---|
| **Handscan + Entwicklung** | Mac mini M5 Pro (64 GB/1 TB) + iPhone 18 Pro Max 512 GB + Reach RX2 + Handle + Lotstab + GCPs | **~7.500–8.500 €** |
| **Drohne** | Matrice 4E + 2 Akkus + Registrierung + Führerscheine | **~5.500–5.800 €** |
| **Gesamt** | | **~13.000–14.300 €** |

Schätzung, gemischt brutto (Apple, DJI) und netto (Emlid). Vorhandene Drohne des Kumpels senkt die Summe um den Drohnenblock.

### 3.7 Laufende Kosten

**Monatlich (Start)**

| Posten | Kosten netto |
|---|---|
| netcup RS 1000 G12.5 | 21,73 € |
| netcup Storage Space B (500 GB) | 8,00 € |
| **Summe monatlich** | **~30 €** |

Mobilfunkdaten fürs iPhone auf der Baustelle, falls nicht schon vorhanden: zusätzlich.

**Jährlich**

| Posten | Kosten |
|---|---|
| netcup (12 × ~30 €) | ~357 € |
| Apple Developer Program | 99 $ |
| Drohnen-Haftpflicht (gewerblich) | Schätzung 100–400 € – Angebote einholen |
| DJI Care Enterprise Plus (Verlängerung ab Jahr 2, optional) | laut Händler |
| AppleCare+ für Mac/iPhone (optional) | laut Apple |
| **Summe pro Jahr (ohne Optionen)** | **~550–850 €** |

**Alle 5 Jahre**

| Posten | Kosten |
|---|---|
| EU-Kompetenznachweis A1/A3 erneuern | ~25 € |
| A2-Fernpilotenzeugnis erneuern | Schätzung |

Ausbau später: RS 2000 + 1 TB Speicher ≈ 56 €/Monat, RS 4000 + 1 TB ≈ 93 €/Monat.

### 3.8 Wo kaufen / beantragen

| Was | Link | Notiz |
|---|---|---|
| Mac mini | [Apple Store – Mac mini kaufen](https://www.apple.com/de/shop/buy-mac/mac-mini) | M5 Pro konfigurieren |
| iPhone 18 Pro / Pro Max | [Apple Store – iPhone kaufen](https://www.apple.com/de/shop/buy-iphone) | Pro-Modell (LiDAR) |
| Apple Developer Program | [developer.apple.com/programs](https://developer.apple.com/programs/) | 99 $/Jahr |
| Emlid Reach RX2, Scanning Handle, Lotstab | [Emlid EU-Store – Reach RX2](https://eu.store.emlid.com/en-es/collections/reachrx2-products) | Preise netto |
| DJI Matrice 4E | [Galaxus – DJI Matrice 4E (EU)](https://www.galaxus.de/en/s1/product/dji-matrice-4e-eu-1219-g-20-mpx-drone-54319549) | EU-Version (C2) nehmen |
| DJI-Enterprise-Fachhändler | [Händlerübersicht drohnen.de](https://www.drohnen.de/22293/dji-enterprise-haendler/) | Alternative mit Beratung, Schulung, Service |
| netcup Root-Server | [netcup Root-Server](https://www.netcup.com/de/server/root-server) | Start: RS 1000 G12.5 |
| netcup Speicher | [netcup Storage Space](https://netcup.com/en/server/storage-space) | Start: Tarif B (500 GB) |
| SAPOS-Korrekturdaten | [Zentrale Stelle SAPOS – Gebühren & Registrierung](https://zentrale-stelle-sapos.de/en/fees-registration/) | in BW kostenlos, Registrierung nötig |
| LBA-Betreiberregistrierung | [uas-betrieb.bund.de – Registrierung als Betreiber](https://uas-betrieb.bund.de/homepage/en/applications-administrative-services/registration-as-an-operator) | e-ID für die Drohne |
| Geozonen prüfen | [dipul.de](https://www.dipul.de) | vor jedem Flug |

---

## 4. Rechtliches (Kurzfassung)

- **Keine Zertifizierung nötig** für das Erfassen, Messen und Massenermitteln als Dienstleistung oder Software.
- **Amtliche Vermessung** (Kataster, Grenzen, Gebäudeeinmessung) dürfen nur ÖbVI/Vermessungsämter. Fixpunkt nie als „amtliche Vermessung“ bezeichnen.
- **Drohne:** Registrierung, Kompetenznachweis, Versicherung (s. 3.4). Über Wohngrundstücken Zustimmung des Eigentümers (§21h LuftVO). Geozonen auf [dipul.de](https://www.dipul.de) prüfen (z. B. Baden Airpark).
- **Datenschutz:** Unbeteiligte Personen und Nachbargrundstücke vermeiden bzw. unkenntlich machen.
- **Eigene Hardware verkaufen** → CE, Funkanlagenrichtlinie, ElektroG/WEEE, Batterierecht. Daher: unsere Software auf gekaufter Hardware.

---

## 5. Architektur

```
 iOS-App (Swift)                 Drohne (Matrice 4E)
 LiDAR + ARKit-Posen             Fotos mit RTK-Position (EXIF/XMP)
 RTK über Bluetooth (Reach RX2)  Flugplan (Pilot 2 → später WPML/KMZ, Cloud API)
          │                               │
          └───────────────┬───────────────┘
                          ▼
           Web / API  (TanStack Start, TypeScript)
           Baustellen · Scans · Upload · Auth · Berichte
                          │  Job-Queue (Postgres)
                          ▼
           Processing-Worker  (Rust)
           Georeferenzierung · Längen · Volumen · Vergleich · Export
           ruft auf: PDAL · PotreeConverter / py3dtiles · NodeODM (Drohne)
                          │
                          ▼
           Storage (S3)  →  Viewer im Browser (Potree / CesiumJS)
```

**Grundprinzip:** Die Cloud nimmt „Punkte bzw. Bilder + RTK-Trajektorie“ entgegen – egal woher.
iPhone, Drohne, später Android-PWA oder ein eigener LiDAR-Scanner sind austauschbare Erfassungsquellen.

### 5.1 iOS-App (Swift) – Verantwortung: iOS-Entwicklerin
- SwiftUI-Oberfläche: Baustelle wählen → RTK-Status (Fix/Float/kein Fix) → Scannen → Hochladen
- ARKit: `sceneDepth`, Scene Reconstruction (`ARMeshAnchor`), Kamerapose pro Frame
- CoreBluetooth: NMEA (GGA, RMC) vom Rover lesen
- NTRIP-Client: SAPOS-Korrekturen holen und an den Rover weiterreichen (oder Rover nutzt eigene Verbindung)
- Synchrone Aufzeichnung: ARKit-Pose + RTK-Position + Zeitstempel
- Passpunkt-Modus: GCP mit Lotstab einmessen
- Offline-Puffer bei Funkloch, Upload im Hintergrund
- Optional: Object Capture (`PhotogrammetrySession`) für texturierte Meshes
- Reine Logik (NMEA, NTRIP, Datenformat) als Swift-Package – auch unter Linux testbar

### 5.2 Web / API (TypeScript) – Verantwortung: Tristan
- TanStack Start, Postgres (Drizzle o. ä.), S3-kompatibler Storage
- Mandanten → Baustellen → Aufnahmen (Scans/Flüge) → Ergebnisse
- Upload großer Dateien (multipart, resumable)
- Viewer: Potree für Punktwolken **oder** CesiumJS + 3D Tiles (Karte + 3D in einem)
- Messwerkzeuge im Viewer (Punkt, Linie, Fläche, Profil, Volumen-Polygon)
- DXF-Import für Pläne, Berichte als PDF

### 5.3 Processing (Rust) – Verantwortung: Tristan
| Aufgabe | Crates / Tools |
|---|---|
| Punktwolken lesen/schreiben | `las`, `laz-rs` |
| Georeferenzierung (Helmert/Umeyama, Ausgleich) | `nalgebra` |
| Koordinaten ETRS89/UTM32 (EPSG:25832), Höhen DHHN2016 über GCG2016 | `proj` |
| Nachbarschaftssuche, Parallelisierung | `kiddo`, `rayon` |
| Bodenfilter, DGM, Raster | anfangs `pdal pipeline` (CLI), später eigener Code |
| Volumen | DGM-Differenz / TIN (`delaunator`) |
| Kacheln für den Browser | PotreeConverter bzw. py3dtiles (CLI) |
| Drohnen-Photogrammetrie | NodeODM per HTTP-API |

### 5.4 Drohnen-Pipeline
1. Mapping-Mission in **DJI Pilot 2** (Fläche zeichnen, Überlappung 75–80 %, ggf. Schrägaufnahmen)
2. Fotos hochladen → **NodeODM** → Orthofoto, DSM/DTM, Punktwolke, Mesh
3. Ergebnis landet im selben Koordinatensystem wie Handscans
4. Später: Flugplan in Fixpunkt zeichnen → **WPML/KMZ** exportieren
5. Noch später: **DJI Cloud API** (MQTT, selbst gehostet) – Mission pushen, Fotos automatisch empfangen

---

## 6. Phasen

### Phase 0 – Setup (1–2 Wochen)
- [ ] Repo-Struktur: `apps/ios`, `apps/web`, `crates/processing`, `infra`, `docs`
- [ ] Mac mini + Apple Developer Account
- [ ] iPhone Pro + Reach RX2 bestellen
- [ ] SAPOS-Zugang (BW) beantragen
- [ ] Testfeld anlegen (Hof/Garten) mit 5–10 eingemessenen Passpunkten
- [ ] Drohnen-Papierkram starten (LBA, A1/A3, Versicherung, ggf. A2)

### Phase 1 – Backend + Viewer (3–4 Wochen)
- [ ] Datenmodell: Mandant, Baustelle, Aufnahme, Rohdaten, Ergebnis, Passpunkt, Messung
- [ ] Upload + Job-Queue
- [ ] Rust-Worker: LAS/LAZ einlesen, transformieren, Browser-Kacheln erzeugen
- [ ] Viewer mit Punkt-, Längen-, Flächen-, Höhenmessung
- [ ] Mit öffentlichen Testdatensätzen arbeiten, bis eigene Daten da sind

### Phase 2 – Drohne + Massenermittlung (2–3 Wochen)
- [ ] NodeODM anbinden
- [ ] Orthofoto, DGM, Punktwolke im Viewer
- [ ] Volumen Haufwerk gegen Basisfläche
- [ ] Aushubmasse: Differenz zweier Aufnahmen
- [ ] Erster echter Flug beim Kumpel

### Phase 3 – iOS-App (4–6 Wochen, parallel ab Phase 1)
- [ ] BLE-Verbindung Reach RX2, NMEA-Parser, RTK-Status
- [ ] NTRIP-Client (SAPOS)
- [ ] ARKit-LiDAR-Scan mit synchroner Pose-/RTK-Aufzeichnung
- [ ] Hebelarm-Kalibrierung (Antenne ↔ Kamera)
- [ ] GCP-Modus mit Lotstab
- [ ] Upload, Offline-Puffer
- [ ] TestFlight an den Kumpel

### Phase 4 – Genauigkeit (laufend, größtes technisches Risiko)
- [ ] Helmert/Umeyama über die Gesamttrajektorie
- [ ] Abschnittsweiser Ausgleich gegen ARKit-Drift bei langen Gräben
- [ ] Passpunkte einbeziehen
- [ ] Genauigkeitsbericht pro Aufnahme (Residuen an Kontrollpunkten)
- [ ] Ziel: ≤ 2–3 cm bei Gräben bis 30–50 m

### Phase 5 – Ausbau
- [ ] Zwangspunkte / Leitungsdokumentation
- [ ] Plan/Ist-Vergleich mit DXF, Abweichungs-Heatmap
- [ ] Zeitreihen, Dashboards, PDF-Berichte
- [ ] Exporte DXF, LandXML, CSV, GeoTIFF
- [ ] Flugplanung in Fixpunkt (WPML), danach DJI Cloud API
- [ ] Android-PWA (Photogrammetrie + Web Bluetooth)
- [ ] Optional: eigener Handscanner (Livox Mid-360 + FAST-LIO2)

**Realistisch:** Erste nutzbare Version für den Kumpel in **ca. 3 Monaten** nebenher.

---

## 7. Machbarkeit

| Teil | Einschätzung |
|---|---|
| Web-UI, API, Viewer | 9/10 |
| Drohnen-Pipeline (ODM) | 8/10 |
| Messen, Volumen, Export | 8/10 |
| iOS-App (mit iOS-Entwicklerin im Team) | 9/10 |
| Georeferenzierung / Drift bei langen Scans | 6/10 |
| **Gesamt: Kumpel nutzt es für Doku & Massen** | **9/10** |
| Grabenaufmaß nachweisbar 1–2 cm | 7/10 |
| Als Produkt an fremde Firmen verkaufen | 5/10 |

---

## 8. Risiken

| Risiko | Gegenmaßnahme |
|---|---|
| ARKit-Drift bei langen Gräben | Abschnittsweiser Ausgleich, Passpunkte, Scanlänge begrenzen |
| Kein RTK-Fix (Abschattung, Funkloch) | Fix-Status erzwingen, Offline-Puffer, Post-Processing (RTKLIB) |
| Hebelarm Antenne ↔ Kamera ungenau | Starre Halterung, Kalibrierroutine im Testfeld |
| Höhenbezug falsch (ellipsoidisch vs. DHHN2016) | GCG2016 einbinden, gegen amtlichen Festpunkt prüfen |
| Große Datenmengen (Drohne: tausende Fotos) | Resumable Upload, Verarbeitung serverseitig, Speicherkonzept |
| Bedienung für Laien | Ein-Knopf-Workflow, klare Fehlermeldungen, früh mit dem Kumpel testen |

---

## 9. Offene Fragen

- [ ] Welche Drohne hat der Kumpel bereits – RTK? mechanischer Verschluss?
- [ ] Welche Use-Cases zuerst: Haufwerke, Gräben/Leitungen oder Flächen?
- [ ] Welche Ausgabeformate braucht der Bauleiter / das Büro konkret (DXF? LandXML? REB?)
- [ ] Open Source oder privat? Geschäftsmodell, falls Produkt
- [ ] iPhone 18 Pro oder Pro Max?

---

## 10. Referenzen

- Vorbild: [SDX-PocketScan](https://makineo.de/produkte/baudokumentation-3d/pocket-scan) · [SDX-Cloud](https://makineo.de/produkte/baudokumentation-3d/infrastruktur-sdx-cloud) · [Datenblatt](https://makineo.de/fileadmin/Home/Produkte/Baudokumentation/SDX-POCKETSCAN/2026/WEB_Makineo_SDX-PocketScan_Datenblatt.pdf)
- Vergleichbar: Pix4Dcatch + Emlid Scanning Kit
- Hardware: [Emlid Reach RX2 (EU-Store)](https://eu.store.emlid.com/en-es/collections/reachrx2-products) · DJI Matrice 4E · Mac mini M5 Pro · iPhone 18 Pro Max · netcup
- Open Source: OpenDroneMap / NodeODM, COLMAP, OpenMVS, PDAL, Open3D, Potree, CesiumJS, RTKLIB, PROJ
- Korrekturdaten: SAPOS Baden-Württemberg (HEPS, kostenlos)
- Drohnenrecht: [dipul.de](https://www.dipul.de), Luftfahrt-Bundesamt
