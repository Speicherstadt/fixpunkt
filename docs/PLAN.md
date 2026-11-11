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

**Realistische Einschätzung, Stand 11.11.2026.** Neuware, gute Qualität, aber nur das, was wir wirklich brauchen.
Wichtig ist die Trennung: Was brauchen **wir zum Entwickeln** – und was braucht die **Firma des Kumpels für den Einsatz** (und hat sie teils schon)?
Preise: Apple inkl. MwSt., Emlid und netcup netto.

### 3.1 Was wir zum Entwickeln brauchen

| Gerät | Empfehlung | Preis (ca.) | Notiz |
|---|---|---|---|
| **Mac** | **Mac mini M5 Pro, 24 GB, 512 GB** (Basis-M5-Pro) | **1.999 €** | Xcode läuft damit flüssig. Mehr RAM braucht es nicht, weil die Photogrammetrie auf dem Server läuft. **Entfällt, wenn die iOS-Entwicklerin schon einen aktuellen Mac (Apple Silicon) hat.** |
| **iPhone mit LiDAR** | **iPhone 18 Pro, 256 GB** | **1.449 €** | Ein Testgerät reicht. **Entfällt, wenn schon ein iPhone Pro ab 12 Pro vorhanden ist** (alle Pro-Modelle haben LiDAR). |
| **RTK-Rover** | **Emlid Reach RX2** | **1.999 € netto** | Brauchen wir zum Entwickeln und Testen. Geht danach an den Kumpel oder er kauft einen eigenen. |
| Halterung Phone + Rover | Selbst gedruckt (Bambu P2S) | ~5 € Filament | Statt Emlid Scanning Handle (199 €) |
| Passpunkte (GCP-Marken) | 10–20 Zielmarken / Vermessungsnägel | ~50 € | Für das Testfeld und den Genauigkeitsnachweis |
| Lotstab | Einfacher Vermessungs-Lotstab mit Handyhalter | ~50–300 € | Nur zum Einmessen der Passpunkte; Emlid Survey Pole 299 € netto |
| Apple Developer Program | – | **99 $/Jahr** | Pflicht für TestFlight |
| Korrekturdaten | SAPOS HEPS | **0 €** | In Baden-Württemberg kostenlos |

**Einmalig für uns: ca. 2.100–5.600 €** – je nachdem, ob Mac und iPhone schon da sind.
(Alles neu: 1.999 + 1.449 + ~1.999 + ~100–350 ≈ 5.550–5.800 €. Mac und iPhone vorhanden: nur Rover + Kleinkram ≈ 2.100–2.350 €.)

### 3.2 Drohne – liegt bei der Firma des Kumpels

Laut Kumpel hat seine Firma **schon Drohnen** und **einen Mitarbeiter mit Drohnenführerschein**. Dann gilt:

- **Wir brauchen keinen eigenen Drohnenführerschein.** Der Führerschein (A1/A3 bzw. A2) gehört zum Piloten, die Registrierung (e-ID) und die Versicherung zum Betreiber – das ist dann die Firma.
- **Wir kaufen keine Drohne.** Fixpunkt verarbeitet nur die Fotos, die deren Pilot fliegt.
- **Haftung und Versicherung** für die Flüge liegen beim Betreiber, nicht bei uns.

Zu klären: **Welche Drohne hat die Firma?**

| Fall | Was es bedeutet |
|---|---|
| Drohne mit RTK (z. B. Matrice 4E, Mavic 3 Enterprise RTK) | Ideal: cm-genau ohne viele Passpunkte |
| Gute Drohne ohne RTK (z. B. Mavic 3 / Air 3) | Funktioniert auch – dann mit 5–10 eingemessenen Passpunkten pro Baustelle (Rover haben wir ja) |
| Nur Consumer-Mini | Für Doku okay, für Massenermittlung eher grenzwertig |

Falls die Firma später eine Mapping-Drohne kauft: **DJI Matrice 4E (EU) ca. 4.900 €** + 2 Ersatzakkus à ~199 € – das ist dann deren Anschaffung, nicht unsere.

### 3.3 Server – netcup, stufenweise

| Phase | Tarif | Preis netto | Warum |
|---|---|---|---|
| **Entwicklung** (ca. erste 3 Monate) | **netcup RS 2000 G12.5** – 8 dedizierte vCores, 16 GB RAM, 256 GB NVMe | **40,70 €/Monat** | Reicht für Web, API, Datenbank, Handscans und Test-Drohnenflüge mit einigen hundert Fotos |
| **Echtbetrieb** (Kumpel fliegt regelmäßig) | **netcup RS 4000 G12.5** – 12 vCores, **32 GB RAM**, 512 GB NVMe | **77,83 €/Monat** | Typische Baustellenflüge (einige hundert bis gut tausend Fotos) laufen flüssig |
| Speicher (Archiv) | netcup Storage Space B (500 GB) → später C (1 TB) | 8 → 15 €/Monat | Rohdaten und alte Projekte |
| Upgrade nur bei Bedarf | netcup RS 8000 G12.5 – 64 GB RAM | 148,42 €/Monat | Erst bei regelmäßig sehr großen Flügen oder mehreren Kunden |
| Domain | Subdomain von teufel-it.de | 0 € | z. B. `fixpunkt.teufel-it.de` |

netcup lässt sich innerhalb der Produktlinie hochstufen – wir müssen also nicht im Voraus für Reserven zahlen.

### 3.4 Laufende Kosten

