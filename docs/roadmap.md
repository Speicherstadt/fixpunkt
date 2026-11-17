---
description: >-
  Roadmap von Fixpunkt: was fertig ist, die Erprobung mit echten Drohnenflügen, die iOS-App mit LiDAR und
  RTK-Rover für Leitungsgräben, große Punktwolken im Browser und spätere Ausbaustufen.
---

# Roadmap

Was fertig ist, steht im [Funktionsumfang](funktionen.md). Hier: was als Nächstes kommt, ungefähr in dieser
Reihenfolge. Welche Geräte dafür nötig sind, steht unter [Geräte](geraete.md).

## Fertig: Web und Drohne

Die Web-Anwendung für Baugruben per Drohne ist vollständig: Daten hochladen oder Drohnenfotos über NodeODM
auswerten, Flug planen, ansehen und messen, Aushub, Haufwerk, Soll/Ist, Massenziele und Lagerbestand,
Abrechnung mit Preisen und Lieferscheinen, Dokumentation, Berichte und Export – getestet auf allen Schichten mit synthetischen Testdaten, deren Ergebnis
vorher bekannt ist ([Entwicklung und Tests](entwicklung.md)).

## Als Nächstes: Erprobung auf der Baustelle

- [ ] Erster echter Flug über eine Baugrube, vor und nach dem Aushub
- [ ] Ergebnisse gegen Lieferscheine, LKW-Ladungen oder ein Aufmaß plausibilisieren
- [ ] 2–3 Kontrollpunkte mit einem Rover einmessen als Genauigkeitsnachweis
- [ ] Höhenbezug DHHN2016 über das Quasigeoid GCG2016 an echten Daten prüfen
- [ ] Bis dahin: mit öffentlichen Datensätzen von OpenDroneMap arbeiten

## iOS-App mit RTK-Rover

Für schmale Leitungsgräben, Verbau und kleine Flächen, wo eine Drohne nicht hinkommt: ein iPhone Pro scannt mit
seinem LiDAR, ein RTK-Rover am iPhone liefert dazu die Position auf 1–2 cm. Der Scan landet als Aufnahme in
Fixpunkt und wird dort wie jede andere ausgewertet. Geräte: [Geräte → Für die iOS-App](geraete.md#für-die-ios-app).

### Was die App können soll

- **Ablauf:** Baustelle wählen → RTK-Status (Fix, Float, kein Fix) → scannen → hochladen
- **Scan:** ARKit mit Tiefendaten und Szenenrekonstruktion, Kamerapose je Bild
- **Rover:** liest die Position (NMEA) über Bluetooth – zwei austauschbare Anbindungen für Rover mit
  Bluetooth LE und für für das iPhone zertifizierte Rover (MFi), damit die App an keinen Hersteller gebunden ist
- **Korrekturdaten:** NTRIP-Client im iPhone holt SAPOS-Korrekturen übers Mobilfunknetz und gibt sie per
  Bluetooth an den Rover – der Rover braucht kein eigenes Internet
- **Synchron aufzeichnen:** ARKit-Pose, RTK-Position und Zeitstempel zusammen
- **Hebelarm:** Abstand zwischen Antenne und Kamera einmal kalibrieren
- **Passpunkte:** mit Lotstab einmessen
- **Offline:** Puffer bei Funkloch, Upload im Hintergrund (über die fortsetzbare Upload-Schnittstelle, die der
  Server schon hat)
- **Anmelden:** mit API-Token oder Passkey (beides hat der Server schon)

### Wie die Genauigkeit entsteht

ARKit allein driftet über längere Strecken. Deshalb richtet Fixpunkt den Scan über die ganze Strecke an den
RTK-Positionen aus (Helmert/Umeyama, wie bei den Passpunkten) und gleicht abschnittsweise gegen die Drift aus.
**Ziel: 2–3 cm bei Gräben bis 30–50 m.**

### Schritte

- [ ] Geräte beschaffen: Mac, iPhone Pro, Rover, Apple Developer Account
- [ ] Rover-Anbindung: Bluetooth LE und MFi, NMEA lesen, RTK-Status anzeigen
- [ ] Beim Rover-Hersteller die Freigabe der App für den App Store klären (MFi)
- [ ] NTRIP-Client für SAPOS
- [ ] LiDAR-Scan mit synchroner Pose- und RTK-Aufzeichnung
- [ ] Hebelarm-Kalibrierung, Passpunkt-Modus
- [ ] Upload mit Offline-Puffer
- [ ] Ausgleich über die Gesamttrajektorie im Server
- [ ] Testphase mit ersten Nutzern über TestFlight

Die reine Logik (NMEA, NTRIP, Datenformat) entsteht als Swift-Paket, das sich auch ohne iPhone testen lässt.

## Große Punktwolken im Browser

- [ ] Kacheln für sehr große Punktwolken (3D Tiles oder Potree) aus dem Rechenkern – heute zeigt die 3D-Ansicht
  höchstens eine Million Punkte, gleichmäßig ausgedünnt; für Baugruben reicht das, für große Scans nicht

## Später

- [ ] Missionen direkt an die Drohne schicken und Fotos automatisch empfangen (DJI Cloud API, selbst gehostet)
- [ ] Android: Scannen im Browser (PWA) mit Rover über Web Bluetooth
- [ ] Optional: leichterer, günstigerer RTK-Empfänger als Alternative zum Reach RX2
- [ ] Optional: eigener Handscanner (z. B. Livox Mid-360 mit FAST-LIO2)
