import { describe, expect, it } from "vitest";
import { heightColor } from "./colors";
import { colorModes, intensityRange, parsePoints, pointScene } from "./points";

/** Puffer wie points.rs ihn schreibt */
const encode = (
  pts: [number, number, number][],
  rgb?: [number, number, number][],
  intensity?: number[],
) => {
  const n = pts.length;
  const at = 29 + n * 12 + (rgb ? n * 3 : 0);
  const buf = new ArrayBuffer(at + (intensity ? n * 2 : 0));
  const v = new DataView(buf);
  v.setUint32(0, n, true);
  v.setUint8(4, (rgb ? 1 : 0) | (intensity ? 2 : 0));
  for (const [i, x] of (intensity ?? []).entries()) v.setUint16(at + i * 2, x, true);
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

  it("liest die Intensität hinter den Farben, auch an ungerader Stelle", () => {
    // 1 Punkt: 29 + 12 + 3 = 44 … gerade; 2 Punkte ohne Farbe: 29 + 24 = 53, ungerade
    const p = parsePoints(
      encode(
        [
          [0, 0, 0],
          [1, 1, 1],
        ],
        undefined,
        [100, 65535],
      ),
    );
    expect(Array.from(p.intensity!)).toEqual([100, 65535]);
    expect(p.rgb).toBeNull();
    expect(colorModes(p)).toEqual(["intensity", "height"]);
    expect(colorModes({ rgb: new Uint8Array(3), intensity: null })).toEqual(["file", "height"]);
  });

  it("färbt nach Intensität grau, Ausreißer abgeschnitten", () => {
    // 100 Werte 0…99 und ein Glanzlicht 65535: 2.–98. Perzentil = 2 … 98
    const v = new Uint16Array([...Array.from({ length: 100 }, (_, i) => i), 65535]);
    expect(intensityRange(v)).toEqual([2, 98]);
    const p = parsePoints(
      encode(
        [
          [0, 0, 0],
          [1, 0, 0],
          [2, 0, 0],
        ],
        [
          [255, 0, 0],
          [255, 0, 0],
          [255, 0, 0],
        ],
        [0, 32768, 65535],
      ),
    );
    const grey = pointScene(p, 1, "intensity").colors;
    expect(grey[0]).toBe(0);
    expect(grey[3]).toBeCloseTo(0.5, 2);
    expect(grey[6]).toBe(1);
    expect(grey[6]).toBe(grey[7]);
    // Vorgabe bleibt die Farbe der Datei
    expect(Array.from(pointScene(p).colors.slice(0, 3))).toEqual([1, 0, 0]);
  });
});
