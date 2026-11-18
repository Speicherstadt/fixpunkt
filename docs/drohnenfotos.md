---
description: >-
  Drohnenfotos direkt in Fixpunkt auswerten: Fotos eines Flugs hochladen, NodeODM (OpenDroneMap) rechnet die
  Punktwolke, Fixpunkt rechnet sie in die UTM-Zone der Baustelle um und legt sie als Aufnahme an.
---

# Drohnenfotos auswerten

Fixpunkt rechnet selbst keine Photogrammetrie, gibt die Fotos aber an
[NodeODM](https://github.com/OpenDroneMap/NodeODM) weiter, den Rechendienst von OpenDroneMap. Ist der Flug
ausgewertet, holt Fixpunkt die Punktwolke ab, bringt sie in das Bezugssystem der Baustelle und legt sie als neue
**Aufnahme** an – geprüft und bereit für die Berechnung.

Den Flug selbst plant Fixpunkt auf Wunsch mit: [Flugplanung](flugplanung.md) erzeugt aus einem Bereich eine
Mission für DJI Pilot 2.

![Abschnitt „Drohnenfotos auswerten“: 40 Fotos ausgewählt, Name, Flugdatum, Qualität und Höhenversatz](screenshot-drohnenfotos.pngNone){ width="1024" height="454" }

## NodeODM einrichten

NodeODM läuft als eigener Container, am einfachsten neben Fixpunkt:

```yaml
services:
  fixpunkt:
    image: tristanteu/fixpunkt:latest
    restart: unless-stopped
    ports:
      - "8080:8080"
    volumes:
      - fixpunkt:/data
    environment:
      FIXPUNKT_NODEODM_URL: http://nodeodm:3000
  nodeodm:
    image: opendronemap/nodeodm:latest
    restart: unless-stopped
volumes:
  fixpunkt:
```

Ein vorhandener NodeODM (z. B. der Knoten einer WebODM-Installation oder ein Rechner im Büro) geht genauso:
`FIXPUNKT_NODEODM_URL` auf dessen Adresse setzen, bei einem Token zusätzlich `FIXPUNKT_NODEODM_TOKEN`
([Konfiguration](konfiguration.md)). Ohne NodeODM zeigt der Abschnitt nur einen Hinweis; Punktwolken aus WebODM
lassen sich weiterhin als Aufnahme hochladen ([Daten vorbereiten](daten.md)).

> [!NOTE]
> Photogrammetrie braucht Speicher: für 200 Fotos mit 20 Megapixeln rechnet man mit 16 GB RAM und einer
> Stunde Rechenzeit in der Qualität „Standard“. Fixpunkt selbst bleibt dabei bedienbar – Flüge haben eine eigene
> Warteschlange und blockieren keine Berechnungen.

## Ablauf

1. Im Reiter **Aufnahmen** der Baustelle **Drohnenfotos auswerten** aufklappen und die Fotos des Flugs wählen (JPG, TIFF oder PNG,
   mindestens 3).
2. **Name** und **Flugdatum** der neuen Aufnahme, **Qualität** und **Höhenversatz** eintragen (siehe unten).
3. **Hochladen und auswerten**: Fixpunkt lädt die Fotos nacheinander hoch und gibt sie an NodeODM. Im Reiter
   **Massen** unter **Berechnungen** steht der Auftrag mit Fortschritt in Prozent.
4. Ist NodeODM fertig, erscheint die Punktwolke im Reiter **Aufnahmen** und wird wie jede Aufnahme geprüft. Das
   Orthofoto des Flugs hängt Fixpunkt gleich mit an; es liegt dann als Hintergrund in der [Ansicht](ansicht.md#orthofoto).

| Qualität | ODM-Einstellung | Wofür |
|---|---|---|
| Schnell (Vorschau) | `pc-quality low`, `feature-quality medium` | prüfen, ob der Flug passt |
| Standard | `pc-quality medium`, `feature-quality high` | Massenermittlung |
| Hoch (langsam) | `pc-quality high`, `feature-quality high` | feine Kanten, kleine Flächen |

Startet der Server während der Auswertung neu, fragt Fixpunkt danach dieselbe Aufgabe in NodeODM weiter ab, ohne
die Fotos noch einmal hochzuladen. Scheitert NodeODM, steht seine Meldung mit den letzten Zeilen der Ausgabe am
Auftrag. Die Fotos bleiben im Auftrag, bis er gelöscht wird; in NodeODM räumt Fixpunkt die Aufgabe danach ab.

## Koordinaten und Höhen

- **Lage:** ODM rechnet in der UTM-Zone, in der die Fotos liegen – östlich von 12° Ost ist das Zone 33.
  Fixpunkt liest das Bezugssystem aus der Punktwolke und rechnet sie millimetergenau in die Zone der Baustelle
  um (Zone 32: EPSG:25832). WGS84 und ETRS89 gelten dabei als gleich; mit RTK über SAPOS sind die Koordinaten
  ohnehin ETRS89.
- **Höhen:** Die Drohne speichert in der Regel ellipsoidische Höhen. Für Normalhöhen im DHHN2016 die
  **Höhenanomalie** der Baustelle als Höhenversatz eintragen – in Deutschland zwischen 36 und 50 m; den Wert
  liefert `fixpunkt coords --geoid GCG2016.txt` ([Kommandozeile](kommandozeile.md)). Auf einer Baustelle ändert
  er sich nur um Millimeter, ein Wert genügt.

> [!TIP]
> Probe mit einem Punkt bekannter Höhe (Kanaldeckel, Höhenbolzen): in der [Ansicht](ansicht.md) mit **Punkt**
> messen und vergleichen. Für Massen zwischen zwei Flügen zählt vor allem, dass beide denselben Höhenversatz
> haben.

Ohne RTK liegt die Drohne einige Meter daneben; dann bringen [Passpunkte](berechnungen.md#passpunkte) die
Punktwolke an die richtige Stelle.
