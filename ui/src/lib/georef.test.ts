import { describe, expect, it } from "vitest";
import { isOutlier, num, parseGcps, toInput } from "@/lib/georef";

describe("georef", () => {
  it("liest Passpunkte wie die Kommandozeile, auch mit Dezimalkomma", () => {
    const csv = [
      "name,x,y,z,X,Y,Z",
      "P1,612000.1,5352999.9,302.1,512000.1,5403000.1,300.0",
      "",
      "P2;612039,9;5352999,9;302,5;512039,9;5403000,1;300,4",
      "P3\t1\t2\t3\t4\t5\t6",
      "kaputt,1,2",
    ].join("\n");
    const pts = parseGcps(csv);
    expect(pts.map((p) => p.name)).toEqual(["P1", "P2", "P3"]);
    expect(pts[0]!.dst).toEqual([512000.1, 5403000.1, 300]);
    expect(pts[1]!.src).toEqual([612039.9, 5352999.9, 302.5]);
    expect(pts[2]!.dst).toEqual([4, 5, 6]);
  });

  it("Zahlen für Eingabefelder", () => {
    expect(toInput(512039.9, ",")).toBe("512039,9");
    expect(toInput(300, ",")).toBe("300");
    expect(num(toInput(-0.125, ","))).toBe(-0.125);
  });

  it("Zahlen aus Eingaben", () => {
    expect(num("300,25")).toBe(300.25);
    expect(num(" ")).toBeNull();
    expect(num("abc")).toBeNull();
  });

  it("Ausreißer: dreifaches RMS, mindestens 5 cm", () => {
    expect(isOutlier(0.04, 0.001)).toBe(false);
    expect(isOutlier(0.06, 0.01)).toBe(true);
    expect(isOutlier(0.25, 0.1)).toBe(false);
  });
});
