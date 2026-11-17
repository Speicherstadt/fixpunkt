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

**Erster Nutzer:** Kumpel aus dem Bau. **Erster Anwendungsfall: Aushub von Baugruben berechnen** (per Drohne).
Später: Leitungsgräben, Haufwerke, kleine Flächen (per iPhone + RTK-Rover).

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

**Korrektur:** Die Firma des Kumpels hat **keine Drohne** und nach jetzigem Stand auch **niemanden mit Drohnenführerschein**.
Gleichzeitig ist der erste Anwendungsfall klar: **Aushub von Baugruben berechnen.** Baugruben sind breit und offen – dafür ist die Drohne das richtige Werkzeug.
Deshalb gilt jetzt: **Drohne zuerst.** iPhone + RTK-Rover (Sodex-Prinzip) kommt erst später für schmale Leitungsgräben.

Preise: Apple und DJI inkl. MwSt., Emlid und netcup netto.

### 3.1 Was man wofür braucht

| Gerät | Was es macht | Wann wir es brauchen |
|---|---|---|
| **Drohne mit RTK** | Fliegt die Baugrube automatisch ab, macht hunderte Fotos mit cm-genauer Position. Daraus rechnet die Cloud ein 3D-Modell. Zwei Flüge (vorher/nachher) → Differenz = **Aushub in m³**. | **Sofort** – Hauptwerkzeug für Baugruben, Haufwerke, Flächen |
| **RTK-Rover** (z. B. Emlid Reach RX2) | Liefert die Position eines Punkts auf 1–2 cm. (a) Am iPhone: verortet den LiDAR-Scan. (b) Auf dem Lotstab: misst Einzelpunkte und Passpunkte ein. | **Später** – für schmale Leitungsgräben (iPhone-Scan) oder als Kontrolle der Drohnengenauigkeit. Mit einer RTK-Drohne nicht nötig. |
| **iPhone Pro (LiDAR)** | Scannt aus nächster Nähe, auch in schmalen Gräben mit Verbau | **Später** – zusammen mit dem Rover |

### 3.2 Drohne – Empfehlung DJI Matrice 4E

| | |
|---|---|
| **Modell** | **DJI Matrice 4E (EU-Version, Klasse C2)** |
| **Preis** | ca. **4.500–4.900 €** inkl. Fernsteuerung DJI RC Plus 2 |
| **Warum** | **RTK eingebaut** → mit SAPOS cm-genau, **keine Passpunkte, kein Rover** nötig. 4/3-Zoll-Kamera mit **mechanischem Verschluss**. Mapping-Missionen in **DJI Pilot 2**: Baugrube auf der Karte einzeichnen, Drohne fliegt das Raster selbst ab (inkl. Schrägaufnahmen für Böschungen). Eine Baugrube: ca. 5–15 Minuten. |
| **Warum nicht günstiger** | Eine Kameradrohne ohne RTK (z. B. DJI Mavic 4 Pro, ~2.000 €) bräuchte bei jedem Flug 5–10 eingemessene Passpunkte → zusätzlich Rover (~2.000 €) und jedes Mal 15–30 Minuten mehr Arbeit. Kaum billiger, deutlich umständlicher. |

**Einmalige Kosten Drohne (Firma des Kumpels):**

| Posten | ca. |
|---|---|
| DJI Matrice 4E inkl. RC Plus 2 | 4.500–4.900 € |
| 2 Ersatzakkus | ~400 € |
| DJI Care Enterprise (optional, Bruchschutz) | ~400–600 € |
| Kompetenznachweis **A1/A3** (Online-Test beim LBA) + **A2** (Zusatzprüfung, wegen Klasse C2 und Flügen in bebautem Gebiet) | ~50–300 € |
| Betreiberregistrierung (e-ID beim LBA) | ~20–50 € |
| **Summe einmalig** | **ca. 5.400–6.300 €** |
| Drohnen-Haftpflicht gewerblich (Pflicht) | ~100–300 €/Jahr |

**Führerschein:** Ein Mitarbeiter der Firma (z. B. der Kumpel selbst) macht A1/A3 online und danach A2. Aufwand: ein paar Abende Lernen plus Prüfung.
**Betreiber** (Registrierung, Versicherung, Haftung) ist, wer die Drohne kauft und fliegt – empfohlen: die Firma, nicht wir.

### 3.2a Alternative: Leica BLK2GO (Handlaserscanner)

Vom Kumpel ins Spiel gebracht. **Kein Rover, sondern ein Laserscanner:** Man läuft damit um die Baugrube, er erzeugt per SLAM eine sehr dichte Punktwolke.
Er sieht die **Form**, weiß aber **nicht, wo auf der Erde** er ist (kein GNSS).

**Begriffe:**
- **Rover** (z. B. Emlid Reach RX2, auch das blaue Sodex-Modul) = genaues GNSS → **Lage** eines Punkts in Landeskoordinaten, 1–2 cm.
- **Scanner / Kamera** (iPhone-LiDAR, Drohnenkamera, BLK2GO) = **Form** der Umgebung als 3D-Modell.
- Jede Variante braucht beides: Die Drohne Matrice 4E hat RTK eingebaut, iPhone und BLK2GO brauchen einen Rover dazu.

**Preis (Recherche Nov. 2026, nicht direkt im Shop geprüft – verbindliches Angebot über Makineo/Leica-Partner einholen):**

| | Preis |
|---|---|
| BLK2GO, Leica-Shop EU | Liste ca. 27.990 € netto, Aktion ca. 19.995 € netto |
| BLK2GO, Leica-Shop USA | 37.500 $ |
| BLK2GO PULSE (neueres Modell, mit iPhone gekoppelt), Leica-Shop EU | Liste ca. 14.995 € netto, Aktion ca. 7.999 € netto (passt nicht zum US-Preis von 42.800 $ – prüfen!) |
| Sehr billige Händlerangebote (5–10 Tsd. $) | vermutlich gebraucht oder unseriös |

**Vergleich für Aushub von Baugruben:**

