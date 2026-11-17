import { describe, expect, it } from "vitest";
import type { Marker } from "@/lib/api";
import { checkConstraint, pinsOf } from "@/lib/markers";
import type { HeightGrid } from "@/lib/view/grid";

const marker = (over: Partial<Marker>): Marker => ({
  id: 1,
  siteId: 1,
  kind: "constraint",
  title: "Kanal",
  note: "",
  x: 5,
  y: 5,
  z: 298,
  photo: null,
  takenAt: null,
  createdAt: 0,
  ...over,
});

/** 10 × 10 m eben auf 298,05 m */
const grid: HeightGrid = {
  minX: 0,
  maxY: 10,
  cell: 1,
  cols: 10,
  rows: 10,
  data: new Float32Array(100).fill(298.05),
};

describe("markers", () => {
  it("Zwangspunkt: Ist − Soll, innerhalb von 3 cm in Ordnung", () => {
    const c = checkConstraint(marker({}), grid)!;
    expect(c.actual).toBeCloseTo(298.05, 4);
    expect(c.diff).toBeCloseTo(0.05, 4);
    expect(c.ok).toBe(false);
    expect(checkConstraint(marker({ z: 298.03 }), grid)!.ok).toBe(true);
  });

  it("ohne Lage, außerhalb oder kein Zwangspunkt: keine Prüfung", () => {
    expect(checkConstraint(marker({ x: 50 }), grid)).toBeNull();
    expect(checkConstraint(marker({ kind: "note" }), grid)).toBeNull();
    expect(checkConstraint(marker({}), undefined)).toBeNull();
  });

  it("Punkte für die Ansicht nur mit Lage", () => {
    const pins = pinsOf([marker({}), marker({ id: 2, kind: "photo", x: null, y: null })]);
    expect(pins).toEqual([{ label: "Kanal", at: [5, 5], diamond: true }]);
  });
});
