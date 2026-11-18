---
description: >-
  Funktionsumfang von Fixpunkt: Daten hochladen, Drohnenfotos auswerten, Flug planen, ansehen und messen,
  Aushub, Haufwerk, Soll/Ist, Massenziele und Lagerbestand, Abrechnung mit Lieferscheinen und GAEB X31, Dokumentation,
  Berichte, Export und Betrieb.
---

# Funktionsumfang

Alles, was Fixpunkt heute kann, auf einer Seite – mit Verweis auf die ausführliche Beschreibung. Was noch kommt,
steht in der [Roadmap](roadmap.md).

## Daten hereinholen

- **Punktwolken:** LAS, LAZ, E57 (Laserscanner), Text (XYZ, CSV, PTS) – mit Farben und Intensität, wo die Datei
  sie hat ([Daten vorbereiten](daten.md))
- **Höhenmodelle:** GeoTIFF (z. B. aus OpenDroneMap), ESRI ASCII Grid
- **Orthofoto** je Aufnahme als Hintergrund der Ansicht ([Ansicht](ansicht.md#orthofoto))
- **Drohnenfotos direkt auswerten** über NodeODM: Fotos hochladen, Fixpunkt holt Punktwolke und Orthofoto ab und
  rechnet sie in das System der Baustelle um ([Drohnenfotos auswerten](drohnenfotos.md))
- **Große Dateien:** der Upload setzt nach einem Abbruch an derselben Stelle fort
- **Flugdatum** je Aufnahme, nachträglich änderbar – Zeitreihe, Überblick und Abrechnung ordnen danach
  ([Aushub berechnen](aushub.md#1-aufnahmen-hochladen))
- **Passpunkte:** Aufnahmen im lokalen System über eingemessene Punkte georeferenzieren, mit Restklaffen in der
  Vorschau ([Berechnungen](berechnungen.md#passpunkte))
- **Abrechnungsgrenzen:** im Browser zeichnen oder als GeoJSON (z. B. aus QGIS) hochladen
- **Pläne:** DXF als Hintergrund, Höhenpunkte und Koten daraus ([Pläne](plaene.md))

## Fliegen

- **Flugplanung:** Kartierflug über einem Bereich aus Bodenauflösung und Überlappung, als KMZ für DJI Pilot 2
  ([Flugplanung](flugplanung.md))

## Ansehen und Messen

- **2D-Ansicht** mit Höhenbild oder Orthofoto, Differenz oder Soll/Ist-Abweichung darüber
- **3D** als Geländenetz oder Punktwolke, gefärbt nach Farbe, Intensität oder Höhe, mit Überhöhung
- **Messen:** Punkt, Strecke, Höhenunterschied, Gefälle, Fläche (horizontal und Oberfläche), Profil
- **Karte** aller Baustellen
- Bedienbar mit Maus, Tastatur und am Tablet ([Ansicht, 3D und Messen](ansicht.md))

## Massen

- **Aushub und Auftrag** zwischen zwei Aufnahmen, je Abrechnungsbereich ([Aushub berechnen](aushub.md))
- **Bodenfilter:** Bagger, Container und Bewuchs werden herausgerechnet
- **Haufwerk** gegen Basisebene, tiefsten Randpunkt oder feste Höhe
- **Zeitreihe** über die ganze Bauphase
- **Massenziele** je Bereich: Material, Soll-Menge, erledigt und offen
- **Lagerbestand** je Material aus den Haufwerken, mit Änderung und Verlauf über die Befliegungen
  ([Material und Massenziel](aushub.md#material-und-massenziel))

## Abrechnung

- **Aufmaß je Zeitraum** zwischen zwei Befliegungen, mit LV-Position, Einheitspreis und Betrag, als PDF
  ([Abrechnung](abrechnung.md))
- **Umrechnung** in Tonnen, m³ lose und LKW-Fahrten über Dichte, Auflockerung und Nutzlast
- **Schichten** (Homogenbereiche) nach Tiefe unter dem Urgelände, je Schicht Material und Preis
- **Lieferscheine** einzeln oder als CSV, mit Abgleich gegen die gemessene Menge
- **GAEB DA XML X31** (Mengenermittlung) je LV-Position für die Abrechnungssoftware
- **Freigabe** durch den Auftraggeber per Link ohne Anmeldung: anerkennen oder Einwand, im PDF festgehalten
- rechtlicher Rahmen: [Abrechnung und Recht](recht.md)

## Soll/Ist

- gegen **Sohlhöhe, geneigte Ebene** oder ein **Soll-Geländemodell**, mit Toleranz, Statistik, Histogramm und
  Restmassen ([Berechnungen](berechnungen.md#sollist))
- **Plan/Ist** der Höhenpunkte aus einem DXF-Plan
- **Zwangspunkte** mit Sollhöhe gegen das Gelände der letzten Aufnahme

## Dokumentieren

- **Zwangspunkte**, **Leitungen** (in der Ansicht nachgezeichnet, mit Höhen), **Notizen**
- **Fotos** mit GPS-Position und Blickrichtung aus dem Foto, als Sichtkegel in der Ansicht
  ([Dokumentation](dokumentation.md))

## Überblick und Berichte

- **Überblick** je Baustelle: Kennzahlen, Aushub je Befliegung, Massenziele, Lagerbestand und Verlauf, was
  Aufmerksamkeit braucht – von jedem selbst zusammengestellt ([Erste Schritte](erste-schritte.md#überblick-einer-baustelle))
- **PDF-Berichte:** Mengenbericht mit Karte, Haufwerk, Soll/Ist, Zeitreihe, Dokumentation mit Fotos, Plan/Ist und
  ein eigener Bericht aus Bausteinen
- **Aufmaßblatt** je Abrechnungszeitraum

## Export

- Höhenmodell als **GeoTIFF**, Höhenlinien und Grenzen als **DXF**, Oberfläche als **LandXML**, Rasterpunkte als
  **CSV**, Dreiecksnetz als **OBJ**, Orthofoto als **GeoTIFF** ([Berechnungen](berechnungen.md#export))
- Dokumentation (Zwangspunkte, Leitungen, Notizen, Fotos) als **DXF** für CAD
- Alle Koordinaten in ETRS89/UTM32 (EPSG:25832), Höhen in DHHN2016

## Team und Betrieb

- **Benutzer und Rollen:** Admin, Bearbeiten, Nur lesen; Anmeldung mit Passwort oder Passkey
  ([Benutzer und Rollen](benutzer.md))
- **API-Tokens** für Programme und Skripte
- **Live-Status:** Prüfung und Berechnungen aktualisieren sich von selbst
- **Selbst gehostet:** eine Programmdatei mit Web-Oberfläche und SQLite, Docker-Image für amd64 und arm64
  ([Installation](installation.md))
- auf Tablet und Handy installierbar, hell und dunkel, Deutsch und Englisch
- **Kommandozeile** für den Rechenkern ([Kommandozeile](kommandozeile.md))
