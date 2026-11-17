import { describe, expect, it } from "vitest";
import { DEFAULT_FLIGHT, flightQuery, flightTime, gsdSpread, projector } from "./flight";

describe("Flugplanung", () => {
  it("schreibt die Einstellungen als Query", () => {
    expect(flightQuery({ ...DEFAULT_FLIGHT, gsd: 1.5, drone: "m3e" })).toBe(
      "gsd=1.5&front=80&side=70&direction=0&speed=10&drone=m3e",
    );
  });

  it("zeichnet Nord oben und hält den Maßstab", () => {
    // 100 × 50 m, Quadrat 216 px mit 8 px Rand → 2 px je m, in der Höhe mittig (50 px frei oben und unten)
    const p = projector(
      [
        [512_000, 5_403_000],
        [512_100, 5_403_050],
      ],
      216,
    );
    expect(p([512_000, 5_403_050])).toEqual([8, 58]);
    expect(p([512_100, 5_403_000])).toEqual([208, 158]);
  });

  it("zeigt kurze Flüge in Sekunden", () => {
    const min = (ms: number) => `${ms / 60_000} min`;
    expect(flightTime(42.4, min)).toBe("42 s");
    expect(flightTime(0.2, min)).toBe("1 s");
    expect(flightTime(120, min)).toBe("2 min");
  });

  it("schätzt die Schwankung der Bodenauflösung", () => {
    // 3 m Gefälle bei 75 m Höhe, Start in der Mitte: ± 1,5 / 75 = ± 2 %
    const s = gsdSpread({ min: 298, max: 301 }, 75);
    expect(s.relief).toBe(3);
    expect(s.percent).toBeCloseTo(2, 9);
  });
});
