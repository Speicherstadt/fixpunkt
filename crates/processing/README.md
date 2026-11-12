# fixpunkt-processing

Rechen-Engine ohne App: Punktwolke (LAS/LAZ, z. B. aus OpenDroneMap, oder XYZ/CSV) → Höhenraster → Volumen.

```bash
# Aushub: Urgelände vs. Baugrube, nur innerhalb der Grenze
cargo run --release -- diff vorher.laz nachher.laz --boundary grube.geojson --cell 0.25 --diff-out diff.asc

# Haufwerk gegen Ausgleichsebene entlang der Grenze (oder: --base lowest | --base 312.4)
cargo run --release -- stockpile haufen.laz --boundary haufen.geojson --base plane
```

Ausgabe: JSON mit `cut_m3`, `fill_m3`, `net_m3`, `area_m2`. Die Grenze muss im selben metrischen
Koordinatensystem liegen (z. B. EPSG:25832); `diff.asc` lässt sich in QGIS öffnen.

Tests (`cargo test`) prüfen gegen exakt bekannte Volumen: Baugrube mit Böschung (Prismenformel) und Kegel auf Hang – Abweichung < 1 %.

Noch offen: DSM-GeoTIFF direkt lesen, TIN-Basisfläche, Anbindung an NodeODM/Web.
