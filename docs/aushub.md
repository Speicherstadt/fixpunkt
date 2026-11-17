---
description: >-
  Aushub mit Fixpunkt berechnen: Aufnahmen hochladen, Abrechnungsgrenzen setzen, zwei Aufnahmen vergleichen,
  Mengenbericht lesen und als CSV, GeoTIFF oder Ausdruck weitergeben.
---

# Aushub berechnen

Der Ablauf für eine Baugrube: **Urgelände fliegen → Aushub fliegen → beide hochladen → Grenzen setzen → berechnen.**

![Baustelle mit drei Aufnahmen, zwei Bereichen und einer fertigen Berechnung](screenshot-baustelle.png#only-light){ width="1440" height="959" }
![Baustelle mit drei Aufnahmen, zwei Bereichen und einer fertigen Berechnung](screenshot-baustelle-dunkel.png#only-dark){ width="1440" height="959" }

## 1. Aufnahmen hochladen

**Aufnahme hochladen** nimmt eine oder mehrere Dateien: Punktwolken (LAS, LAZ, XYZ, CSV, TXT, PTS) oder
Höhenmodelle (GeoTIFF, ASC). Der Name der Aufnahme kommt aus dem Dateinamen.

Nach dem Hochladen prüft Fixpunkt jede Datei im Hintergrund – Status **Wird geprüft**, dann **Bereit** mit Anzahl
der Punkte. Eine Datei, die sich nicht lesen lässt, steht auf **Fehlgeschlagen** mit dem Grund. Die Liste
aktualisiert sich von selbst, auch für Kolleginnen und Kollegen, die die Baustelle gerade offen haben.

> [!TIP]
> Aus OpenDroneMap: die georeferenzierte Punktwolke `odm_georeferencing/odm_georeferenced_model.laz` oder das
> Geländemodell `odm_dem/dtm.tif`. Mehr unter [Daten vorbereiten](daten.md).

## 2. Abrechnungsgrenzen

Bereiche lassen sich direkt in der [Ansicht](ansicht.md#bereiche-zeichnen) zeichnen oder als GeoJSON hochladen:
**Grenze hochladen** nimmt eine GeoJSON-Datei. Jedes Polygon darin wird ein eigener Bereich, mit dem Namen aus
der Eigenschaft `name` (oder `Bezeichnung`, `id`). So lässt sich eine Baugrube in Abschnitte teilen, die einzeln
abgerechnet werden. Ohne Bereich rechnet Fixpunkt über die ganze Überlappung der beiden Aufnahmen.

## 3. Berechnen

Unter **Berechnungen**:

- **Urgelände (vorher)** und **Nach dem Aushub** wählen – vorausgewählt sind die älteste und die jüngste Aufnahme.
- **Rasterweite:** 25 cm passt für Drohnenflüge; 10 cm für sehr dichte Wolken, 50 cm–1 m für schnelle Übersichten.
- **Bagger, Container und Bewuchs herausfiltern:** an lassen, außer die Aufnahme ist schon ein reines Geländemodell (DTM).
- **Abrechnungsbereiche** an- oder abwählen.

**Aushub berechnen** reiht die Berechnung ein; sie läuft im Hintergrund, die Liste zeigt den Stand.

## 4. Mengenbericht

![Mengenbericht: Abtrag, Auftrag, Bilanz und Tabelle je Bereich](screenshot-bericht.png#only-light){ width="1440" height="900" }
![Mengenbericht: Abtrag, Auftrag, Bilanz und Tabelle je Bereich](screenshot-bericht-dunkel.png#only-dark){ width="1440" height="900" }

| Wert | Bedeutung |
|---|---|
| **Abtrag** | Volumen, um das das Gelände tiefer liegt als vorher (Aushub) |
| **Auftrag** | Volumen, um das es höher liegt (Anschüttung, Haufwerk) |
| **Bilanz** | Auftrag − Abtrag; bei reinem Aushub negativ |
| **Fläche** | Fläche des Abrechnungsbereichs |

Fehlen in einem Bereich für mehr als 5 % der Fläche Daten, warnt der Bericht: diese Stellen sind **nicht**
mitgerechnet. Meist deckt dann eine der Aufnahmen den Bereich nicht ganz ab.

Weitergeben:

- **Als CSV** – Tabelle für Excel (Semikolon, Dezimalkomma)
- **Differenzraster (GeoTIFF)** – Höhenänderung je Rasterzelle, zum Ansehen in QGIS oder im CAD
- **Drucken** – der Bericht als PDF über den Druckdialog des Browsers

Haufwerke, Soll/Ist-Vergleiche, Zeitreihen und Exporte stehen unter
[Haufwerk, Soll/Ist, Zeitreihe, Export](berechnungen.md).

> [!WARNING]
> Für die Abrechnung den Bericht mit Lieferscheinen, LKW-Ladungen oder einem Aufmaß plausibilisieren, bis die
> Genauigkeit auf euren Baustellen belegt ist ([Methode und Genauigkeit](methode.md)).
