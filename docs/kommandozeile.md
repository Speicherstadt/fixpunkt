---
description: >-
  Fixpunkt auf der Kommandozeile: diff, stockpile, series, compare, ground, clean, georef, coords, measure, profile,
  export – JSON-Ausgabe für Skripte, Exporte nach GeoTIFF, DXF, LandXML, OBJ, PLY, CSV.
---

# Kommandozeile

Dieselbe Programmdatei rechnet auch ohne Server. Jeder Befehl gibt sein Ergebnis als **JSON** aus (Fehler auf
stderr, Exit-Code ≠ 0) – gut für Skripte. Im Container: `docker run --rm -v "$PWD":/w -w /w tristanteu/fixpunkt diff …`.

```bash
# Überblick über eine Datei
fixpunkt info aufnahme.laz

# Aushub je Bereich, Bagger & Co. herausfiltern, Differenzraster schreiben
fixpunkt diff vorher.laz nachher.laz --boundary grenzen.geojson --classify-ground --diff-out diff.tif

# Haufwerk gegen Ausgleichsebene (oder --base lowest | --base 312.4)
fixpunkt stockpile haufen.laz --boundary haufen.geojson --base plane

# Fortschritt über die Bauphase
fixpunkt series urgelaende.laz kw10.laz kw12.laz --boundary grube.geojson --labels KW10,KW12

# Soll/Ist: Sohle 296,40 m ± 3 cm, geneigte Ebene (x,y,z,Gefälle %,Richtung °) oder Soll-DGM
fixpunkt compare nachher.laz --height 296.40 --tolerance 0.03 --boundary sohle.geojson --diff-out abw.tif
fixpunkt compare nachher.laz --slope 512000,5403000,296.4,2,90
fixpunkt compare nachher.laz --design soll_dgm.tif

# Messen und Profile
fixpunkt measure dgm.tif --at 512010.5,5403020.0 --line achse.geojson --area flaeche.geojson
fixpunkt profile nachher.laz --line achse.txt --step 0.5 --csv profil.csv

# Aufbereiten
fixpunkt clean roh.laz sauber.laz --neighbors 8 --std-ratio 2.5 --voxel 0.05
fixpunkt ground sauber.laz boden.laz --dtm-out dtm.tif
fixpunkt dem boden.laz -o dgm.tif --ground-only --cell 0.25

# Georeferenzierung über Passpunkte (CSV: name, x,y,z Quelle, x,y,z Ziel)
fixpunkt georef --gcp passpunkte.csv --mode similarity scan_lokal.laz scan_utm.laz

# Koordinaten: lat lon h → E N H (DHHN2016 mit Quasigeoid GCG2016), oder --inverse
fixpunkt coords punkte.txt --geoid GCG2016.txt

# Exporte für CAD und Vermessung
fixpunkt export boden.laz --ground-only --boundary grenze.geojson \
  --tif dgm.tif --dxf dgm.dxf --contours 0.5 --major 2.5 --landxml dgm.xml --obj dgm.obj --ply dgm.ply --csv dgm.csv
```

Rasteroptionen gelten überall: `--cell`, `--ground-only`, `--classify-ground`, `--aggregate mean|min|max`, `--fill`.
`--epsg` (Standard 25832) gilt für alle Ausgaben. `fixpunkt <befehl> --help` zeigt alle Optionen.

| Export | Inhalt |
|---|---|
| GeoTIFF, ASC | Höhenraster, mit EPSG-Code |
| DXF (R12) | Höhenlinien (Haupt- und Zwischenlinien), Grenzen, optional DGM als 3D-Flächen |
| LandXML 1.2 | TIN-Oberfläche für Civil-Programme |
| OBJ, PLY | Dreiecksnetz; `--local` schreibt relativ zu einem Offset (für Programme mit float32) |
| CSV | Rasterpunkte `x;y;z`, Profile `station;x;y;z` |
