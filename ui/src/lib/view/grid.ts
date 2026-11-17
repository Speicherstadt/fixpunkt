/**
 * Höhenraster der 2D-Ansicht: vom Server als Float32 (Zeile 0 = Norden, NaN = keine Daten), Lage in den
 * Headern `x-grid-*` (crates/server/src/view.rs). Koordinaten in m (ETRS89/UTM32).
 */
import { ApiError, errorOf } from "@/lib/api";

export type HeightGrid = {
  minX: number;
  maxY: number;
  cell: number;
  cols: number;
  rows: number;
  data: Float32Array;
};

export type BBox = { minX: number; minY: number; maxX: number; maxY: number };

export const parseGrid = (headers: Headers, buffer: ArrayBuffer): HeightGrid => {
  const num = (k: string) => {
    const v = Number(headers.get(`x-grid-${k}`));
    if (!Number.isFinite(v)) throw new ApiError(0, "api.internal", `x-grid-${k}`);
    return v;
  };
  const g = {
    minX: num("min-x"),
    maxY: num("max-y"),
    cell: num("cell"),
    cols: num("cols"),
    rows: num("rows"),
  };
  const data = new Float32Array(buffer);
  if (data.length !== g.cols * g.rows) throw new ApiError(0, "api.internal", "grid size");
  return { ...g, data };
};

export const fetchGrid = async (url: string): Promise<HeightGrid> => {
  const res = await fetch(url, { credentials: "same-origin" });
  if (!res.ok) throw await errorOf(res);
  return parseGrid(res.headers, await res.arrayBuffer());
};

export const bboxOf = (g: HeightGrid): BBox => ({
  minX: g.minX,
  minY: g.maxY - g.rows * g.cell,
  maxX: g.minX + g.cols * g.cell,
  maxY: g.maxY,
});

const at = (g: HeightGrid, c: number, r: number): number => g.data[r * g.cols + c] ?? Number.NaN;

/** Höhe an (x, y), bilinear zwischen den Zellmitten; null außerhalb oder in einer Lücke */
export const sampleAt = (g: HeightGrid, x: number, y: number): number | null => {
  const b = bboxOf(g);
  if (x < b.minX || x > b.maxX || y < b.minY || y > b.maxY) return null;
  const fc = Math.min(Math.max((x - g.minX) / g.cell - 0.5, 0), g.cols - 1);
  const fr = Math.min(Math.max((g.maxY - y) / g.cell - 0.5, 0), g.rows - 1);
  const c0 = Math.min(Math.floor(fc), Math.max(g.cols - 2, 0));
  const r0 = Math.min(Math.floor(fr), Math.max(g.rows - 2, 0));
  const c1 = Math.min(c0 + 1, g.cols - 1);
  const r1 = Math.min(r0 + 1, g.rows - 1);
  const tx = Math.min(Math.max(fc - c0, 0), 1);
  const ty = Math.min(Math.max(fr - r0, 0), 1);
  const v = [at(g, c0, r0), at(g, c1, r0), at(g, c0, r1), at(g, c1, r1)] as const;
  if (v.some(Number.isNaN)) {
    const z = at(
      g,
      Math.min(Math.floor((x - g.minX) / g.cell), g.cols - 1),
      Math.min(Math.floor((g.maxY - y) / g.cell), g.rows - 1),
    );
    return Number.isNaN(z) ? null : z;
  }
  return v[0] * (1 - tx) * (1 - ty) + v[1] * tx * (1 - ty) + v[2] * (1 - tx) * ty + v[3] * tx * ty;
};

/** Kleinste und größte Höhe (Lücken übersprungen) */
export const zRange = (g: HeightGrid): [number, number] | null => {
  let lo = Number.POSITIVE_INFINITY;
  let hi = Number.NEGATIVE_INFINITY;
  for (const v of g.data) {
    if (Number.isNaN(v)) continue;
    if (v < lo) lo = v;
    if (v > hi) hi = v;
  }
  return lo <= hi ? [lo, hi] : null;
};

/** Grenzen für die Farbskala: 2.–98. Perzentil, damit einzelne Ausreißer die Farben nicht stauchen */
export const robustRange = (g: HeightGrid): [number, number] | null => {
  const v = Array.from(g.data).filter((x) => !Number.isNaN(x));
  if (!v.length) return null;
  v.sort((a, b) => a - b);
  const q = (p: number) => v[Math.min(v.length - 1, Math.floor(p * (v.length - 1)))]!;
  const lo = q(0.02);
  const hi = q(0.98);
  return hi > lo ? [lo, hi] : [lo - 0.5, hi + 0.5];
};
