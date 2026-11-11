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

### 3.1 Entwicklung

| Gerät | Empfehlung | Preis (ca.) | Wofür |
|---|---|---|---|
| **Mac mini** | Mac mini M6, 16 GB (aktuelles Einstiegsmodell). Alternativ gebraucht M1/M2/M4 | **1.049 €** neu / 400–700 € gebraucht | Xcode: iOS-App bauen, signieren, aufs iPhone bringen. Entwickelt wird weiter unter Linux. |
| Apple Developer Program | – | **99 $/Jahr** | TestFlight, Verteilung an Kumpel |
| GitHub Actions macOS-Runner | optional | nutzungsabhängig | CI-Builds ohne Mac am Schreibtisch |

### 3.2 Handscan (PocketScan-Variante)

| Gerät | Empfehlung | Preis (ca.) | Notiz |
|---|---|---|---|
| **iPhone mit LiDAR** | **iPhone 18 Pro** (neu) – oder gebrauchtes iPhone 15/16/17 Pro | **1.449 €** neu (256 GB) / 500–900 € gebraucht | LiDAR haben alle Pro-Modelle ab 12 Pro. Kein Plus/Standard-Modell! |
| **RTK-Rover** | **Emlid Reach RX2** (Bluetooth, Neigungskompensation, 1 Jahr Korrekturdaten inklusive) | **1.999 €** netto | Bewährt, Bluetooth-NMEA, funktioniert mit jeder NTRIP-Quelle |
| Halterung Phone + Rover | Emlid Scanning Handle **oder** selbst gedruckt (Bambu P2S) | 199 € / ~5 € | Muss starr sein, Antennen-Offset bekannt |
| Lotstab mit Handyhalter | Emlid Survey Pole | 299 € | Passpunkte einmessen |
| Passpunkte (GCP-Marken) | 10–20 Zielmarken / Vermessungsnägel | ~50 € | Genauigkeitsnachweis |
| Korrekturdaten | **SAPOS HEPS** (NTRIP) | **0 €** | In Baden-Württemberg kostenlos |
| **Budget-Alternative Rover** | ArduSimple-Board mit UM980 oder ZED-F9P + Multiband-Antenne + Gehäuse (selbst gedruckt) + BLE-Modul | ~300–600 € (Schätzung) | Mehr Bastelaufwand, kein Support |

Zum Vergleich: Das fertige *Pix4D & Emlid Scanning Kit* kostet 3.900 € netto – das ist praktisch das Produkt, das wir nachbauen.

### 3.3 Drohne

| Gerät | Empfehlung | Preis (ca.) | Notiz |
|---|---|---|---|
| **Drohne** | **DJI Matrice 4E** – RTK eingebaut, mechanischer Verschluss, 20-MP-Weitwinkel + Tele, EU-Klasse **C2** | **~4.900 €** | Standard für Baustellen-Mapping |
| Ersatzakkus | 2× Matrice 4 Intelligent Flight Battery | 2× ~199 € | Ein Akku reicht nicht für einen Arbeitstag |
| Alternative | Gebrauchte **Mavic 3 Enterprise (M3E)** + RTK-Modul | gebraucht günstiger | Vorgänger, für Baustellen weiterhin gut |
| Prüfen | Vorhandene Drohne des Kumpels | – | Reicht sie (RTK? mechanischer Verschluss?) |

### 3.4 Drohnen-Betrieb (rechtlich nötig)

| Posten | Preis (ca.) |
|---|---|
| LBA-Betreiberregistrierung | 20 € (natürliche Person) / 50 € (juristische Person) |
| EU-Kompetenznachweis A1/A3 (Online-Test) | ~25 € |
| EU-Fernpilotenzeugnis A2 (für C2 nahe an Personen) | Kurs + Prüfung, Schätzung 150–300 € |
| Drohnen-Haftpflicht (gewerblich) | Schätzung 100–400 €/Jahr |
| Kennzeichen mit e-ID | wenige € |

### 3.5 Betrieb / Server

| Posten | Preis (ca.) | Notiz |
|---|---|---|
| Server (Hetzner Dedicated oder vorhandener Heimserver) | Schätzung 40–100 €/Monat | Für Web, Postgres, Rust-Worker |
| GPU für ODM (optional) | nach Bedarf | ODM läuft auch auf CPU, nur langsamer |
| S3-Storage | wenige €/Monat pro 100 GB | Rohdaten sind groß (Drohnenflug: mehrere GB) |

### 3.6 Zusammenfassung

| Paket | Inhalt | Summe (ca.) |
|---|---|---|
| **Start (empfohlen)** | Mac mini M6 + Developer Account + iPhone 18 Pro + Reach RX2 + Handle + Lotstab + GCPs | **~5.100 €** |
| Start (sparsam) | Gebrauchter Mac mini + gebrauchtes iPhone Pro + ArduSimple-Rover (gedruckte Halterung) | **~1.500–2.000 €** |
| **+ Drohne** | Matrice 4E + 2 Akkus + Papierkram + Versicherung | **+ ~5.700 €** |
| Laufend | Developer Account, Versicherung, Server | **~100–200 €/Monat** |

Vorhandenes (iPhone, Drohne des Kumpels, Heimserver) senkt die Summen entsprechend.

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
- [ ] Self-hosted beim Kumpel oder zentral gehostet?
- [ ] Open Source oder privat? Geschäftsmodell, falls Produkt
- [ ] iPhone neu (18 Pro) oder gebraucht?

---

## 10. Referenzen

- Vorbild: [SDX-PocketScan](https://makineo.de/produkte/baudokumentation-3d/pocket-scan) · [SDX-Cloud](https://makineo.de/produkte/baudokumentation-3d/infrastruktur-sdx-cloud) · [Datenblatt](https://makineo.de/fileadmin/Home/Produkte/Baudokumentation/SDX-POCKETSCAN/2026/WEB_Makineo_SDX-PocketScan_Datenblatt.pdf)
- Vergleichbar: Pix4Dcatch + Emlid Scanning Kit
- Hardware: [Emlid Reach RX2 (EU-Store)](https://eu.store.emlid.com/en-es/collections/reachrx2-products) · DJI Matrice 4E · Mac mini M6 · iPhone 18 Pro
- Open Source: OpenDroneMap / NodeODM, COLMAP, OpenMVS, PDAL, Open3D, Potree, CesiumJS, RTKLIB, PROJ
- Korrekturdaten: SAPOS Baden-Württemberg (HEPS, kostenlos)
- Drohnenrecht: [dipul.de](https://www.dipul.de), Luftfahrt-Bundesamt
