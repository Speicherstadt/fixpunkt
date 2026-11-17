---
description: >-
  Welche Geräte Fixpunkt braucht: Drohne mit RTK für Baugruben, Server für Auswertung und NodeODM, optional
  Laserscanner; für die geplante iOS-App iPhone Pro mit LiDAR, RTK-Rover und Mac zum Entwickeln.
---

# Geräte

Was man für Fixpunkt braucht – heute für die Drohne, später für die iOS-App. Preise sind Richtwerte (Stand
November 2026, inkl. MwSt. bei Endgeräten, netto bei Servern und Vermessungsgeräten); vor dem Kauf prüfen.

## Was man wofür braucht

| Gerät | Was es macht | Wann |
|---|---|---|
| **Drohne mit RTK** | fliegt die Baugrube automatisch ab, macht Fotos mit cm-genauer Position; daraus entsteht das Geländemodell | **jetzt** – Hauptwerkzeug für Baugruben, Haufwerke, Flächen |
| **Server** | Fixpunkt selbst und NodeODM für die Auswertung der Drohnenfotos | **jetzt** |
| **iPhone Pro mit LiDAR** | scannt aus nächster Nähe, auch in schmalen Gräben mit Verbau | **mit der iOS-App** |
| **RTK-Rover** | Position eines Punkts auf 1–2 cm; am iPhone verortet er den Scan, auf dem Lotstab misst er Passpunkte | **mit der iOS-App**, oder zur Kontrolle der Drohne |

Jedes Verfahren braucht zweierlei: die **Form** des Geländes (Kamera, LiDAR, Laserscanner) und die **Lage** in
Landeskoordinaten (RTK). Eine RTK-Drohne hat beides eingebaut; iPhone und Handlaserscanner brauchen einen Rover.

## Drohne

**Empfehlung: DJI Matrice 4E** (EU-Version, Klasse C2), ca. 4.500–4.900 € mit Fernsteuerung DJI RC Plus 2.

- **RTK eingebaut:** mit SAPOS-Korrekturdaten cm-genau, ohne Passpunkte und ohne Rover
- 4/3-Zoll-Kamera mit mechanischem Verschluss – wichtig für scharfe Kartierfotos in Bewegung
- Kartierflüge in DJI Pilot 2; die Route plant Fixpunkt ([Flugplanung](flugplanung.md))
- eine Baugrube: etwa 5–15 Minuten Flug

Eine günstigere Kameradrohne ohne RTK bräuchte bei jedem Flug 5–10 eingemessene Passpunkte, also zusätzlich einen
Rover und jedes Mal deutlich mehr Arbeit vor Ort.

| Einmalig | ca. |
|---|---|
| DJI Matrice 4E mit RC Plus 2 | 4.500–4.900 € |
| 2 Ersatzakkus | 400 € |
| Bruchschutz (optional) | 400–600 € |
| Kompetenznachweis A1/A3 und A2 (Luftfahrt-Bundesamt) | 50–300 € |
| Betreiberregistrierung (e-ID) | 20–50 € |
| **Summe** | **ca. 5.400–6.300 €** |
| Drohnen-Haftpflicht (Pflicht, jährlich) | 100–300 € |

Betreiber – Registrierung, Versicherung, Haftung – ist, wer die Drohne kauft und fliegt. Vor jedem Flug Geozonen
auf [dipul.de](https://www.dipul.de) prüfen; über Wohngrundstücken braucht es die Zustimmung des Eigentümers.

### Alternative: Handlaserscanner

Ein Handlaserscanner mit SLAM (z. B. Leica BLK2GO) erzeugt beim Umrunden der Grube eine sehr dichte Punktwolke,
ohne Führerschein und unabhängig vom Wetter, auch in schmalen Gräben und unter Bäumen. Er kennt aber seine Lage
in Landeskoordinaten nicht: 3–4 Zielmarken am Grubenrand werden einmal mit einem Rover eingemessen, jeder Scan
darauf ausgerichtet ([Passpunkte](berechnungen.md#passpunkte)). Fixpunkt liest E57 direkt. Lohnt sich vor allem,
wenn man günstig an ein Gerät kommt; zum Neupreis hat die Drohne für Baugruben das bessere Verhältnis.

## Server

Fixpunkt selbst braucht wenig; die Auswertung der Drohnenfotos mit NodeODM braucht Arbeitsspeicher.

| Einsatz | Empfehlung | ca. |
|---|---|---|
| Einstieg, Testflüge mit einigen hundert Fotos | 8 Kerne, 16 GB RAM, 256 GB NVMe | 40 €/Monat |
| Regelmäßige Baugrubenflüge (bis gut 1.000 Fotos) | 12 Kerne, **32 GB RAM**, 512 GB NVMe | 80 €/Monat |
| Archiv für Rohdaten und alte Baustellen | 500 GB–1 TB Speicher | 8–15 €/Monat |

Faustregel für NodeODM: etwa 32 GB RAM für 500 Fotos, 64 GB für 1.500. Wer öfter große Flüge rechnet, fährt mit
einem eigenen Rechner günstiger: 16 Kerne, 64 GB RAM und eine NVIDIA-Grafikkarte mit 16 GB (NodeODM rechnet mit
CUDA) für etwa 3.000–3.500 €. Installation: [Installation](installation.md).

## Für die iOS-App

Die iOS-App ist geplant ([Roadmap](roadmap.md#ios-app-mit-rtk-rover)): Scannen mit dem LiDAR des iPhones, verortet
durch einen RTK-Rover – für schmale Leitungsgräben und kleine Flächen, wo die Drohne nicht hinkommt.

### Zum Messen auf der Baustelle

| Gerät | Empfehlung | ca. | Warum |
|---|---|---|---|
| iPhone | **iPhone 18 Pro**, 256 GB (jedes iPhone Pro ab 12 Pro hat LiDAR) | 1.450 € | LiDAR-Sensor; ein vorhandenes Pro-Modell reicht für den Anfang |
| RTK-Rover | **Emlid Reach RX2** | 2.000 € netto | in der EU erhältlich, für das iPhone zertifiziert (MFi), Schnittstelle offen dokumentiert |
| Halterung iPhone + Rover | selbst gedruckt | 5 € | hält beide fest übereinander; der Abstand zwischen Antenne und Kamera wird einmal kalibriert |
| Lotstab | Vermessungslotstab 2 m mit Libelle | 50–100 € | für Passpunkte und Kontrollpunkte |
| Korrekturdaten | SAPOS HEPS | 0 € | in Baden-Württemberg kostenlos; das iPhone holt sie übers Mobilfunknetz und gibt sie an den Rover |

**Kleiner und günstiger:** Rover mit Bluetooth LE (z. B. ArduSimple, u-blox ZED-F9P, ab ca. 300 €) funktionieren
ebenfalls, sind aber eher Bastlerqualität. Wichtig beim Kauf: Der Rover muss NMEA über **Bluetooth LE** senden
oder für das iPhone zertifiziert sein (MFi). Rover nur mit klassischem Bluetooth gehen am iPhone nicht.

### Zum Entwickeln

| Was | Empfehlung | ca. |
|---|---|---|
| Mac | Mac mini mit Apple-Chip, 24 GB RAM, 512 GB | 2.000 € |
| Apple Developer Program | für TestFlight und App Store | 99 $/Jahr |
| Testgerät | das iPhone Pro von oben | – |

Ein vorhandener aktueller Mac mit Xcode reicht. Für den App Store mit einem MFi-Rover muss in der Regel der
Hersteller die App bei Apple freigeben – das wird früh geklärt.