| | **Leica BLK2GO** | **Drohne Matrice 4E** | **iPhone + Reach RX2** |
|---|---|---|---|
| Genauigkeit | **am besten** (ca. 1–2 cm, sehr dicht) | gut (ca. 2–5 cm) | ordentlich (Ziel 2–3 cm) |
| Führerschein / Versicherung / Wetter | **nicht nötig** | A1/A3 + A2, Haftpflicht, Flugwetter | nicht nötig |
| Landeskoordinaten | nur mit Passpunkten → **Rover nötig** | **direkt** (RTK eingebaut) | **direkt** (Rover am iPhone) |
| Große Baugrube | Rand ablaufen, 10–20 min | **am schnellsten**, 5–15 min | mühsam |
| Schmale Gräben, Verbau, unter Bäumen | **sehr gut** | schlecht | gut |
| Kosten einmalig | **ca. 20.000–28.000 € netto + ~2.000 € Rover** (+ ggf. Software-Abo) | ca. 5.400–6.300 € inkl. Führerschein | ca. 3.500 € |
| Auswertung | Leica-Software (Cyclone) oder Fixpunkt (E57/LAS-Import) | Fixpunkt | Fixpunkt (App muss erst gebaut werden) |

**Genauigkeit in m³ (Beispiel 20 × 15 × 3 m ≈ 900 m³):** Drohne ± ca. 9 m³ (~1 %), BLK2GO ± ca. 3 m³ (~0,3 %). Für Aushubabrechnung reicht beides.

**Ablauf mit BLK2GO:** 3–4 Zielmarken an festen Stellen am Grubenrand (z. B. Nägel im Bordstein), einmal mit dem Rover auf dem Lotstab einmessen → jeder Scan (vorher/nachher) wird darauf ausgerichtet → Differenz = Aushub.

**Einschätzung:**
- Firma kommt **günstig** an einen BLK2GO (Aktion, Miete, Teststellung, gebraucht vom Händler) → **nehmen**: kein Führerschein, sehr genau, auch für Gräben.
- Muss **zum vollen Preis** neu gekauft werden und es geht hauptsächlich um Baugruben → **Drohne** hat das bessere Preis-Leistungs-Verhältnis.
- **Am besten vor dem Kauf beides auf einer echten Baugrube testen** (Makineo-Teststellung, Vorführflug beim DJI-Fachhändler).

**Für Fixpunkt:** E57/LAS-Import und Ausrichtung auf Passpunkte bzw. Vorscan einbauen – passt ins bestehende Konzept („Punkte egal woher“).

### 3.3 Was wir zum Entwickeln brauchen

| Gerät | Empfehlung | Preis (ca.) | Notiz |
|---|---|---|---|
| **Server** | netcup (s. 3.4) | ab 49 €/Monat | Reicht für die komplette Drohnen-Pipeline |
| **Testdaten** | Öffentliche Datensätze von OpenDroneMap | 0 € | Bis der erste eigene Flug da ist |
| Mac | Mac mini M5 Pro, 24 GB, 512 GB | 1.999 € | **Erst für die iOS-App (später).** Entfällt, wenn die iOS-Entwicklerin schon einen aktuellen Mac hat. |
| iPhone mit LiDAR | iPhone 18 Pro, 256 GB | 1.449 € | **Erst für die iOS-App (später).** Entfällt, wenn ein iPhone Pro ab 12 Pro vorhanden ist. |
| RTK-Rover | Emlid Reach RX2 | 1.999 € netto | **Vorerst nicht.** Erst für den iPhone-Scan oder zur Genauigkeitskontrolle. |
| Halterung Phone + Rover | Selbst gedruckt (Bambu P2S) | ~5 € Filament | Statt Emlid Scanning Handle (229 $ netto) |
| Apple Developer Program | – | 99 $/Jahr | Erst für TestFlight (später) |
| Korrekturdaten | SAPOS HEPS | 0 € | In Baden-Württemberg kostenlos – auch die Drohne nutzt das |

**Einmalig für uns in der ersten Phase: ca. 0 €** – nur Serverkosten.
Kommt später die iPhone-Variante dazu: Rover + Kleinkram ≈ 2.100–2.350 €, ggf. Mac und iPhone (alles neu ≈ 5.550–5.800 €).

#### Zum Vergleich: Was Sodex beim PocketScan verbaut

