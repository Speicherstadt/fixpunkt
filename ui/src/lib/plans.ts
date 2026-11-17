/** Plan/Ist: Höhen aus dem Plan gegen das Gelände der Aufnahme. */
import type { PlanGeometry } from "@/lib/api";
import { type HeightGrid, sampleAt } from "@/lib/view/grid";

export type PlanIstRow = {
  layer: string;
  at: [number, number];
  soll: number;
  /** null: außerhalb der Aufnahme oder ohne Daten */
  ist: number | null;
  /** Ist − Soll in m */
  diff: number | null;
};

export const planIst = (g: PlanGeometry, grid: HeightGrid | undefined): PlanIstRow[] =>
  g.spots.map((s) => {
    const v = grid ? sampleAt(grid, s.at[0], s.at[1]) : null;
    const ist = v === null || Number.isNaN(v) ? null : v;
    return {
      layer: s.layer,
      at: [s.at[0], s.at[1]],
      soll: s.at[2],
      ist,
      diff: ist === null ? null : ist - s.at[2],
    };
  });

/** Kennzahlen: Punkte mit Ist, mittlere und größte Abweichung (Betrag) */
export const planIstSummary = (rows: PlanIstRow[]) => {
  const d = rows.flatMap((r) => (r.diff === null ? [] : [r.diff]));
  return {
    measured: d.length,
    mean: d.length ? d.reduce((a, b) => a + b, 0) / d.length : null,
    maxAbs: d.length ? Math.max(...d.map(Math.abs)) : null,
  };
};
