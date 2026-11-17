import { describe, expect, it } from "vitest";
import type { HeightGrid } from "./grid";
import { buildMesh, MAX_SIDE } from "./mesh";

const grid = (cols: number, rows: number, z: (c: number, r: number) => number): HeightGrid => {
  const data = new Float32Array(cols * rows);
  for (let r = 0; r < rows; r++) for (let c = 0; c < cols; c++) data[r * cols + c] = z(c, r);
  return { minX: 512_000, maxY: 5_403_000, cell: 0.5, cols, rows, data };
};

const grey = () => [128, 128, 128] as [number, number, number];

/** Normale eines Dreiecks (Kreuzprodukt), unabhängig von three.js */
const normal = (p: Float32Array, a: number, b: number, c: number) => {
  const v = (i: number) => [p[i * 3]!, p[i * 3 + 1]!, p[i * 3 + 2]!];
  const [A, B, C] = [v(a), v(b), v(c)];
  const u = B.map((x, i) => x - A[i]!);
  const w = C.map((x, i) => x - A[i]!);
  return [
    u[1]! * w[2]! - u[2]! * w[1]!,
    u[2]! * w[0]! - u[0]! * w[2]!,
    u[0]! * w[1]! - u[1]! * w[0]!,
  ];
};

describe("mesh", () => {
  it("zwei Dreiecke je Zelle, Normalen nach oben, um die Mitte gelegt", () => {
    const m = buildMesh(
      grid(3, 3, () => 300),
      grey,
    );
    expect(m.triangles).toBe(8);
    for (let t = 0; t < m.triangles; t++) {
      const n = normal(
        m.positions,
        m.indices[t * 3]!,
        m.indices[t * 3 + 1]!,
        m.indices[t * 3 + 2]!,
      );
      expect(n[1]).toBeGreaterThan(0);
    }
    // 3 × 0,5 m breit: Stützpunkte bei −0,5 … +0,5 m um die Mitte (Zellmitten)
    const xs = Array.from({ length: 9 }, (_, i) => m.positions[i * 3]!);
    expect(Math.min(...xs)).toBeCloseTo(-0.5);
    expect(Math.max(...xs)).toBeCloseTo(0.5);
    expect(m.size).toEqual([1.5, 1.5, 0]);
    expect(m.colors[0]).toBeCloseTo(128 / 255);
  });

  it("Höhe über der tiefsten Stelle, überhöht; Norden liegt bei −z", () => {
    // Zeile 0 = Norden 2 m höher
    const m = buildMesh(
      grid(2, 2, (_, r) => (r === 0 ? 302 : 300)),
      grey,
      3,
    );
    expect(m.zMin).toBe(300);
    expect(m.positions[1]).toBeCloseTo(6); // Nordwest: 2 m × 3
    expect(m.positions[2]).toBeLessThan(0); // Norden bei −z
    expect(m.positions[2 * 3 + 1]).toBeCloseTo(0); // Südwest
    expect(m.size[2]).toBeCloseTo(6);
  });

  it("Lücken bleiben Löcher, große Raster werden ausgedünnt", () => {
    const holed = buildMesh(
      grid(3, 3, (c, r) => (c === 1 && r === 1 ? Number.NaN : 300)),
      grey,
    );
    expect(holed.triangles).toBe(0); // jede der vier Zellen berührt die Lücke
    const big = buildMesh(
      grid(1000, 600, () => 1),
      grey,
    );
    // jede 3. Zelle (1000 / 400 aufgerundet): 334 × 200 Stützpunkte, höchstens MAX_SIDE je Seite
    expect(big.positions.length / 3).toBe(334 * 200);
    expect(334).toBeLessThanOrEqual(MAX_SIDE);
    expect(big.triangles).toBe(333 * 199 * 2);
  });
});
