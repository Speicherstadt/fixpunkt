import { describe, expect, it } from "vitest";
import type { PlanGeometry } from "@/lib/api";
import { planIst, planIstSummary } from "@/lib/plans";
import type { HeightGrid } from "@/lib/view/grid";

/** 10 × 10 m, Höhe = 300 + 0,1 · Ost (wie eine Rampe) */
const ramp: HeightGrid = {
  minX: 0,
  maxY: 10,
  cell: 1,
  cols: 10,
  rows: 10,
  data: Float32Array.from({ length: 100 }, (_, i) => 300 + 0.1 * ((i % 10) + 0.5)),
};

const plan = (spots: [number, number, number][]): PlanGeometry => ({
  paths: [],
  labels: [],
  layers: ["Hoehen"],
  spots: spots.map((at) => ({ layer: "Hoehen", at })),
});

describe("plans", () => {
  it("Ist − Soll je Höhenpunkt, außerhalb ohne Ist", () => {
    const rows = planIst(
      plan([
        [5.5, 5, 300.5],
        [2.5, 5, 300.35],
        [50, 5, 300],
      ]),
      ramp,
    );
    expect(rows[0]!.ist).toBeCloseTo(300.55, 4);
    expect(rows[0]!.diff).toBeCloseTo(0.05, 4);
    expect(rows[1]!.diff).toBeCloseTo(-0.1, 4);
    expect(rows[2]).toMatchObject({ ist: null, diff: null });
    const s = planIstSummary(rows);
    expect(s.measured).toBe(2);
    expect(s.mean).toBeCloseTo(-0.025, 4);
    expect(s.maxAbs).toBeCloseTo(0.1, 4);
  });

  it("ohne Aufnahme nur Soll", () => {
    const rows = planIst(plan([[1, 1, 299]]), undefined);
    expect(rows[0]).toMatchObject({ soll: 299, ist: null });
    expect(planIstSummary(rows)).toEqual({ measured: 0, mean: null, maxAbs: null });
  });
});
