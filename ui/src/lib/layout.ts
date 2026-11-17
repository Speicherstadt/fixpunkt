/** Eigener Überblick: Abschnitte ein- und ausblenden und verschieben (gespeichert unter /api/me/layouts/{site}) */

export type LayoutItem = { id: string; visible: boolean };

/** Abschnitt um einen Platz nach oben (−1) oder unten (+1); am Rand bleibt alles, wie es ist */
export const move = (items: LayoutItem[], index: number, by: -1 | 1): LayoutItem[] => {
  const to = index + by;
  if (index < 0 || to < 0 || to >= items.length) return items;
  const out = [...items];
  [out[index], out[to]] = [out[to]!, out[index]!];
  return out;
};

export const toggle = (items: LayoutItem[], id: string): LayoutItem[] =>
  items.map((i) => (i.id === id ? { ...i, visible: !i.visible } : i));
