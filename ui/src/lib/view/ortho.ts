/**
 * Orthofoto in der 2D-Ansicht: das Bild liegt über drei Ecken (links oben, rechts oben, links unten) im System
 * der Baustelle. Daraus die affine Abbildung Bildpixel → Bildschirm für `setTransform`.
 */
import { toScreen, type Viewport } from "./viewport";

export type Corners = [[number, number], [number, number], [number, number]];

/** [a, b, c, d, e, f] für `ctx.setTransform`: Bildpixel (u, v) → Bildschirm (a·u + c·v + e, b·u + d·v + f) */
export const orthoTransform = (
  corners: Corners,
  width: number,
  height: number,
  view: Viewport,
): [number, number, number, number, number, number] => {
  const [tl, tr, bl] = corners.map(([x, y]) => toScreen(view, x, y)) as [
    [number, number],
    [number, number],
    [number, number],
  ];
  return [
    (tr[0] - tl[0]) / width,
    (tr[1] - tl[1]) / width,
    (bl[0] - tl[0]) / height,
    (bl[1] - tl[1]) / height,
    tl[0],
    tl[1],
  ];
};
