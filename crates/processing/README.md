# fixpunkt-processing

Rechenkern von Fixpunkt – ohne App, ohne Server, ohne PROJ/PDAL/GDAL. Reines Rust, eine Binärdatei.
Jeder Befehl gibt sein Ergebnis als **JSON auf stdout** aus (Fehler auf stderr, Exit-Code ≠ 0),
damit der Worker der Web-App ihn direkt aufrufen kann.

**Eingaben:** LAS/LAZ (z. B. OpenDroneMap `odm_georeferencing/odm_georeferenced_model.laz`), XYZ/CSV/TXT,
GeoTIFF (z. B. `odm_dem/dsm.tif`, `dtm.tif` – auch LZW/Deflate, gekachelt), ESRI-ASCII-Grid.
**Grenzen/Linien:** GeoJSON – metrisch (EPSG:25832, QGIS-Export mit `crs`) oder Länge/Breite (wird nach UTM32 umgerechnet).

## Befehle

```bash
# Überblick
fixpunkt info aufnahme.laz

# Aushub: Urgelände vs. Baugrube, je Bereich (mehrere Polygone im GeoJSON), Bagger & Co. herausfiltern
fixpunkt diff vorher.laz nachher.laz --boundary grenzen.geojson --classify-ground --diff-out diff.tif

# Haufwerk gegen Ausgleichsebene entlang der Grenze (oder: --base lowest | --base 312.4)
fixpunkt stockpile haufen.laz --boundary haufen.geojson --base plane

# Fortschritt über die Bauphase
fixpunkt series urgelaende.laz kw10.laz kw12.laz kw14.laz --boundary grube.geojson

# Soll/Ist: Sohle 296,40 m ± 3 cm, oder geneigt (x,y,z,Gefälle %,Richtung °), oder gegen Soll-DGM
fixpunkt compare nachher.laz --height 296.40 --tolerance 0.03 --boundary sohle.geojson --diff-out abw.tif
fixpunkt compare nachher.laz --slope 512000,5403000,296.4,2,90
fixpunkt compare nachher.laz --design soll_dgm.tif

# Messen und Profile
fixpunkt measure dgm.tif --at 512010.5,5403020.0 --line achse.geojson --area flaeche.geojson
fixpunkt profile nachher.laz --line achse.txt --step 0.5 --csv profil.csv

# Aufbereiten
fixpunkt clean roh.laz sauber.laz --neighbors 8 --std-ratio 2.5 --voxel 0.05   # Ausreißer, Ausdünnen
fixpunkt ground sauber.laz boden.laz --dtm-out dtm.tif                          # Boden klassifizieren (SMRF)
fixpunkt dem boden.laz -o dgm.tif --ground-only --cell 0.25

# Georeferenzierung über Passpunkte (Helmert 7 Par. | rigid | translation), Restklaffen im JSON
fixpunkt georef --gcp passpunkte.csv --mode similarity scan_lokal.laz scan_utm.laz

# Koordinaten: lat lon h → E N H (DHHN2016 mit Quasigeoid GCG2016), oder --inverse
fixpunkt coords punkte.txt --geoid GCG2016.txt

# Exporte für CAD/Vermessung
fixpunkt export boden.laz --ground-only --boundary grenze.geojson \
  --tif dgm.tif --dxf dgm.dxf --contours 0.5 --major 2.5 --landxml dgm.xml --obj dgm.obj --ply dgm.ply --csv dgm.csv
```

`--epsg` (Standard 25832) gilt für alle Ausgaben. Rasteroptionen überall gleich: `--cell`, `--ground-only`,
`--classify-ground`, `--aggregate mean|min|max`, `--fill`.

## Module

