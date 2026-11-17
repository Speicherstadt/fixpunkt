import { describe, expect, it } from "vitest";
import type { Marker } from "@/lib/api";
import { checkConstraint, lineLength, linesOf, pinsOf, tracePoints } from "@/lib/markers";
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

describe("Linien", () => {
  it("misst die Länge in der Ebene", () => {
    // 3-4-5-Dreieck, dann 6 m: 11 m; die Höhe zählt nicht
    expect(
      lineLength([
        [0, 0],
        [3, 4, 298],
        [3, 10],
      ]),
    ).toBe(11);
    expect(lineLength([[1, 1]])).toBe(0);
  });

  it("nimmt die Höhe des Geländes, wo es eine gibt", () => {
    const pts = tracePoints(
      [
        [2.5, 2.5],
        [50, 50],
      ],
      grid,
    );
    expect(pts[0]).toEqual([2.5, 2.5, 298.05]);
    expect(pts[1]).toEqual([50, 50]);
  });

  it("zeichnet Linien als Linien, nicht als Punkte", () => {
    const line = marker({
      kind: "line",
      title: "Leitung",
      points: [
        [0, 0],
        [1, 1],
      ],
    });
    expect(pinsOf([line])).toEqual([]);
    expect(linesOf([line])).toEqual([
      {
        name: "Leitung",
        points: [
          [0, 0],
          [1, 1],
        ],
      },
    ]);
  });
});
