/** Ansichtsfenster: Welt (m, Nord oben) ↔ Bildschirm (px). */
import type { BBox } from "./grid";

export type Viewport = { scale: number; cx: number; cy: number; width: number; height: number };

/** Ganze Ausdehnung mit etwas Rand */
export const fitTo = (b: BBox, width: number, height: number, margin = 0.06): Viewport => {
  const w = Math.max(b.maxX - b.minX, 1e-6);
  const h = Math.max(b.maxY - b.minY, 1e-6);
  const scale = Math.min(width / w, height / h) * (1 - 2 * margin);
  return { scale, cx: (b.minX + b.maxX) / 2, cy: (b.minY + b.maxY) / 2, width, height };
};

export const toScreen = (v: Viewport, x: number, y: number): [number, number] => [
  v.width / 2 + (x - v.cx) * v.scale,
  v.height / 2 - (y - v.cy) * v.scale,
];

export const toWorld = (v: Viewport, px: number, py: number): [number, number] => [
  v.cx + (px - v.width / 2) / v.scale,
  v.cy - (py - v.height / 2) / v.scale,
];

/** Zoomen um einen Bildschirmpunkt: dieser Punkt bleibt, wo er ist */
export const zoomAt = (v: Viewport, px: number, py: number, factor: number): Viewport => {
  const [wx, wy] = toWorld(v, px, py);
  const scale = Math.min(Math.max(v.scale * factor, 0.01), 2000);
  const next = { ...v, scale };
  const [nx, ny] = toWorld(next, px, py);
  return { ...next, cx: next.cx + (wx - nx), cy: next.cy + (wy - ny) };
};

export const pan = (v: Viewport, dx: number, dy: number): Viewport => ({
  ...v,
  cx: v.cx - dx / v.scale,
  cy: v.cy + dy / v.scale,
});

/** Maßstabsbalken: runde Länge in m, die etwa `target` px breit ist */
export const scaleBar = (v: Viewport, target = 120): { meters: number; px: number } => {
  const raw = target / v.scale;
  const pow = 10 ** Math.floor(Math.log10(raw));
  const meters = [1, 2, 5, 10].map((f) => f * pow).find((m) => m >= raw * 0.75) ?? 10 * pow;
  return { meters, px: meters * v.scale };
};
