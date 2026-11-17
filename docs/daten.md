---
description: >-
  Daten für Fixpunkt vorbereiten: Punktwolken und Höhenmodelle aus OpenDroneMap, Koordinatensystem ETRS89/UTM32,
  Höhen DHHN2016, Abrechnungsgrenzen als GeoJSON aus QGIS.
---

# Daten vorbereiten

## Dateiformate

| Art | Formate | Hinweise |
|---|---|---|
| Punktwolke | `.las`, `.laz` | auch klassifiziert (Klasse 2 = Boden); Farben und Attribute bleiben erhalten |
| Punktwolke als Text | `.xyz`, `.txt`, `.csv`, `.pts` | Spalten `x y z`, Trenner Leerzeichen, Tab, Komma oder Semikolon; Kopfzeile erlaubt |
| Höhenmodell | `.tif`, `.tiff` (GeoTIFF), `.asc` | einkanalig, quadratische Zellen; LZW/Deflate und Kacheln werden gelesen |

## Aus OpenDroneMap

| ODM-Ergebnis | Datei | Wofür |
|---|---|---|
| Punktwolke | `odm_georeferencing/odm_georeferenced_model.laz` | beste Wahl; Fixpunkt filtert den Boden selbst |
| Geländemodell (DTM) | `odm_dem/dtm.tif` | schon ohne Objekte; Bodenfilter in Fixpunkt ausschalten |
| Oberflächenmodell (DSM) | `odm_dem/dsm.tif` | mit Objekten; für Haufwerke |

Für georeferenzierte Ergebnisse braucht ODM Fotos mit RTK-Position (z. B. DJI Matrice 4E mit SAPOS) oder
Passpunkte. Das Ausgabesystem in ODM auf **EPSG:25832** stellen (`--proj` bzw. in WebODM unter den Optionen).

## Koordinaten und Höhen

- **Lage:** ETRS89/UTM Zone 32 (EPSG:25832). Beide Aufnahmen einer Berechnung müssen im selben System sein – sonst
  überlappen sie nicht, und Fixpunkt meldet einen leeren Bereich.
- **Höhen:** Für die Massen zählt nur, dass beide Aufnahmen dieselbe Höhenbezugsfläche haben. Drohnen-RTK liefert
  ellipsoidische Höhen; für Normalhöhen (DHHN2016) rechnet `fixpunkt coords --geoid GCG2016.txt` um
  ([Kommandozeile](kommandozeile.md)).

> [!NOTE]
> Web-Karten nutzen WGS84, SAPOS und amtliche Daten ETRS89. In Deutschland liegen beide heute rund 0,8–1 m
> auseinander. Für Abrechnungsgrenzen ist das meist unerheblich, für einzelne Messpunkte nicht.

## Abrechnungsgrenzen aus QGIS

1. Neuen Polygon-Layer im Projekt-KBS **EPSG:25832** anlegen, Feld `name` hinzufügen.
2. Bereiche zeichnen (auf Orthofoto oder Lageplan) und benennen.
3. Layer → **Exportieren → Objekte speichern als… → GeoJSON**, KBS EPSG:25832.

QGIS schreibt das Koordinatensystem in die Datei (`crs`), Fixpunkt übernimmt die Koordinaten dann unverändert.
GeoJSON in Länge/Breite (z. B. von geojson.io) rechnet Fixpunkt nach UTM32 um. Löcher in Polygonen
(z. B. ein ausgesparter Kran) werden berücksichtigt.