Laut [Datenblatt](https://makineo.de/fileadmin/Home/Produkte/Baudokumentation/SDX-POCKETSCAN/2026/WEB_Makineo_SDX-PocketScan_Datenblatt.pdf) **kein Emlid**, sondern ein flaches Modul (95 × 64 × 12,5 mm, 115 g, „gl“-Logo), magnetisch hinten am iPhone, Antenne am Schwanenhals.
1408 Kanäle und die Signalliste (inkl. Galileo E6, BDS B3I, NavIC) deuten auf einen **Unicore-UM980/UM981-Chip** hin.
Genauigkeit identisch zum Reach RX2 (H 7 mm + 1 ppm, V 14 mm + 1 ppm), BLE 5.3, RTCM3, IP67, 10 h Akku.
→ Für die Entwicklung bleibt der Reach RX2 die sichere Wahl (MFi-zertifiziert, NMEA-Protokoll offiziell dokumentiert). Eine leichtere Unicore-Lösung im Sodex-Stil ist später möglich, weil die App Standard-NMEA liest.
Makineo bietet eine **14-tägige Teststellung** an – gut für einen direkten Vergleich.

### 3.3a Kleinere Rover als Alternative zum Reach RX2 (Option für Phase 4)

Der Reach RX2 ist zuverlässig, aber am iPhone klobig (ca. 250 g Puck, s. Bilder im [Emlid Scanning Kit](https://emlid.com/pix4d-emlid-scanning-kit/)).
**Kriterium: in Deutschland / der EU kaufbar** (Gewährleistung, keine Zollabwicklung, CE).

| Gerät | Gewicht | Preis (ca.) | In DE/EU kaufbar? | Anbindung ans iPhone | Einschätzung |
|---|---|---|---|---|---|
| [Emlid Reach RX2](https://eu.store.emlid.com/en-es/collections/reachrx2-products) (Referenz) | ca. 250 g | ~2.000 € netto | ✅ Emlid EU-Store | Bluetooth Classic + MFi (`com.emlid.nmea`) | Zuverlässig, gut dokumentiert, klobig |
| [ArduSimple RTK Portable 2](https://www.ardusimple.com/news/new-product-rtk-portable-2/) / [RTK Smart Antenna](https://www.ardusimple.com/product/rtk-smart-antenna/) | leicht, kompakt | ab ~270–300 € | ✅ Hersteller in Spanien, [Shop](https://www.ardusimple.com/shop) | Bluetooth LE ([Anleitung iPhone](https://www.ardusimple.com/how-to-connectrtk-receiver-to-ios-device-iphone-ipad-or-ipod-via-bluetooth/)) | **Beste EU-Option für „klein + günstig“.** u-blox ZED-F9P (2 Frequenzen), eher Bastler-/Integratorqualität, IP53. Vor dem Kauf genaue Variante mit BLE klären. |
| [SparkFun RTK Torch](https://www.sparkfun.com/sparkfun-rtk-torch.html) | 428 g | ~2.150 $ | ⚠️ SparkFun über [exp-tech.de](https://exp-tech.de/en/products/sparkfun-rtk-express) (Torch dort nicht bestätigt) | Bluetooth LE | **Schwerer** als der RX2 – keine Lösung |
| [mapry R2](https://mapryshop.com/products/r1-1) | 188 g | ~660 $ | ❌ kein EU-Händler gefunden (Japan) | USB-C-Kabel | Ausgeschlossen |
| [DATAGNSS NANO RTK Pro](https://www.datagnss.com/products/nano-rtk-receiver) | sehr klein | sehr günstig | ❌ nur Direktimport China | Bluetooth LE | Ausgeschlossen |
| [Pix4D viDoc](https://www.adorama.com/p4d324001.html) | 242 g | auf Anfrage | ❌ nur US-Händler, teils eingestellt | Hülle pro iPhone-Modell | Ausgeschlossen |
| Sodex-Modul (blau, Unicore) | 115 g + Antenne | nicht einzeln erhältlich | nur im Paket über [Makineo](https://makineo.de/produkte/baudokumentation-3d/pocket-scan) | BLE 5.3 | Nicht einzeln kaufbar |

**Empfehlung:**
1. Für die Entwicklung beim **Emlid Reach RX2** bleiben – EU-Shop, MFi-zertifiziert, NMEA-Anbindung offiziell dokumentiert, Support.
2. Wird „klein“ wichtig: zusätzlich einen **ArduSimple**-Empfänger mit Bluetooth LE (~300 €) testen. Die App liest Standard-NMEA, ist also nicht an Emlid gebunden.
3. Aktuell nicht dringend: Zuerst kommt Drohne bzw. BLK2GO. Dort sitzt der Rover nur auf dem Lotstab – Größe egal.

Preise und Varianten aus Händler-/Herstellerseiten, vor dem Kauf prüfen.

### 3.4 Server – netcup, stufenweise

| Phase | Tarif | Preis netto | Warum |
|---|---|---|---|
| **Entwicklung** (ca. erste 3 Monate) | **netcup RS 2000 G12.5** – 8 dedizierte vCores, 16 GB RAM, 256 GB NVMe | **40,70 €/Monat** | Reicht für Web, API, Datenbank und Test-Drohnenflüge mit einigen hundert Fotos |
| **Echtbetrieb** (Kumpel fliegt regelmäßig) | **netcup RS 4000 G12.5** – 12 vCores, **32 GB RAM**, 512 GB NVMe | **77,83 €/Monat** | Typische Baugrubenflüge (einige hundert bis gut tausend Fotos) laufen flüssig |
| Speicher (Archiv) | netcup Storage Space B (500 GB) → später C (1 TB) | 8 → 15 €/Monat | Rohdaten und alte Projekte |
| Upgrade nur bei Bedarf | netcup RS 8000 G12.5 – 64 GB RAM | 148,42 €/Monat | Erst bei regelmäßig sehr großen Flügen oder mehreren Kunden |
| Domain | Subdomain von teufel-it.de | 0 € | z. B. `fixpunkt.teufel-it.de` |

netcup lässt sich innerhalb der Produktlinie hochstufen – wir müssen also nicht im Voraus für Reserven zahlen.

### 3.4a Alternative: eigener Rechner statt Mietserver (Stand 12.11.2026)

Die schwere Arbeit (OpenDroneMap) fällt nur gelegentlich an – ein eigener Rechner ist dafür deutlich stärker als ein Mietserver.
**Entscheidung erst nach den ersten echten Flügen**; bis dahin reicht ein vorhandener PC mit öffentlichen Testdaten.

| Teil | Empfehlung | ca. Preis |
|---|---|---|
| CPU | AMD Ryzen 9 9950X (16 Kerne) | ~430–460 € |
| RAM | 64 GB DDR5 (2× 32 GB) + großer Swap – 128 GB aktuell zu teuer (RAM-Preise stark gestiegen), später aufrüstbar | ~1.100–1.400 € |
| GPU | **NVIDIA** RTX 5060 Ti 16 GB (ODM nutzt nur CUDA, Image `opendronemap/odm:gpu`) | ~735–850 € |
| Rest | AM5-Board, 2 TB NVMe, 850-W-Netzteil, Gehäuse | ~600–800 € |
| **Summe** | Linux (Ubuntu Server), Docker/Podman | **ca. 3.000–3.500 €** |

- Kaufen: Fertig-PC mit Garantie über einen deutschen Systembauer (z. B. CSL, MIFCOM, dubaro) – Bewertungen auf Geizhals/Trustpilot vor der Bestellung vergleichen; Vergleich: [Geizhals Komplettsysteme 9950X](https://geizhals.eu/komplettsystem-amd-ryzen-9-9950x-a3442862.html)
- ODM-Faustregel RAM: ~32 GB für 500 Fotos, ~64 GB für 1.500 Fotos – eine Baugrube sind einige hundert Fotos
- Betrieb: Rechner zu Hause für die Verarbeitung, kleiner Mietserver (5–10 €/Monat) oder Cloudflare Tunnel/Tailscale für Web/Upload, Backup außer Haus (4–15 €/Monat)
- Laufend ca. 20–40 €/Monat statt 49–93 €; amortisiert sich gegenüber RS 4000 nach etwa 3–4 Jahren, ist dafür etwa doppelt so stark

### 3.5 Laufende Kosten

| | Entwicklung | Echtbetrieb |
|---|---|---|
| netcup Server | 40,70 € | 77,83 € |
| netcup Speicher | 8,00 € | 15,00 € |
| **Monatlich** | **~49 €** | **~93 €** |
| Apple Developer (jährlich, erst mit iOS-App) | – | 99 $ |
| **Pro Jahr gesamt** | **~590 €** | **~1.120–1.210 €** |

Nicht bei uns (Firma des Kumpels): Drohnen-Haftpflicht, Führerscheine, ggf. Mobilfunk für SAPOS an der Fernsteuerung.

### 3.6 Gesamtbild

| | Wir (Entwicklung) | Firma des Kumpels (Einsatz) |
|---|---|---|
| Einmalig, Phase 1 (Drohne) | **~0 €** | **ca. 5.400–6.300 €** (Matrice 4E, Akkus, Führerscheine, Registrierung) |
| Einmalig, später (iPhone + Rover) | ca. 2.100–5.600 € (Rover, ggf. Mac/iPhone) | ggf. iPhone Pro + eigener Rover (~3.450 €) |
| Laufend | ca. 49–93 €/Monat Server (+ 99 $/Jahr Apple, sobald iOS-App) | Versicherung ~100–300 €/Jahr, Mobilfunk |

Zum Vergleich: Allein das fertige *Pix4D & Emlid Scanning Kit* kostet 3.900 € netto, Sodex/Makineo verlangen für Hardware + Cloud deutlich mehr (Angebot auf Anfrage).

### 3.7 Wo kaufen / beantragen

**Jetzt (Drohne bzw. BLK2GO):**

| Was | Link | Notiz |
|---|---|---|
| **DJI Matrice 4E** (Empfehlung) | [Galaxus – DJI Matrice 4E (EU)](https://www.galaxus.de/en/s1/product/dji-matrice-4e-eu-1219-g-20-mpx-drone-54319549) · [Test bei drohnen.de](https://www.drohnen.de/56774/dji-matrice-4t-4e-test/) | EU-Version (C2) nehmen |
| DJI-Enterprise-Fachhändler | [Händlerübersicht drohnen.de](https://www.drohnen.de/22293/dji-enterprise-haendler/) | Mit Beratung, Schulung, Service – für den Erstkauf empfehlenswert |
| DJI Mavic 4 Pro (nur zum Vergleich, ohne RTK) | [Preise bei drohnen.de](https://www.drohnen.de/60004/dji-mavic-4-pro-preise/) | Bräuchte Passpunkte + Rover |
| **Leica BLK2GO** (Alternative) | [Leica-Shop EU](https://cpec.leica-geosystems.com/producto/blk2go-laser-scanner/) · [Leica-Shop](https://shop.leica-geosystems.com/reality-capture/blk2go/buy) | Verbindliches Angebot über Makineo |
| Leica BLK2GO PULSE | [Leica-Shop EU](https://cpec.leica-geosystems.com/producto/blk2go-pulse-laser-scanner/) · [Leica-Shop](https://shop.leica-geosystems.com/reality-capture/blk2go-pulse/buy) | Preis EU vs. USA widersprüchlich – prüfen |
| Makineo (Leica-Partner, Sodex-Vertrieb) | [Makineo – Pocket Scan / Teststellung](https://makineo.de/produkte/baudokumentation-3d/pocket-scan) | 14 Tage Teststellung, Angebot BLK2GO anfragen |
| Kompetenznachweis A1/A3, A2 | [LBA-Portal uas-betrieb.bund.de](https://uas-betrieb.bund.de) | A1/A3 online, A2 mit Zusatzprüfung |
| LBA-Betreiberregistrierung | [uas-betrieb.bund.de – Registrierung als Betreiber](https://uas-betrieb.bund.de/homepage/en/applications-administrative-services/registration-as-an-operator) | e-ID für die Drohne |
| Geozonen prüfen | [dipul.de](https://www.dipul.de) | vor jedem Flug |
| SAPOS-Korrekturdaten | [Zentrale Stelle SAPOS – Gebühren & Registrierung](https://zentrale-stelle-sapos.de/en/fees-registration/) | in BW kostenlos, Registrierung nötig |
| netcup Root-Server | [netcup Root-Server](https://www.netcup.com/de/server/root-server) | RS 2000 (Entwicklung) → RS 4000 (Betrieb) |
| netcup Speicher | [netcup Storage Space](https://netcup.com/en/server/storage-space) | Tarif B (500 GB) → C (1 TB) |

**Später (iPhone-App mit Rover, Phase 4):**

| Was | Link | Notiz |
|---|---|---|
| Emlid Reach RX2 | [Emlid EU-Store – Reach RX2](https://eu.store.emlid.com/en-es/collections/reachrx2-products) · [Datenblatt](https://cdn.emlid.com/rx2-landing/Reach+RX2+datasheet.pdf) | Preise netto; Lotstab ebenfalls dort |
| Emlid Scanning Handle | [Emlid Store – Scanning Handle](https://store.emlid.com/products/pix4d-emlid-scanning-handle) | 229 $ netto – oder selbst drucken |
| Pix4D & Emlid Scanning Kit (zum Ansehen) | [Emlid – Scanning Kit](https://emlid.com/pix4d-emlid-scanning-kit/) | Fotos, wie RX2 + iPhone zusammen aussehen |
| Kleinerer Rover (Test, EU) | [ArduSimple Shop](https://www.ardusimple.com/shop) · [RTK Portable 2](https://www.ardusimple.com/news/new-product-rtk-portable-2/) | s. 3.3a, Variante mit Bluetooth LE |
| Mac mini | [Apple Store – Mac mini kaufen](https://www.apple.com/de/shop/buy-mac/mac-mini) | M5 Pro konfigurieren |
| iPhone 18 Pro / Pro Max | [Apple Store – iPhone kaufen](https://www.apple.com/de/shop/buy-iphone) | Pro-Modell (LiDAR) |
| Apple Developer Program | [developer.apple.com/programs](https://developer.apple.com/programs/) | 99 $/Jahr |

---|---|---|
| **DJI Matrice 4E** | [Galaxus – DJI Matrice 4E (EU)](https://www.galaxus.de/en/s1/product/dji-matrice-4e-eu-1219-g-20-mpx-drone-54319549) | EU-Version (C2) nehmen |
| DJI-Enterprise-Fachhändler | [Händlerübersicht drohnen.de](https://www.drohnen.de/22293/dji-enterprise-haendler/) | Mit Beratung, Schulung, Service – für den Erstkauf empfehlenswert |
| Kompetenznachweis A1/A3, A2 | [LBA-Portal uas-betrieb.bund.de](https://uas-betrieb.bund.de) | A1/A3 online, A2 mit Zusatzprüfung |
| LBA-Betreiberregistrierung | [uas-betrieb.bund.de – Registrierung als Betreiber](https://uas-betrieb.bund.de/homepage/en/applications-administrative-services/registration-as-an-operator) | e-ID für die Drohne |
| Geozonen prüfen | [dipul.de](https://www.dipul.de) | vor jedem Flug |
| SAPOS-Korrekturdaten | [Zentrale Stelle SAPOS – Gebühren & Registrierung](https://zentrale-stelle-sapos.de/en/fees-registration/) | in BW kostenlos, Registrierung nötig |
| netcup Root-Server | [netcup Root-Server](https://www.netcup.com/de/server/root-server) | RS 2000 (Entwicklung) → RS 4000 (Betrieb) |
| netcup Speicher | [netcup Storage Space](https://netcup.com/en/server/storage-space) | Tarif B (500 GB) → C (1 TB) |
| *Später:* Emlid Reach RX2, Scanning Handle, Lotstab | [Emlid EU-Store – Reach RX2](https://eu.store.emlid.com/en-es/collections/reachrx2-products) | Preise netto |
| *Später:* Mac mini | [Apple Store – Mac mini kaufen](https://www.apple.com/de/shop/buy-mac/mac-mini) | M5 Pro konfigurieren |
| *Später:* iPhone 18 Pro / Pro Max | [Apple Store – iPhone kaufen](https://www.apple.com/de/shop/buy-iphone) | Pro-Modell (LiDAR) |
| *Später:* Apple Developer Program | [developer.apple.com/programs](https://developer.apple.com/programs/) | 99 $/Jahr |

---

## 4. Rechtliches (Kurzfassung)

- **Keine Zertifizierung nötig** für das Erfassen, Messen und Massenermitteln als Dienstleistung oder Software.
- **Amtliche Vermessung** (Kataster, Grenzen, Gebäudeeinmessung) dürfen nur ÖbVI/Vermessungsämter. Fixpunkt nie als „amtliche Vermessung“ bezeichnen.
- **Drohne:** Registrierung, Kompetenznachweis und Versicherung liegen beim Betreiber. Die Firma des Kumpels hat bisher **weder Drohne noch Führerschein** – empfohlen: Die Firma kauft die Drohne und ist Betreiber, ein Mitarbeiter macht A1/A3 + A2 (s. 3.2). Haftpflichtversicherung ist Pflicht. Über Wohngrundstücken Zustimmung des Eigentümers (§21h LuftVO). Geozonen auf [dipul.de](https://www.dipul.de) prüfen (z. B. Baden Airpark).
- **Datenschutz:** Unbeteiligte Personen und Nachbargrundstücke vermeiden bzw. unkenntlich machen.
- **Eigene Hardware verkaufen** → CE, Funkanlagenrichtlinie, ElektroG/WEEE, Batterierecht. Daher: unsere Software auf gekaufter Hardware.

---

## 5. Architektur

```
 iOS-App (Swift, später)         Drohne (Matrice 4E, Firma des Kumpels)
 LiDAR + ARKit-Posen             Fotos mit RTK-Position (EXIF/XMP)
 RTK über Bluetooth (Reach RX2)  Flugplan (Pilot 2 → später WPML/KMZ, Cloud API)
          │                               │
          └───────────────┬───────────────┘
                          ▼  HTTPS (Upload, API, SSE)
   ┌──────────────────────────────────────────────────────────┐
   │  fixpunkt  – EINE Programmdatei (Rust)                   │
   │  axum: /api · Auth · Upload · Live-Updates (SSE)         │
   │  Web-UI eingebettet (rust-embed)  ← ui/dist              │
   │  Jobs im Prozess (tokio + rayon), Status in SQLite       │
   │  Rechenkern als Bibliothek (crates/processing)           │
   │  Kacheln für den Viewer selbst erzeugen                  │
   └───────────────┬───────────────────────┬──────────────────┘
                   │ Dateien               │ HTTP
                   ▼                       ▼
     Datenverzeichnis (/var/lib/fixpunkt)  NodeODM (Docker, nur für Drohnenfotos)
     SQLite · Rohdaten · Ergebnisse · Kacheln
```

**Ein Server, eine Datei:** wie bei `haul` – der Rust-Server bringt die gebaute Web-UI mit (`rust-embed`),
dazu SQLite und das Dateisystem statt Postgres und S3. Installation = Datei kopieren + systemd-Dienst
(oder ein Docker-Image). Einziger Fremddienst ist NodeODM für die Photogrammetrie.

**Grundprinzip:** Die Cloud nimmt „Punkte bzw. Bilder + RTK-Trajektorie“ entgegen – egal woher.
iPhone, Drohne, später Android-PWA oder ein eigener LiDAR-Scanner sind austauschbare Erfassungsquellen.

### 5.1 iOS-App (Swift) – Verantwortung: iOS-Entwicklerin
- SwiftUI-Oberfläche: Baustelle wählen → RTK-Status (Fix/Float/kein Fix) → Scannen → Hochladen
- ARKit: `sceneDepth`, Scene Reconstruction (`ARMeshAnchor`), Kamerapose pro Frame
- Rover-Anbindung über zwei austauschbare „Treiber“ (s. 5.1a): **External Accessory / MFi** (Emlid) und **CoreBluetooth / BLE** (ArduSimple, SparkFun u. a.) → NMEA (GGA, GST, RMC) lesen
- NTRIP-Client: SAPOS-Korrekturen (RTCM3) übers iPhone-Internet holen und per Bluetooth an den Rover schicken – der Rover hat **kein eigenes Internet**
- Synchrone Aufzeichnung: ARKit-Pose + RTK-Position + Zeitstempel
- Passpunkt-Modus: GCP mit Lotstab einmessen
- Offline-Puffer bei Funkloch, Upload im Hintergrund
- Optional: Object Capture (`PhotogrammetrySession`) für texturierte Meshes
- Reine Logik (NMEA, NTRIP, Datenformat) als Swift-Package – auch unter Linux testbar

### 5.1a Wie der Rover mit der App spricht

Keine Web-API nötig – der Rover funkt über **Bluetooth** Textzeilen im Standardformat **NMEA**, mehrmals pro Sekunde
(z. B. `$GNGGA,…,4,…` – die `4` heißt RTK-Fix). Das sprechen praktisch alle GNSS-Empfänger.

```
SAPOS-Server ──Internet──▶ iPhone (NTRIP-Client) ──Bluetooth──▶ Rover   Korrekturdaten (RTCM3)
Rover ──Bluetooth──▶ iPhone (unsere App)                                Position (NMEA)
```

**Das iPhone-Problem:** Apple erlaubt nur zwei Wege zu Bluetooth-Geräten:

| Weg | Apple-Framework | Was es heißt |
|---|---|---|
| **Bluetooth LE** | CoreBluetooth | Keine Erlaubnis nötig. Jeder Rover, der NMEA über BLE sendet (ArduSimple-BLE-Varianten, SparkFun) |
| **Bluetooth Classic mit MFi** („Made for iPhone“) | External Accessory, Protokoll des Herstellers (Emlid: `com.emlid.nmea`) | Funktioniert, NMEA lesen und RTCM senden. Für den **App Store** muss vermutlich der Hersteller unsere App bei Apple freigeben → früh mit Emlid klären. Entwickeln/Testen geht sofort. |
| Bluetooth Classic **ohne** MFi | – | **Geht auf dem iPhone nicht** (viele billige Rover sind nur für Android) |

Emlid bietet zusätzlich „Rover als Ortungsquelle“ (iOS ersetzt das interne GPS, Core Location liefert die Rover-Position).
Für uns **nicht ausreichend**: iOS kann die Werte verfälschen, kein RTK-Status → wir lesen NMEA direkt.
Quelle: [Emlid – How to integrate with Reach RX2](https://docs.emlid.com/reachrx2/api-integration/api-integration-intro/)

**Funktioniert es mit jedem Rover?** Nein. Voraussetzung: **NMEA über Bluetooth LE** *oder* **MFi + veröffentlichtes Protokoll**.

| Rover | Mit unserer App? |
|---|---|
| Emlid Reach RX2 | ✅ MFi (`com.emlid.nmea`), offiziell dokumentiert – App-Store-Freigabe mit Emlid klären |
| ArduSimple (Variante mit BLE) | ✅ Bluetooth LE – **nicht** die Variante mit nur klassischem Bluetooth kaufen |
| SparkFun (BLE) | ✅ Bluetooth LE |
| Billig-Rover nur mit klassischem Bluetooth | ❌ |
| Leica, Trimble, Sodex-Modul | ⚠️ nur mit Hersteller-SDK/Protokoll, meist nicht frei |

**Umsetzung:** Zwei kleine Treiber (MFi und BLE) hinter einer gemeinsamen Schnittstelle. NMEA-Parser, NTRIP-Client und Scan-Logik bleiben gleich → nicht an einen Hersteller gebunden.

### 5.2 Server (Rust) – Verantwortung: Tristan
Aufbau wie in `haul`:

| Bereich | Technik |
|---|---|
| HTTP, API | `tokio`, `axum`, `serde`, JSON; Live-Fortschritt der Jobs per Server-Sent Events (`/api/events`); Logs mit `tracing` |
| Datenbank | SQLite über `sqlx`, Migrationen eingecheckt, in die Binärdatei eingebettet, laufen beim Start |
| Dateien | Datenverzeichnis: Rohdaten, Ergebnisse, Kacheln je Aufnahme; Sicherung per restic/rsync auf netcup Storage Space |
| Jobs | Warteschlange in SQLite, Ausführung im Prozess (`spawn_blocking` + `rayon`), überlebt Neustarts |
| Rechenkern | `crates/processing` direkt als Bibliothek (kein Aufruf der CLI) |
| Web-UI | `rust-embed` auf `ui/dist`; `build.rs` legt leere Ordner an, damit `cargo test` ohne Bun baut |
| Auth | eigener Login: argon2id, Sitzungs-Cookie, CSRF, Passkeys (`webauthn-rs`); Rollen (Admin, Bauleiter, nur lesen); API-Token für die iOS-App |
| Upload | große Dateien und Fotomengen in Teilen, fortsetzbar (tus-Protokoll o. ä.) |
| Drohne | NodeODM per HTTP: Fotos übergeben, Fortschritt abfragen, Ergebnis abholen |
| Ausliefern | Docker-Image `tristanteu/fixpunkt` (amd64 + arm64) in drei Stufen: Bun (UI) → Rust mit cargo-chef (Binärdatei) → Debian slim; betrieben mit Podman-Quadlet oder Docker Compose. Zusätzlich statische musl-Binärdateien + `install.sh` mit systemd-Unit (wie haul) |

Datenmodell: Mandant → Baustelle → Aufnahme (Scan/Flug) → Rohdaten, Ergebnisse, Passpunkte, Messungen, Bereiche.

Begründung der Abweichungen von `tech-stack.md` (Rust statt TypeScript, SQLite trotz mehrerer Nutzer,
SPA statt TanStack Start, eigener Login statt Better Auth) steht in `CLAUDE.md`.

### 5.2a Web-UI (TypeScript) – Verantwortung: Tristan
Nach `tech-stack.md` in `firsttris/workflows`, wie in snapraid-ui:
- **Bun**, **Vite**, **TypeScript strict**, **React** mit **TanStack Router** (dateibasierte Routen), **TanStack Query**, **TanStack Table/Virtual** für lange Listen
- **shadcn/ui**-Stil (eigene Komponenten auf **Radix**, `class-variance-authority`), **Tailwind CSS 4** (hell/dunkel), Icons von **lucide**
- Diagramme (Massen über die Zeit, Profile) mit **uPlot**
- Formulare kontrolliert, bei Bedarf **TanStack Form**; **Zod** an jeder Grenze (API-Antworten, Uploads)
- **Paraglide JS** für Texte (`ui/messages/{de,en}.json`, Deutsch zuerst); der Server schickt Schlüssel statt fertiger Texte
- **PWA**: auf dem Tablet/Handy auf der Baustelle installierbar
- **Single-Page-App** (kein TanStack Start mit eigenem JS-Server): Der Rust-Server liefert nur statische Dateien aus
- Viewer: **CesiumJS** (Karte + Punktwolke/3D Tiles + Orthofoto in einem) oder Potree – Entscheidung nach einem Test mit echten ODM-Daten
- Messwerkzeuge im Viewer (Punkt, Linie, Fläche, Profil, Volumen-Polygon), Ergebnisse vom Rechenkern
- Berichte als PDF, DXF-Pläne als Overlay
- `bun run dev`: `bun run --parallel dev:server dev:ui` – Server auf :8080, UI mit Hot Reload auf :5173

### 5.2b Tooling, CI, Releases
Wie in allen Projekten (`firsttris/workflows`):
- **Biome** (TS), `cargo fmt` + `cargo clippy -D warnings` (Rust); **Vitest**, `cargo test`; **Playwright** auf Bun für E2E und Screenshots
- `ci.yml` mit dem gemeinsamen Kopf (Entwürfe bauen nicht, `workflow_call` für den Release) – ✅ angelegt, prüft bisher den Rechenkern
- Release nur per Tag `vX.Y.Z`: `bump.yml` (Button „Bump version“, Version aus `Cargo.toml`) → `release.yml` → `tested-on-main` → `docker-release.yml` (Image + GitHub-Release) → musl-Binärdateien anhängen
- **Renovate** mit den gemeinsamen Einstellungen – ✅ angelegt
- Doku später mit **MkDocs Material** auf GitHub Pages, README nach der Vorlage

### 5.3 Rechenkern (Rust, `crates/processing`) – Verantwortung: Tristan – ✅ fertig
| Aufgabe | Crates / Tools |
|---|---|
| Punktwolken lesen/schreiben | `las`, `laz-rs` |
| Georeferenzierung (Helmert/Umeyama, Ausgleich) | `nalgebra` |
| Koordinaten ETRS89/UTM32 (EPSG:25832), Höhen DHHN2016 über GCG2016 | eigener Code (Krüger-Reihen), kein PROJ nötig |
| Nachbarschaftssuche, Parallelisierung | `kiddo`, `rayon` |
| Bodenfilter, DGM, Raster | eigener Code (SMRF), kein PDAL nötig |
| Volumen | DGM-Differenz / TIN (`delaunator`) |
| Kacheln für den Browser | eigener Octree-Schreiber in Rust (3D Tiles bzw. Potree 2) – sonst bräuchte die Programmdatei PotreeConverter/py3dtiles daneben |
| Drohnen-Photogrammetrie | NodeODM per HTTP-API |

### 5.4 Drohnen-Pipeline
1. Mapping-Mission in **DJI Pilot 2** (Fläche zeichnen, Überlappung 75–80 %, ggf. Schrägaufnahmen)
2. Fotos hochladen → **NodeODM** → Orthofoto, DSM/DTM, Punktwolke, Mesh
3. Ergebnis landet im selben Koordinatensystem wie Handscans
4. Später: Flugplan in Fixpunkt zeichnen → **WPML/KMZ** exportieren
5. Noch später: **DJI Cloud API** (MQTT, selbst gehostet) – Mission pushen, Fotos automatisch empfangen

---

## 6. Phasen

**Reihenfolge: Drohne zuerst** (Baugruben), iPhone + Rover danach.

### Phase 0 – Setup (1–2 Wochen)
- [x] Repo-Struktur begonnen: `crates/processing` (Volumen-Engine: Aushub-Differenz, Haufwerk, Tests)
- [x] Rechenkern fertig (`crates/processing`, s. dortige README): Aushub je Bereich, Haufwerk, Zeitreihe, Soll/Ist,
      Bodenfilter (SMRF), Ausreißer, Passpunkt-Transformation (Helmert), UTM32 + GCG2016, Messen/Profile,
      Exporte GeoTIFF/DXF/LandXML/OBJ/PLY/CSV, ODM-GeoTIFF als Eingabe – 43 Tests
- [x] CI (`ci.yml`), Renovate, `CLAUDE.md` nach den Vorlagen aus `firsttris/workflows`
- [ ] Weitere Struktur: `crates/server` (Rust, axum), `ui/` (Bun, Vite, React, TanStack, shadcn/ui, Paraglide), `docker/`, `bump.yml` + `release.yml` (`apps/ios` erst ab Phase 4)
- [ ] netcup RS 2000 aufsetzen, NodeODM testen
- [ ] SAPOS-Zugang (BW) beantragen
- [ ] **Mit der Firma klären:** Drohne (Matrice 4E) oder BLK2GO (+ Rover)? Matrice 4E kaufen? Wer macht A1/A3 + A2? Betreiberregistrierung, Versicherung
- [ ] Erste Baugrube als Pilotprojekt aussuchen (Termin vor dem Aushub!)

### Phase 1 – Backend + Viewer (3–4 Wochen)
- [ ] Server-Gerüst: axum, SQLite/sqlx, Login, UI per rust-embed, eine Binärdatei + Docker-Image
- [ ] Datenmodell: Mandant, Baustelle, Aufnahme, Rohdaten, Ergebnis, Passpunkt, Messung, Bereich
- [ ] Upload großer Fotomengen (fortsetzbar) + Job-Warteschlange mit Live-Fortschritt (SSE)
- [ ] Jobs rufen den Rechenkern auf: LAS/LAZ (und E57 vom BLK2GO) einlesen, transformieren, Browser-Kacheln erzeugen
- [ ] Viewer mit Punkt-, Längen-, Flächen-, Höhenmessung
- [ ] Mit öffentlichen OpenDroneMap-Datensätzen arbeiten, bis eigene Flüge da sind

### Phase 2 – Drohne + Aushubberechnung (2–3 Wochen)
- [ ] NodeODM anbinden (Fotos mit RTK-Position aus EXIF/XMP)
- [ ] Orthofoto, DGM, Punktwolke im Viewer
- [ ] **Aushub: Differenz zweier Flüge (Urgelände ↔ Baugrube)**, Ergebnis in m³ + Höhendifferenz-Karte
- [ ] Volumen Haufwerk gegen Basisfläche
- [ ] Erster echter Flug: Baugrube vor und nach dem Aushub

### Phase 3 – Genauigkeit Drohne (laufend)
- [ ] Ergebnisse gegen Lieferscheine / LKW-Ladungen bzw. Aufmaß plausibilisieren
- [ ] Optional: 2–3 Kontrollpunkte einmessen (Rover leihen oder kaufen) für einen Genauigkeitsnachweis
- [ ] Höhenbezug DHHN2016 über GCG2016 prüfen

### Phase 4 – iOS-App mit Rover (4–6 Wochen, erst wenn Leitungsgräben gewünscht)
- [ ] Mac, iPhone Pro, Reach RX2, Apple Developer Account beschaffen (s. 3.3); ggf. kleineren Rover testen (s. 3.3a)
- [ ] Rover-Treiber: MFi/External Accessory (Reach RX2, `com.emlid.nmea`) und BLE (CoreBluetooth); NMEA-Parser, RTK-Status
- [ ] Bei Emlid klären: Freigabe unserer App für MFi im App Store
- [ ] NTRIP-Client (SAPOS)
- [ ] ARKit-LiDAR-Scan mit synchroner Pose-/RTK-Aufzeichnung
- [ ] Hebelarm-Kalibrierung (Antenne ↔ Kamera)
- [ ] GCP-Modus mit Lotstab
- [ ] Upload, Offline-Puffer
- [ ] Helmert/Umeyama über die Gesamttrajektorie, abschnittsweiser Ausgleich gegen ARKit-Drift
- [ ] Ziel: ≤ 2–3 cm bei Gräben bis 30–50 m
- [ ] TestFlight an den Kumpel

### Phase 5 – Ausbau
- [ ] Zwangspunkte / Leitungsdokumentation
- [ ] Plan/Ist-Vergleich mit DXF, Abweichungs-Heatmap
- [ ] Zeitreihen, Dashboards, PDF-Berichte
- [ ] Exporte DXF, LandXML, CSV, GeoTIFF
- [ ] Flugplanung in Fixpunkt (WPML), danach DJI Cloud API
- [ ] Android-PWA (Photogrammetrie + Web Bluetooth)
- [ ] Optional: leichter Unicore-Empfänger im Sodex-Stil statt Reach RX2
- [ ] Optional: eigener Handscanner (Livox Mid-360 + FAST-LIO2)

**Realistisch:** Erste nutzbare Aushubberechnung per Drohne in **ca. 2 Monaten** nebenher – vorausgesetzt, die Firma hat bis dahin Drohne und Führerschein.

---

## 7. Machbarkeit

| Teil | Einschätzung |
|---|---|
| Web-UI, API, Viewer | 9/10 |
| Drohnen-Pipeline (ODM) | 8/10 |
| Messen, Volumen, Export | 8/10 |
| iOS-App (mit iOS-Entwicklerin im Team) | 9/10 |
| Georeferenzierung / Drift bei langen Scans | 6/10 |
| **Aushub Baugrube per RTK-Drohne (erstes Ziel)** | **8/10** |
| **Gesamt: Kumpel nutzt es für Doku & Massen** | **9/10** |
| Grabenaufmaß nachweisbar 1–2 cm | 7/10 |
| Als Produkt an fremde Firmen verkaufen | 5/10 |

---

## 8. Risiken

| Risiko | Gegenmaßnahme |
|---|---|
| Firma kauft keine Drohne / niemand macht den Führerschein | Früh klären (Phase 0); notfalls selbst Betreiber werden oder Drohnendienstleister beauftragen |
| Baugrube wird verfüllt, bevor geflogen wurde | Flugtermine in den Bauablauf einplanen (vor Aushub, nach Aushub) |
| ARKit-Drift bei langen Gräben | Abschnittsweiser Ausgleich, Passpunkte, Scanlänge begrenzen |
| Kein RTK-Fix (Abschattung, Funkloch) | Fix-Status erzwingen, Offline-Puffer, Post-Processing (RTKLIB) |
| Hebelarm Antenne ↔ Kamera ungenau | Starre Halterung, Kalibrierroutine im Testfeld |
| Höhenbezug falsch (ellipsoidisch vs. DHHN2016) | GCG2016 einbinden, gegen amtlichen Festpunkt prüfen |
| Große Datenmengen (Drohne: tausende Fotos) | Resumable Upload, Verarbeitung serverseitig, Speicherkonzept |
| Bedienung für Laien | Ein-Knopf-Workflow, klare Fehlermeldungen, früh mit dem Kumpel testen |

---

## 9. Offene Fragen

- [ ] **Kauft die Firma die Matrice 4E** (ca. 5.400–6.300 € inkl. Akkus, Führerschein, Registrierung)? Oder wollen wir selbst Betreiber sein und als Dienstleister fliegen?
- [ ] **Wer macht den Führerschein** (A1/A3 + A2)?
- [ ] **Drohne oder Leica BLK2GO?** Bekommt die Firma einen BLK2GO günstig (Makineo, Aktion, Miete)? Teststellung/Vorführung beider Varianten auf einer echten Baugrube
- [ ] Hat die Firma einen GNSS-Rover (Leica/Trimble, z. B. für Absteckung oder Maschinensteuerung)? Dann Kontrollpunkte ohne Neukauf.
- [ ] Typische Baugrubengröße? Wie lange bleibt sie offen (Flug vor dem Verfüllen)?
- [ ] Welche Ausgabeformate braucht der Bauleiter / das Büro konkret (m³-Bericht als PDF? DXF? LandXML? REB?)
- [ ] Open Source oder privat? Geschäftsmodell, falls Produkt
- [ ] Später: iPhone 18 Pro oder Pro Max?

---

## 10. Referenzen

- Vorbild: [SDX-PocketScan](https://makineo.de/produkte/baudokumentation-3d/pocket-scan) · [SDX-Cloud](https://makineo.de/produkte/baudokumentation-3d/infrastruktur-sdx-cloud) · [Datenblatt](https://makineo.de/fileadmin/Home/Produkte/Baudokumentation/SDX-POCKETSCAN/2026/WEB_Makineo_SDX-PocketScan_Datenblatt.pdf)
- Vergleichbar: Pix4Dcatch + Emlid Scanning Kit
- Hardware: [Emlid Reach RX2 (EU-Store)](https://eu.store.emlid.com/en-es/collections/reachrx2-products) · DJI Matrice 4E · Mac mini M5 Pro · iPhone 18 Pro · netcup
- Open Source: OpenDroneMap / NodeODM, COLMAP, OpenMVS, PDAL, Open3D, Potree, CesiumJS, RTKLIB, PROJ
- Korrekturdaten: SAPOS Baden-Württemberg (HEPS, kostenlos)
- Drohnenrecht: [dipul.de](https://www.dipul.de), Luftfahrt-Bundesamt
