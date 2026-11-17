/** Messen auf dem Höhenraster: Strecke, Fläche, Profil, Gefälle. Koordinaten in m (UTM). */
import { bboxOf, type HeightGrid, sampleAt } from "./grid";

export type Pt = [number, number];

const dist = (a: Pt, b: Pt) => Math.hypot(b[0] - a[0], b[1] - a[1]);

export const length2d = (pts: Pt[]): number =>
  pts.slice(1).reduce((s, p, i) => s + dist(pts[i]!, p), 0);

/** Fläche eines Polygons (Gaußsche Trapezformel, relativ zum ersten Punkt gegen Auslöschung) */
export const polygonArea = (pts: Pt[]): number => {
  if (pts.length < 3) return 0;
  const [ox, oy] = pts[0]!;
  let s = 0;
  for (let i = 0; i < pts.length; i++) {
    const a = pts[i]!;
    const b = pts[(i + 1) % pts.length]!;
    s += (a[0] - ox) * (b[1] - oy) - (b[0] - ox) * (a[1] - oy);
  }
  return Math.abs(s) / 2;
};

export const perimeter = (pts: Pt[]): number => (pts.length < 2 ? 0 : length2d([...pts, pts[0]!]));

export const pointInPolygon = (pts: Pt[], x: number, y: number): boolean => {
  let inside = false;
  for (let i = 0, j = pts.length - 1; i < pts.length; j = i++) {
    const [xi, yi] = pts[i]!;
    const [xj, yj] = pts[j]!;
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
};

export type ProfilePoint = { station: number; x: number; y: number; z: number | null };

/** Punkte entlang der Linie etwa alle `step` m, inklusive aller Stützpunkte */
export const profile = (g: HeightGrid, pts: Pt[], step: number): ProfilePoint[] => {
  const out: ProfilePoint[] = [];
  let station = 0;
  for (let i = 1; i < pts.length; i++) {
    const a = pts[i - 1]!;
    const b = pts[i]!;
    const len = dist(a, b);
    const n = Math.max(1, Math.ceil(len / step));
    for (let k = 0; k < n; k++) {
      const t = k / n;
      const x = a[0] + t * (b[0] - a[0]);
      const y = a[1] + t * (b[1] - a[1]);
      out.push({ station: station + t * len, x, y, z: sampleAt(g, x, y) });
    }
    station += len;
  }
  const last = pts.at(-1);
  if (last) out.push({ station, x: last[0], y: last[1], z: sampleAt(g, last[0], last[1]) });
  return out;
};

export type LineMeasure = {
  length2d: number;
  length3d: number;
  zStart: number | null;
  zEnd: number | null;
  dz: number | null;
  /** mittleres Gefälle in % (dz / Länge) */
  slope: number | null;
};

export const measureLine = (g: HeightGrid, pts: Pt[]): LineMeasure => {
  const p = profile(g, pts, g.cell / 2);
  let l3 = 0;
  for (let i = 1; i < p.length; i++) {
    const a = p[i - 1]!;
    const b = p[i]!;
    const d2 = b.station - a.station;
    const dz = a.z !== null && b.z !== null ? b.z - a.z : 0;
    l3 += Math.hypot(d2, dz);
  }
  const l2 = length2d(pts);
  const zStart = p[0]?.z ?? null;
  const zEnd = p.at(-1)?.z ?? null;
  const dz = zStart !== null && zEnd !== null ? zEnd - zStart : null;
  return {
    length2d: l2,
    length3d: l3,
    zStart,
    zEnd,
    dz,
    slope: dz !== null && l2 > 0 ? (dz / l2) * 100 : null,
  };
};

export type AreaMeasure = {
  area2d: number;
  area3d: number;
  perimeter: number;
  zMin: number | null;
  zMax: number | null;
};

/** Fläche horizontal (exakt) und Oberfläche (aus dem Raster, Neigung je Zelle) */
export const measureArea = (g: HeightGrid, pts: Pt[]): AreaMeasure => {
  const b = bboxOf(g);
  const xs = pts.map((p) => p[0]);
  const ys = pts.map((p) => p[1]);
  const c0 = Math.max(0, Math.floor((Math.min(...xs) - b.minX) / g.cell));
  const c1 = Math.min(g.cols - 1, Math.ceil((Math.max(...xs) - b.minX) / g.cell));
  const r0 = Math.max(0, Math.floor((b.maxY - Math.max(...ys)) / g.cell));
  const r1 = Math.min(g.rows - 1, Math.ceil((b.maxY - Math.min(...ys)) / g.cell));
  const z = (c: number, r: number) =>
    c < 0 || r < 0 || c >= g.cols || r >= g.rows ? Number.NaN : g.data[r * g.cols + c]!;
  let area3d = 0;
  let lo = Number.POSITIVE_INFINITY;
  let hi = Number.NEGATIVE_INFINITY;
  for (let r = r0; r <= r1; r++) {
    for (let c = c0; c <= c1; c++) {
      const x = b.minX + (c + 0.5) * g.cell;
      const y = b.maxY - (r + 0.5) * g.cell;
      const h = z(c, r);
      if (Number.isNaN(h) || !pointInPolygon(pts, x, y)) continue;
      const grad = (a: number, d: number) =>
        Number.isNaN(a) || Number.isNaN(d) ? 0 : (d - a) / (2 * g.cell);
      const gx = grad(z(c - 1, r), z(c + 1, r));
      const gy = grad(z(c, r + 1), z(c, r - 1));
      area3d += g.cell * g.cell * Math.sqrt(1 + gx * gx + gy * gy);
      lo = Math.min(lo, h);
      hi = Math.max(hi, h);
    }
  }
  return {
    area2d: polygonArea(pts),
    area3d,
    perimeter: perimeter(pts),
    zMin: lo <= hi ? lo : null,
    zMax: lo <= hi ? hi : null,
  };
};

/** GeoJSON eines gezeichneten Bereichs im metrischen System (crs wie ein QGIS-Export) */
export const polygonGeojson = (pts: Pt[], epsg: number) => ({
  type: "Polygon",
  crs: { type: "name", properties: { name: `urn:ogc:def:crs:EPSG::${epsg}` } },
  coordinates: [[...pts, pts[0]!]],
});

/** Außenring eines gespeicherten Bereichs (Feature oder Polygon) ohne doppelten Schlusspunkt */
export const ringOf = (geojson: unknown): Pt[] => {
  const g = (geojson as { geometry?: unknown })?.geometry ?? geojson;
  const coords = (g as { coordinates?: unknown })?.coordinates;
  const ring = Array.isArray(coords) ? coords[0] : undefined;
  if (!Array.isArray(ring)) return [];
  const pts = ring.filter(
    (p): p is Pt => Array.isArray(p) && typeof p[0] === "number" && typeof p[1] === "number",
  );
  const first = pts[0];
  const last = pts.at(-1);
  return first && last && pts.length > 1 && first[0] === last[0] && first[1] === last[1]
    ? pts.slice(0, -1)
    : pts;
};