| Modul | Inhalt |
|---|---|
| `pointcloud` | LAS/LAZ/XYZ lesen und schreiben, LAS umschreiben mit allen Attributen (Farbe, Intensität …) |
| `grid` | Höhenraster: Rastern, Lücken füllen, bilinear abtasten, umrastern, ASC |
| `geotiff` | GeoTIFF lesen (ODM) und schreiben (Float32, Deflate, EPSG) |
| `surface` | Oberfläche aus Punktwolke oder Raster auf ein gemeinsames Gitter laden |
| `filter` | Ausreißer (statistisch, kNN), Voxel-Ausdünnung, Bodenfilter SMRF, Gefälleraster |
| `volume` | Aushub/Auftrag, je Bereich, Zeitreihe, Haufwerk gegen Ebene/Tiefpunkt/Höhe |
| `compare` | Soll/Ist gegen Höhe, geneigte Ebene, Soll-DGM; Statistik, Toleranz, Histogramm |
| `measure` | Punkthöhe, Strecke 2D/3D, Gefälle, Profil, Fläche horizontal/Oberfläche |
| `transform` | Passpunkt-Transformation (Umeyama/Helmert), Restklaffen |
| `crs` | ETRS89 ↔ UTM (Krüger-Reihen 6. Ordnung), Quasigeoid (GRAVSOFT- oder Spaltenformat) |
| `export` | Höhenlinien (Marching Squares), Mesh/TIN, DXF R12, LandXML 1.2, OBJ, PLY, CSV |
| `flightplan` | Kartierflug: Flughöhe aus GSD, Bahnen über einem Bereich, Foto- und Bahnabstand, Flugzeit |
| `wpml` | Flugplan als KMZ (DJI WPML) für DJI Pilot 2 |
| `geom` | Polygon, BBox, GeoJSON (mehrere benannte Bereiche, Linien) |

## Tests

```bash
cargo test            # 40 Unit-Tests + 3 End-to-End-Tests der Kommandozeile
```

Die Tests prüfen gegen unabhängig bekannte Ergebnisse: Pyramidenstumpf- und Kegelvolumen, UTM gegen numerisch
integrierte Meridianbogenlänge und Konformität, Helmert mit vorgegebenen Parametern, Bodenfilter mit Bagger auf
geneigtem Gelände, Höhenlinien eines Kegels (Kreisumfang), Rundreisen für LAS/LAZ/GeoTIFF/ASC.
`tests/cli.rs` baut eine komplette synthetische Baustelle (Grube, Bagger, Zeitreihe, Passpunkte) und ruft jeden Befehl auf.

**Plausibilität (Release-Build, 4 Kerne):** 4 Mio. Punkte, 200 × 200 m, 2 cm Rauschen, Grube 25 600 m³ + Bagger →
`diff --classify-ground` liefert netto −25 600,96 m³ in 3,7 s; Bodenfilter 2,6 s, Ausreißerfilter 6 s.

## Hinweise zur Genauigkeit

- **Netto zählt:** Abtrag und Auftrag einzeln enthalten das Messrauschen beider Aufnahmen (bei 2 cm Rauschen auf
  4 ha je ca. 150 m³). Die Nettomasse ist davon frei. Für saubere Einzelwerte: `--aggregate`/`--cell` größer, `clean` vorher.
- **ETRS89 ≠ WGS84:** Grenzen aus Web-Karten (WGS84) liegen in Deutschland ca. 0,8–1 m neben ETRS89 – für
  Abrechnungsgrenzen meist egal, für Messpunkte nicht.
- **Höhen:** Drohnen-RTK und SAPOS liefern ellipsoidische Höhen. Für DHHN2016 das Quasigeoid GCG2016 (BKG) mit
  `coords --geoid` verwenden bzw. die Punktwolke vorab umrechnen.
- `georef` aktualisiert die CRS-Angabe im LAS-Header nicht – das Zielsystem ist das der Passpunkte.

## Nicht im Rechenkern

Im Server (`crates/server`): NodeODM-Anbindung, PDF-Berichte, DXF-Pläne. Noch offen
([Roadmap](https://speicherstadt.github.io/fixpunkt/roadmap.html)): Kacheln für sehr große Punktwolken im
Browser und der abschnittsweise Ausgleich der iPhone-Trajektorie für die iOS-App.
