import { describe, expect, it } from "vitest";
import {
  deviationColor,
  deviationLimit,
  heightColor,
  hillshade,
  renderDeviation,
  renderHeights,
} from "./colors";
import {
  length2d,
  measureArea,
  measureLine,
  type Pt,
  perimeter,
  pointInPolygon,
  polygonArea,
  polygonGeojson,
  profile,
} from "./geometry";
import { bboxOf, type HeightGrid, parseGrid, robustRange, sampleAt, zRange } from "./grid";
import { fitTo, pan, scaleBar, toScreen, toWorld, zoomAt } from "./viewport";

const X0 = 512_000;
const Y0 = 5_403_000;

/** 40 × 40 m, 0,5 m Raster, Ebene z = 300 + 0,02·dx (2 % nach Osten steigend) */
const plane = (f = (dx: number, _dy: number) => 300 + 0.02 * dx): HeightGrid => {
  const cols = 80;
  const rows = 80;
  const data = new Float32Array(cols * rows);
  for (let r = 0; r < rows; r++)
    for (let c = 0; c < cols; c++) data[r * cols + c] = f((c + 0.5) * 0.5, 40 - (r + 0.5) * 0.5);
  return { minX: X0, maxY: Y0 + 40, cell: 0.5, cols, rows, data };
};

describe("grid", () => {
  it("liest das Binärformat mit den Headern", () => {
    const data = new Float32Array([1, 2, Number.NaN, 4]);
    const h = new Headers({
      "x-grid-min-x": "10",
      "x-grid-max-y": "20",
      "x-grid-cell": "0.5",
      "x-grid-cols": "2",
      "x-grid-rows": "2",
    });
    const g = parseGrid(h, data.buffer);
    expect([g.cols, g.rows, g.cell]).toEqual([2, 2, 0.5]);
    expect(bboxOf(g)).toEqual({ minX: 10, minY: 19, maxX: 11, maxY: 20 });
    expect(zRange(g)).toEqual([1, 4]);
    expect(() => parseGrid(h, new Float32Array(3).buffer)).toThrow();
  });

  it("tastet bilinear ab und kennt Lücken und Rand", () => {
    const g = plane();
    expect(sampleAt(g, X0 + 10, Y0 + 20)).toBeCloseTo(300.2, 4);
    expect(sampleAt(g, X0 + 33.3, Y0 + 5)).toBeCloseTo(300.666, 3);
    expect(sampleAt(g, X0 - 1, Y0)).toBeNull();
    g.data.fill(Number.NaN);
    expect(sampleAt(g, X0 + 10, Y0 + 20)).toBeNull();
  });

  it("Farbskala ohne Ausreißer", () => {
    const g = plane();
    g.data[0] = 999;
    const [lo, hi] = robustRange(g)!;
    expect(hi).toBeLessThan(301);
    expect(lo).toBeGreaterThanOrEqual(300);
  });
});

describe("geometry", () => {
  const square: Pt[] = [
    [X0 + 5, Y0 + 5],
    [X0 + 25, Y0 + 5],
    [X0 + 25, Y0 + 15],
    [X0 + 5, Y0 + 15],
  ];

  it("Strecke, Fläche, Umfang, Punkt im Polygon", () => {
    expect(
      length2d([
        [0, 0],
        [3, 4],
        [3, 10],
      ]),
    ).toBe(11);
    expect(polygonArea(square)).toBeCloseTo(200, 9);
    expect(perimeter(square)).toBeCloseTo(60, 9);
    expect(pointInPolygon(square, X0 + 10, Y0 + 10)).toBe(true);
    expect(pointInPolygon(square, X0 + 30, Y0 + 10)).toBe(false);
  });

  it("Linie auf geneigter Ebene: 2 % Gefälle, 3D-Länge", () => {
    const m = measureLine(plane(), [
      [X0 + 5, Y0 + 20],
      [X0 + 35, Y0 + 20],
    ]);
    expect(m.length2d).toBeCloseTo(30, 9);
    expect(m.length3d).toBeCloseTo(30 * Math.sqrt(1 + 0.02 ** 2), 2);
    expect(m.slope).toBeCloseTo(2, 2);
    expect(m.dz).toBeCloseTo(0.6, 3);
  });

  it("Profil mit Stationen bis zum Ende", () => {
    const p = profile(
      plane(),
      [
        [X0 + 5, Y0 + 5],
        [X0 + 5, Y0 + 15],
        [X0 + 15, Y0 + 15],
      ],
      1,
    );
    expect(p[0]!.station).toBe(0);
    expect(p.at(-1)!.station).toBeCloseTo(20, 9);
    expect(p.length).toBe(21);
  });

  it("Oberfläche einer 3:4-Böschung ist 1,25 × Grundfläche", () => {
    const g = plane((dx) => 300 + 0.75 * dx);
    const a = measureArea(g, square);
    expect(a.area2d).toBeCloseTo(200, 9);
    expect(a.area3d).toBeGreaterThan(245);
    expect(a.area3d).toBeLessThan(255);
    expect(a.zMin!).toBeLessThan(a.zMax!);
  });

  it("GeoJSON mit Koordinatensystem und geschlossenem Ring", () => {
    const gj = polygonGeojson(square, 25832);
    expect(gj.crs.properties.name).toBe("urn:ogc:def:crs:EPSG::25832");
    expect(gj.coordinates[0]).toHaveLength(5);
    expect(gj.coordinates[0]![4]).toEqual(square[0]);
  });
});

