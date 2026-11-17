import { describe, expect, it } from "vitest";
import {
  formatArea,
  formatBytes,
  formatCount,
  formatDuration,
  formatHeight,
  formatVolume,
} from "@/lib/format";

describe("format (deutsch)", () => {
  it("Mengen mit einer Nachkommastelle und Tausenderpunkt", () => {
    expect(formatVolume(25600.96)).toBe("25.601,0 m³");
    expect(formatVolume(-12.34)).toBe("-12,3 m³");
    expect(formatArea(900)).toBe("900,0 m²");
    expect(formatHeight(312.456)).toBe("312,46 m");
    expect(formatCount(4000000)).toBe("4.000.000");
  });

  it("Dauer auf Minuten", () => {
    expect(formatDuration(61_000)).toBe("1 min");
    expect(formatDuration(65 * 60_000)).toBe("1 h 05 min");
    expect(formatDuration(0)).toBe("0 min");
  });

  it("Dateigrößen in 1000er-Schritten", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(3_481_531)).toBe("3,5 MB");
    expect(formatBytes(20_000_000_000)).toBe("20,0 GB");
  });
});
