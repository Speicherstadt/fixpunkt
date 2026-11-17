/**
 * Dreiecksnetz für die 3D-Ansicht aus dem Höhenraster. Koordinaten relativ zur Mitte (sonst frisst Float32 bei
 * Rechtswerten um 500 000 m die Zentimeter), y nach oben: x = Ost, y = Höhe × Überhöhung, z = Süd.
 * Zellen ohne Daten bleiben Löcher; große Raster werden ausgedünnt.
 */
import type { HeightGrid } from "./grid";

/** längste Seite in Stützpunkten (≈ 160 000 Punkte, flüssig auch auf Tablets) */
export const MAX_SIDE = 400;

export type Rgb = [number, number, number];

export type TerrainMesh = {
  positions: Float32Array;
  colors: Float32Array;
  indices: Uint32Array;
  /** Ausdehnung in m: Breite (Ost), Tiefe (Nord), Höhe (schon überhöht) */
  size: [number, number, number];
  /** Höhe der Unterkante (z = 0 im Netz) */
  zMin: number;
  triangles: number;
};

/** Farbe je Stützpunkt aus Lage (UTM) und Höhe; 0…255 */
export type ColorAt = (x: number, y: number, z: number) => Rgb;

export const buildMesh = (g: HeightGrid, colorAt: ColorAt, exaggeration = 1): TerrainMesh => {
  const step = Math.max(1, Math.ceil(Math.max(g.cols, g.rows) / MAX_SIDE));
  const cols = Math.ceil(g.cols / step);
  const rows = Math.ceil(g.rows / step);
  const at = (c: number, r: number) =>
    g.data[Math.min(r * step, g.rows - 1) * g.cols + Math.min(c * step, g.cols - 1)]!;
  let zMin = Number.POSITIVE_INFINITY;
  let zMax = Number.NEGATIVE_INFINITY;
  for (let r = 0; r < rows; r++)
    for (let c = 0; c < cols; c++) {
      const z = at(c, r);
      if (Number.isNaN(z)) continue;
      zMin = Math.min(zMin, z);
      zMax = Math.max(zMax, z);
    }
  if (!Number.isFinite(zMin)) zMin = zMax = 0;
  const width = cols * step * g.cell;
  const depth = rows * step * g.cell;
  const positions = new Float32Array(cols * rows * 3);
  const colors = new Float32Array(cols * rows * 3);
  for (let r = 0; r < rows; r++)
    for (let c = 0; c < cols; c++) {
      const i = r * cols + c;
      const x = g.minX + (Math.min(c * step, g.cols - 1) + 0.5) * g.cell;
      const y = g.maxY - (Math.min(r * step, g.rows - 1) + 0.5) * g.cell;
      const z = at(c, r);
      const h = Number.isNaN(z) ? 0 : (z - zMin) * exaggeration;
      positions.set([x - g.minX - width / 2, h, g.maxY - y - depth / 2], i * 3);
      if (!Number.isNaN(z))
        colors.set(
          colorAt(x, y, z).map((v) => v / 255),
          i * 3,
        );
    }
  const indices: number[] = [];
  const valid = (c: number, r: number) => !Number.isNaN(at(c, r));
  for (let r = 0; r + 1 < rows; r++)
    for (let c = 0; c + 1 < cols; c++) {
      if (!(valid(c, r) && valid(c + 1, r) && valid(c, r + 1) && valid(c + 1, r + 1))) continue;
      const i00 = r * cols + c;
      const i01 = i00 + cols; // südlich
      const i10 = i00 + 1; // östlich
      const i11 = i01 + 1;
      // gegen den Uhrzeigersinn von oben gesehen: Normalen zeigen nach oben
      indices.push(i00, i01, i10, i10, i01, i11);
    }
  return {
    positions,
    colors,
    indices: Uint32Array.from(indices),
    size: [width, depth, (zMax - zMin) * exaggeration],
    zMin,
    triangles: indices.length / 3,
  };
};
