/**
 * Testdaten: synthetische Baustelle in ETRS89/UTM32 – Urgelände mit 1 % Gefälle, danach eine Grube
 * 20 × 20 × 2 m (= 800 m³) und ein Bagger (5 × 3 m, 3,5 m hoch), den der Bodenfilter entfernen muss.
 */
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export const X0 = 512_000;
export const Y0 = 5_403_000;
export const PIT_M3 = 800;

const inPit = (dx: number, dy: number) => dx >= 10 && dx < 30 && dy >= 10 && dy < 30;
const onExcavator = (dx: number, dy: number) => dx >= 33 && dx < 38 && dy >= 2 && dy < 5;

/** `east`/`north`: Verschiebung in m, damit Baustellen auf der Karte nicht übereinanderliegen */
export const terrainXyz = (after: boolean, east = 0, north = 0): string => {
  const lines = ["x y z"];
  for (let j = 0; j < 200; j++) {
    for (let i = 0; i < 200; i++) {
      const dx = (i + 0.5) * 0.2;
      const dy = (j + 0.5) * 0.2;
      let z = 300 + 0.01 * dx;
      if (after && inPit(dx, dy)) z -= 2;
      if (after && onExcavator(dx, dy)) z += 3.5;
      lines.push(`${(X0 + east + dx).toFixed(3)} ${(Y0 + north + dy).toFixed(3)} ${z.toFixed(3)}`);
    }
  }
  return lines.join("\n");
};

const rect = (name: string, x0: number, y0: number, x1: number, y1: number) => ({
  type: "Feature",
  properties: { name },
  geometry: {
    type: "Polygon",
    coordinates: [
      [
        [X0 + x0, Y0 + y0],
        [X0 + x1, Y0 + y0],
        [X0 + x1, Y0 + y1],
        [X0 + x0, Y0 + y1],
        [X0 + x0, Y0 + y0],
      ],
    ],
  },
});

/** Grube und ein Bereich um den Bagger: dort darf nach dem Filter kein Auftrag übrig bleiben */
export const boundaries = () => ({
  type: "FeatureCollection",
  crs: { type: "name", properties: { name: "urn:ogc:def:crs:EPSG::25832" } },
  features: [rect("Grube", 5, 5, 35, 35), rect("Bagger", 31, 0.5, 39.5, 7)],
});

/** Schreibt die Dateien in ein Verzeichnis und gibt die Pfade zurück */
export const writeFixtures = (dir: string) => {
  mkdirSync(dir, { recursive: true });
  const files = {
    before: join(dir, "Urgelaende.xyz"),
    after: join(dir, "Nach Aushub.xyz"),
    regions: join(dir, "grenzen.geojson"),
  };
  writeFileSync(files.before, terrainXyz(false));
  writeFileSync(files.after, terrainXyz(true));
  writeFileSync(files.regions, JSON.stringify(boundaries()));
  return files;
};
