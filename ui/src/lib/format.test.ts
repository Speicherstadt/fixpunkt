import { describe, expect, it } from "vitest";
import {
  dateInputToMs,
  formatArea,
  formatBytes,
  formatCount,
  formatDay,
  formatDuration,
  formatEuro,
  formatHeight,
  formatVolume,
  msToDateInput,
  parseDecimal,
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

describe("Abrechnung: Euro, Tag, Eingaben", () => {
  it("Euro und Tag in UTC", () => {
    expect(formatEuro(12740)).toBe("12.740,00\u00a0€");
    // Mitternacht UTC bleibt derselbe Tag, egal in welcher Zeitzone
    expect(formatDay(Date.UTC(2026, 10, 17))).toBe("17.11.2026");
  });

  it("Zahlen aus Eingabefeldern", () => {
    expect(parseDecimal("1.234,5")).toBe(1234.5);
    expect(parseDecimal("12,50")).toBe(12.5);
    expect(parseDecimal("12.5")).toBe(12.5);
    expect(parseDecimal("1.000")).toBe(1000);
    expect(parseDecimal(" 15 ")).toBe(15);
    expect(parseDecimal("")).toBeUndefined();
    expect(parseDecimal("viel")).toBeUndefined();
  });
});

describe("Datumsfelder", () => {
  it("Hin und zurück ohne Tageswechsel", () => {
    expect(new Date(dateInputToMs("2026-11-17")).getDate()).toBe(17);
    expect(msToDateInput(dateInputToMs("2026-11-03"))).toBe("2026-11-03");
    expect(msToDateInput(new Date(2026, 0, 5, 23, 59).getTime())).toBe("2026-01-05");
  });
});
