import { describe, expect, it } from "vitest";
import { heightColor } from "./colors";
import { parsePoints, pointScene } from "./points";

/** Puffer wie points.rs ihn schreibt */
const encode = (pts: [number, number, number][], rgb?: [number, number, number][]) => {
  const n = pts.length;
  const buf = new ArrayBuffer(29 + n * 12 + (rgb ? n * 3 : 0));
  const v = new DataView(buf);
  v.setUint32(0, n, true);
  v.setUint8(4, rgb ? 1 : 0);
  for (const [i, o] of [512020, 5403020, 298].entries()) v.setFloat64(5 + 8 * i, o, true);
  for (const [i, p] of pts.entries())
    for (const [k, c] of p.entries()) v.setFloat32(29 + i * 12 + k * 4, c, true);
  for (const [i, c] of (rgb ?? []).entries())
    for (const [k, x] of c.entries()) v.setUint8(29 + n * 12 + i * 3 + k, x);
  return buf;
};

describe("points", () => {
  it("liest Kopf, Lage und Farben", () => {
    const p = parsePoints(
      encode(
        [
          [-1, 2, 0.5],
          [3, -4, 2],
        ],
        [
          [255, 0, 0],
          [0, 0, 255],
        ],
      ),
    );
    expect(p).toMatchObject({ count: 2, colored: true, origin: [512020, 5403020, 298] });
    expect(Array.from(p.xyz)).toEqual([-1, 2, 0.5, 3, -4, 2]);
    expect(Array.from(p.rgb!)).toEqual([255, 0, 0, 0, 0, 255]);
    expect(() => parsePoints(encode([[0, 0, 0]]).slice(0, 35))).toThrow();
  });

  it("three.js: y nach oben überhöht, Norden bei −z, Farben 0…1 oder nach Höhe", () => {
    const s = pointScene(
      parsePoints(
        encode(
          [
            [1, 2, 0],
            [3, 4, 2],
          ],
          [
            [255, 0, 0],
            [0, 0, 255],
          ],
        ),
      ),
      2,
    );
    expect(Array.from(s.positions)).toEqual([1, 0, -2, 3, 4, -4]);
    expect(Array.from(s.colors)).toEqual([1, 0, 0, 0, 0, 1]);
    expect(s.size).toEqual([2, 2, 4]);
    const plain = pointScene(
      parsePoints(
        encode([
          [0, 0, 0],
          [0, 0, 2],
        ]),
      ),
    );
    // Float32: auf 6 Stellen gleich
    const want = heightColor(1).map((v) => v / 255);
    for (const [i, v] of Array.from(plain.colors.slice(3, 6)).entries())
      expect(v).toBeCloseTo(want[i]!, 6);
  });
});