describe("colors", () => {
  it("Rampen an den Enden und in der Mitte", () => {
    expect(heightColor(0)).toEqual([38, 70, 166]);
    expect(heightColor(1)).toEqual([240, 235, 230]);
    expect(deviationColor(0, 1)).toEqual([245, 245, 245]);
    expect(deviationColor(-5, 1)).toEqual([29, 78, 216]);
    expect(deviationColor(5, 1)).toEqual([180, 83, 9]);
  });

  it("Schummerung: Hang zum Licht heller als vom Licht weg", () => {
    // Licht aus Nordwest: ein Hang, der nach Osten ansteigt, schaut nach Westen – zum Licht – und ist heller
    const facingWest = plane((dx) => 300 + 0.5 * dx);
    const facingEast = plane((dx) => 300 - 0.5 * dx);
    expect(hillshade(facingWest, 40, 40)).toBeGreaterThan(hillshade(facingEast, 40, 40));
  });

  it("Bilder haben die Rastergröße, Lücken sind durchsichtig", () => {
    const g = plane();
    g.data[0] = Number.NaN;
    const img = renderHeights(g, [300, 300.8]);
    expect(img.length).toBe(80 * 80 * 4);
    expect(img[3]).toBe(0);
    expect(img[7]).toBe(255);
    const d = plane(() => 0.2);
    expect(deviationLimit(d)).toBeCloseTo(0.2, 5);
    expect(renderDeviation(d, 0.2)[3]).toBe(220);
    // Differenz: −0,2 (abgetragen) wird orange wie +0,2 bei Soll/Ist
    const cut = plane(() => -0.2);
    expect(Array.from(renderDeviation(cut, 0.2, true).slice(0, 3))).toEqual([180, 83, 9]);
  });
});

describe("viewport", () => {
  const b = { minX: X0, minY: Y0, maxX: X0 + 40, maxY: Y0 + 20 };

  it("passt ein und rechnet hin und zurück", () => {
    const v = fitTo(b, 800, 400, 0);
    expect(v.scale).toBe(20);
    expect(toScreen(v, X0, Y0 + 20)).toEqual([0, 0]);
    const [x, y] = toWorld(v, 400, 200);
    expect([x, y]).toEqual([X0 + 20, Y0 + 10]);
  });

  it("Zoom hält den Punkt unter der Maus fest, Verschieben in Pixeln", () => {
    const v = fitTo(b, 800, 400, 0);
    const before = toWorld(v, 100, 50);
    const z = zoomAt(v, 100, 50, 2);
    expect(z.scale).toBe(40);
    const after = toWorld(z, 100, 50);
    expect(after[0]).toBeCloseTo(before[0], 9);
    expect(after[1]).toBeCloseTo(before[1], 9);
    const p = pan(v, 20, 0);
    expect(p.cx).toBeCloseTo(v.cx - 1, 9);
  });

  it("runder Maßstabsbalken", () => {
    expect(scaleBar({ scale: 20, cx: 0, cy: 0, width: 1, height: 1 })).toEqual({
      meters: 5,
      px: 100,
    });
    expect(scaleBar({ scale: 0.5, cx: 0, cy: 0, width: 1, height: 1 }).meters).toBe(200);
  });
});

describe("ringOf", async () => {
  const { ringOf } = await import("./geometry");
  it("liest Feature und Polygon, entfernt den Schlusspunkt", () => {
    const ring = [
      [1, 2],
      [3, 2],
      [3, 4],
      [1, 2],
    ];
    expect(ringOf({ type: "Feature", geometry: { type: "Polygon", coordinates: [ring] } })).toEqual(
      [
        [1, 2],
        [3, 2],
        [3, 4],
      ],
    );
    expect(ringOf({ type: "Polygon", coordinates: [ring.slice(0, 3)] })).toHaveLength(3);
    expect(ringOf(null)).toEqual([]);
    expect(ringOf({ coordinates: "kaputt" })).toEqual([]);
  });
});
