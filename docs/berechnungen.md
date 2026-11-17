---
description: >-
  Weitere Berechnungen in Fixpunkt: Haufwerk gegen Basisfläche, Soll/Ist gegen Sohlhöhe, geneigte Ebene oder
  Soll-Geländemodell, Zeitreihe über die Bauphase, Export nach GeoTIFF, DXF, LandXML, CSV und OBJ.
---

# Haufwerk, Soll/Ist, Zeitreihe, Export

Neben dem [Aushub](aushub.md) rechnet Fixpunkt unter **Berechnungen** vier weitere Arten. Die Art wählt man oben
im Bereich; jede hat ihr eigenes Formular, das Ergebnis öffnet sich aus der Liste darunter.

## Haufwerk

Volumen eines Haufens (Oberboden-Miete, Schotter, Aushub-Zwischenlager) aus **einer** Aufnahme:

1. Um jeden Haufen einen Bereich legen – die Grenze knapp außerhalb des Haufenfußes.
2. **Haufwerk**, Aufnahme und Bereiche wählen.
3. **Basisfläche:**
    - *Ebene durch die Grenze* – Ausgleichsebene durch die Geländehöhen entlang der Grenze; richtig für Haufen
      auf geneigtem Gelände
    - *Tiefster Punkt der Grenze* – waagrechte Basis; rechnet eher zu viel
    - *Feste Höhe* – z. B. die bekannte Höhe einer Lagerfläche

Der Bericht zeigt je Bereich das **Volumen** über der Basis und, falls vorhanden, was darunter liegt
(Mulden, Fahrspuren).

> [!TIP]
> Den Bodenfilter beim Haufwerk ausschalten: er würde den Haufen sonst teilweise als „Objekt“ abtragen.

## Soll/Ist

Prüft, ob die Sohle, ein Planum oder eine Böschung stimmt:

![Soll/Ist-Vergleich einer Baugrubensohle: 63 % in Toleranz ±5 cm, Verteilung der Abweichung, Restmassen](screenshot-soll-ist.png#only-light){ width="1440" height="900" }
![Soll/Ist-Vergleich einer Baugrubensohle: 63 % in Toleranz ±5 cm, Verteilung der Abweichung, Restmassen](screenshot-soll-ist-dunkel.png#only-dark){ width="1440" height="900" }

| Soll | Eingabe |
|---|---|
| **Waagrechte Sohle** | Sollhöhe in m (z. B. 296,40) |
| **Geneigte Ebene** | Bezugspunkt (Ost-, Nordwert, Höhe), Gefälle in %, Richtung, in die sie fällt (0° = Nord, 90° = Ost) |
| **Soll-Geländemodell** | eine zweite Aufnahme, z. B. das Planungs-DGM als GeoTIFF |

Dazu einen **Bereich** (am besten nur die Sohle, nicht den Grubenrand) und die **Toleranz** in cm.

Der Bericht zeigt den Anteil der Fläche **in Toleranz**, **zu hoch** und **zu tief**, die Verteilung der
Abweichung und die **Restmassen**: was noch abzutragen und was aufzufüllen ist. Das Abweichungsraster
(GeoTIFF) zeigt in QGIS, *wo* nachgearbeitet werden muss.

## Zeitreihe

Fortschritt über die Bauphase: jede Aufnahme gegen das Urgelände und gegen die vorige. Fixpunkt ordnet die
Aufnahmen nach ihrem Datum, die älteste ist das Urgelände. Der Bericht zeigt je Befliegung den Abtrag seit Beginn
und seit der letzten Befliegung – passend zu Abschlagsrechnungen.

## Export

Für CAD und Vermessung aus einer Aufnahme, optional nur innerhalb von Bereichen:

| Format | Inhalt |
|---|---|
| Höhenmodell (GeoTIFF) | Raster mit EPSG-Code, für QGIS und GIS |
| Höhenlinien und Grenzen (DXF) | Haupt- und Zwischenlinien im gewählten Abstand, Bereichsgrenzen |
| Oberfläche als TIN (LandXML) | für Civil-Programme (Civil 3D, Card/1, BricsCAD) |
| Rasterpunkte (CSV) | `x;y;z` |
| Dreiecksnetz (OBJ) | für 3D-Programme, relativ zu einem Bezugspunkt |

Große Flächen werden für LandXML, CSV und OBJ ausgedünnt (höchstens 250.000 Punkte), damit CAD-Programme sie
öffnen. Alle Dateien liegen im Koordinatensystem der Baustelle (EPSG:25832).