| | Entwicklung | Echtbetrieb |
|---|---|---|
| netcup Server | 40,70 € | 77,83 € |
| netcup Speicher | 8,00 € | 15,00 € |
| **Monatlich** | **~49 €** | **~93 €** |
| Apple Developer (jährlich) | 99 $ | 99 $ |
| **Pro Jahr gesamt** | **~680 €** | **~1.210 €** |

Nicht bei uns (Firma des Kumpels): Drohnen-Haftpflicht, Führerscheine, Mobilfunkdaten fürs Baustellen-iPhone.

### 3.5 Gesamtbild

| | Wir (Entwicklung) | Firma des Kumpels (Einsatz) |
|---|---|---|
| Einmalig | ca. 2.100–5.600 € (Mac, iPhone, Rover, Kleinkram) | iPhone Pro + eigener Rover (~3.450 €), falls sie unseren nicht übernehmen; Drohne vorhanden |
| Laufend | ca. 49–93 €/Monat Server + 99 $/Jahr Apple | Versicherung, Führerscheine (vorhanden), Mobilfunk |

Zum Vergleich: Allein das fertige *Pix4D & Emlid Scanning Kit* kostet 3.900 € netto, Sodex/Makineo verlangen für Hardware + Cloud deutlich mehr (Angebot auf Anfrage).

### 3.6 Wo kaufen / beantragen

| Was | Link | Notiz |
|---|---|---|
| Mac mini | [Apple Store – Mac mini kaufen](https://www.apple.com/de/shop/buy-mac/mac-mini) | M5 Pro konfigurieren |
| iPhone 18 Pro / Pro Max | [Apple Store – iPhone kaufen](https://www.apple.com/de/shop/buy-iphone) | Pro-Modell (LiDAR) |
| Apple Developer Program | [developer.apple.com/programs](https://developer.apple.com/programs/) | 99 $/Jahr |
| Emlid Reach RX2, Scanning Handle, Lotstab | [Emlid EU-Store – Reach RX2](https://eu.store.emlid.com/en-es/collections/reachrx2-products) | Preise netto |
| DJI Matrice 4E (nur falls die Firma eine kauft) | [Galaxus – DJI Matrice 4E (EU)](https://www.galaxus.de/en/s1/product/dji-matrice-4e-eu-1219-g-20-mpx-drone-54319549) | EU-Version (C2) nehmen |
| DJI-Enterprise-Fachhändler | [Händlerübersicht drohnen.de](https://www.drohnen.de/22293/dji-enterprise-haendler/) | Alternative mit Beratung, Schulung, Service |
| netcup Root-Server | [netcup Root-Server](https://www.netcup.com/de/server/root-server) | RS 2000 (Entwicklung) → RS 4000 (Betrieb) |
| netcup Speicher | [netcup Storage Space](https://netcup.com/en/server/storage-space) | Tarif B (500 GB) → C (1 TB) |
| SAPOS-Korrekturdaten | [Zentrale Stelle SAPOS – Gebühren & Registrierung](https://zentrale-stelle-sapos.de/en/fees-registration/) | in BW kostenlos, Registrierung nötig |
| LBA-Betreiberregistrierung | [uas-betrieb.bund.de – Registrierung als Betreiber](https://uas-betrieb.bund.de/homepage/en/applications-administrative-services/registration-as-an-operator) | e-ID für die Drohne |
| Geozonen prüfen | [dipul.de](https://www.dipul.de) | vor jedem Flug |

---

## 4. Rechtliches (Kurzfassung)

- **Keine Zertifizierung nötig** für das Erfassen, Messen und Massenermitteln als Dienstleistung oder Software.
- **Amtliche Vermessung** (Kataster, Grenzen, Gebäudeeinmessung) dürfen nur ÖbVI/Vermessungsämter. Fixpunkt nie als „amtliche Vermessung“ bezeichnen.
- **Drohne:** Registrierung, Kompetenznachweis und Versicherung liegen beim Betreiber – also der Firma des Kumpels, die bereits einen Piloten mit Führerschein hat (s. 3.2). Über Wohngrundstücken Zustimmung des Eigentümers (§21h LuftVO). Geozonen auf [dipul.de](https://www.dipul.de) prüfen (z. B. Baden Airpark).
- **Datenschutz:** Unbeteiligte Personen und Nachbargrundstücke vermeiden bzw. unkenntlich machen.
- **Eigene Hardware verkaufen** → CE, Funkanlagenrichtlinie, ElektroG/WEEE, Batterierecht. Daher: unsere Software auf gekaufter Hardware.

---

## 5. Architektur

```
 iOS-App (Swift)                 Drohne (Firma des Kumpels)
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
- [ ] Klären: Hat die iOS-Entwicklerin schon einen Mac / ein iPhone Pro? Sonst Mac mini M5 Pro + iPhone 18 Pro kaufen
- [ ] Apple Developer Account
- [ ] Reach RX2 bestellen, Halterung drucken
- [ ] SAPOS-Zugang (BW) beantragen
- [ ] Testfeld anlegen (Hof/Garten) mit 5–10 eingemessenen Passpunkten
- [ ] Mit der Firma des Kumpels klären: Welche Drohne (RTK?), wer fliegt, Testflug-Termin

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
- Hardware: [Emlid Reach RX2 (EU-Store)](https://eu.store.emlid.com/en-es/collections/reachrx2-products) · DJI Matrice 4E · Mac mini M5 Pro · iPhone 18 Pro · netcup
- Open Source: OpenDroneMap / NodeODM, COLMAP, OpenMVS, PDAL, Open3D, Potree, CesiumJS, RTKLIB, PROJ
- Korrekturdaten: SAPOS Baden-Württemberg (HEPS, kostenlos)
- Drohnenrecht: [dipul.de](https://www.dipul.de), Luftfahrt-Bundesamt
