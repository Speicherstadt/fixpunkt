/** Farben der 2D-Ansicht: Höhen mit Schummerung, Abweichungen blau – weiß – rot. */
import { type HeightGrid, sampleAt } from "./grid";

type Rgb = [number, number, number];

/** Höhenstufen von tief nach hoch (blau – grün – gelb – braun – hell), gut unterscheidbar */
const TERRAIN: Rgb[] = [
  [38, 70, 166],
  [48, 140, 120],
  [140, 190, 90],
  [235, 210, 110],
  [180, 120, 70],
  [240, 235, 230],
];

const DIVERGING: Rgb[] = [
  [29, 78, 216],
  [147, 197, 253],
  [245, 245, 245],
  [253, 186, 116],
  [180, 83, 9],
];

const ramp = (stops: Rgb[], t: number): Rgb => {
  const u = Math.min(Math.max(t, 0), 1) * (stops.length - 1);
  const i = Math.min(Math.floor(u), stops.length - 2);
  const f = u - i;
  const a = stops[i]!;
  const b = stops[i + 1]!;
  return [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f];
};

export const heightColor = (t: number): Rgb => ramp(TERRAIN, t);

/** Abweichung: −limit blau, 0 weiß, +limit rot (Abtrag = Gelände tiefer = blau) */
export const deviationColor = (v: number, limit: number): Rgb =>
  ramp(DIVERGING, 0.5 + v / (2 * limit));

/** Schummerung aus Nordwest (Licht 315°, 45° hoch), 0…1 */
export const hillshade = (g: HeightGrid, c: number, r: number): number => {
  const z = (cc: number, rr: number) => {
    const v =
      g.data[
        Math.min(Math.max(rr, 0), g.rows - 1) * g.cols + Math.min(Math.max(cc, 0), g.cols - 1)
      ]!;
    return Number.isNaN(v) ? g.data[r * g.cols + c]! : v;
  };
  const dzdx = (z(c + 1, r) - z(c - 1, r)) / (2 * g.cell);
  const dzdy = (z(c, r - 1) - z(c, r + 1)) / (2 * g.cell);
  const slope = Math.atan(Math.hypot(dzdx, dzdy));
  const aspect = Math.atan2(dzdy, -dzdx);
  const zenith = Math.PI / 4;
  const azimuth = (Math.PI * 3) / 4; // 315° als mathematischer Winkel
  const s =
    Math.cos(zenith) * Math.cos(slope) +
    Math.sin(zenith) * Math.sin(slope) * Math.cos(azimuth - aspect);
  return Math.min(Math.max(s, 0), 1);
};

/** RGBA-Bild in Rastergröße: Höhenfarbe × Schummerung; Lücken durchsichtig */
export const renderHeights = (g: HeightGrid, range: [number, number]): Uint8ClampedArray => {
  const out = new Uint8ClampedArray(g.cols * g.rows * 4);
  const [lo, hi] = range;
  for (let r = 0; r < g.rows; r++) {
    for (let c = 0; c < g.cols; c++) {
      const i = r * g.cols + c;
      const v = g.data[i]!;
      if (Number.isNaN(v)) continue;
      const [cr, cg, cb] = heightColor((v - lo) / (hi - lo));
      const shade = 0.55 + 0.45 * hillshade(g, c, r);
      out.set([cr * shade, cg * shade, cb * shade, 255], i * 4);
    }
  }
  return out;
};

/**
 * RGBA-Bild einer Abweichung oder Differenz, symmetrisch um 0. Orange heißt überall „Material weg“ wie im
 * Mengenbericht: bei Soll/Ist (Ist − Soll) zu hoch, bei einer Differenz (nachher − vorher) abgetragen – dafür
 * `removedIsNegative`.
 */
export const renderDeviation = (
  g: HeightGrid,
  limit: number,
  removedIsNegative = false,
): Uint8ClampedArray => {
  const out = new Uint8ClampedArray(g.cols * g.rows * 4);
  const sign = removedIsNegative ? -1 : 1;
  for (let i = 0; i < g.data.length; i++) {
    const v = g.data[i]!;
    if (Number.isNaN(v)) continue;
    const [r, gg, b] = deviationColor(sign * v, limit);
    out.set([r, gg, b, 220], i * 4);
  }
  return out;
};

/** Symmetrische Grenze für die Abweichungsfarben: 98. Perzentil der Beträge, mindestens 5 cm */
export const deviationLimit = (g: HeightGrid): number => {
  const v = Array.from(g.data)
    .filter((x) => !Number.isNaN(x))
    .map(Math.abs)
    .sort((a, b) => a - b);
  return Math.max(0.05, v[Math.floor(0.98 * (v.length - 1))] ?? 0.05);
};

/**
 * Farbe eines Geländepunkts in 3D: Abweichung/Differenz, wo das überlagerte Raster Daten hat (Orange = Material
 * weg wie in 2D), sonst die Höhenfarbe
 */
export const terrainColor =
  (
    range: [number, number],
    overlay: { grid: HeightGrid; limit: number; removedIsNegative: boolean } | null,
  ) =>
  (x: number, y: number, z: number): Rgb => {
    if (overlay) {
      const v = sampleAt(overlay.grid, x, y);
      if (v !== null && !Number.isNaN(v))
        return deviationColor((overlay.removedIsNegative ? -1 : 1) * v, overlay.limit);
    }
    return heightColor((z - range[0]) / (range[1] - range[0]));
  };
