import { describe, expect, it } from "vitest";
import type { Capture } from "@/lib/api";
import {
  defaultPeriod,
  deliveredText,
  deliveryUnitLabel,
  priceUnitLabel,
  readyCaptures,
  signedPercent,
  today,
} from "@/lib/billing";

const capture = (id: number, capturedAt: number, status = "ready") =>
  ({ id, capturedAt, status }) as Capture;

describe("Abrechnung", () => {
  it("Zeitraum: die letzten zwei geprüften Befliegungen", () => {
    const cs = [capture(3, 300), capture(1, 100), capture(4, 400, "failed"), capture(2, 200)];
    expect(readyCaptures(cs).map((c) => c.id)).toEqual([1, 2, 3]);
    expect(defaultPeriod(cs)).toEqual({ from: 2, to: 3 });
    expect(defaultPeriod([capture(1, 100)])).toBeNull();
    expect(defaultPeriod([])).toBeNull();
  });

  it("Einheiten und Texte", () => {
    expect(priceUnitLabel("m3")).toBe("m³");
    expect(priceUnitLabel("m3_loose")).toBe("m³ lose");
    // Lieferscheine in m³ sind lose, wie geladen
    expect(deliveryUnitLabel("m3")).toBe("m³ lose");
    expect(
      deliveredText([
        ["t", 600],
        ["loads", 10],
      ]),
    ).toBe("600,0 t + 10 Ladungen");
    expect(signedPercent(15)).toBe("+15,0 %");
    expect(signedPercent(-2.34)).toBe("−2,3 %");
    expect(signedPercent(0)).toBe("±0,0 %");
    expect(today(new Date(Date.UTC(2026, 10, 17, 12)))).toBe("2026-11-17");
  });
});
