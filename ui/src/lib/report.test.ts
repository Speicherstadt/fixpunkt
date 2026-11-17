import { describe, expect, it } from "vitest";
import type { RegionReport, VolumeReport } from "@/lib/api";
import { missingShare, needsCoverageWarning, reportCsv, reportRows } from "@/lib/report";

const vol = (cut: number, fill: number, used = 100, missing = 0): VolumeReport => ({
  area_m2: used * 0.0625,
  cut_m3: cut,
  fill_m3: fill,
  net_m3: fill - cut,
  cells_used: used,
  cells_missing: missing,
  cell_size_m: 0.25,
});

const region = (name: string, area: number, v: VolumeReport): RegionReport => ({
  ...v,
  name,
  boundary_area_m2: area,
});

describe("report", () => {
  it("Zeilen je Bereich und Summe mit Grenzfläche", () => {
    const { rows, total } = reportRows(
      [region("Grube", 900, vol(800, 1)), region("Rand", 108, vol(0.2, 0.1))],
      vol(800.2, 1.1),
      "Summe",
    );
    expect(rows.map((r) => r.name)).toEqual(["Grube", "Rand"]);
    expect(rows[0]?.area).toBe(900);
    expect(total.area).toBe(1008);
    expect(total.net).toBeCloseTo(-799.1);
  });

  it("ohne Bereiche nur die Summe mit gerasterter Fläche", () => {
    const { rows, total } = reportRows([], vol(10, 0, 160), "Summe");
    expect(rows).toEqual([]);
    expect(total.area).toBe(10);
  });

  it("warnt bei mehr als 5 % Lücken", () => {
    expect(missingShare(vol(1, 0, 90, 10))).toBe(0.1);
    expect(missingShare(vol(1, 0, 0, 0))).toBe(0);
    const ok = reportRows([region("A", 1, vol(1, 0, 96, 4))], vol(1, 0), "S");
    expect(needsCoverageWarning(ok.rows)).toBe(false);
    const bad = reportRows([region("A", 1, vol(1, 0, 90, 10))], vol(1, 0), "S");
    expect(needsCoverageWarning(bad.rows)).toBe(true);
  });

  it("CSV mit Semikolon und Komma", () => {
    const { rows, total } = reportRows(
      [region("Grube", 900, vol(800, 1.5))],
      vol(800, 1.5),
      "Summe",
    );
    const csv = reportCsv(rows, total, ["Bereich", "Fläche", "Abtrag", "Auftrag", "Bilanz"]);
    expect(csv.split("\n")).toEqual([
      "Bereich;Fläche;Abtrag;Auftrag;Bilanz",
      "Grube;900,00;800,00;1,50;-798,50",
      "Summe;900,00;800,00;1,50;-798,50",
    ]);
  });
});

describe("Soll/Ist und Zeitreihe", () => {
  it("Histogramm-Beschriftung in cm", async () => {
    const { binLabel, binSide } = await import("@/lib/report");
    expect(binLabel(null, -0.48)).toBe("< −48");
    expect(binLabel(-0.03, 0.03)).toBe("−3 … 3");
    expect(binLabel(0.48, null)).toBe("> 48");
    expect(binLabel(0.015, 0.03)).toBe("1,5 … 3");
    expect(binSide(-0.03, 0.03, 0.03)).toBe("ok");
    expect(binSide(0.03, 0.06, 0.03)).toBe("high");
    expect(binSide(null, -0.48, 0.03)).toBe("low");
    expect(binSide(0.48, null, 0.03)).toBe("high");
  });

  it("Zeitreihe mit Anteil für die Balken", async () => {
    const { seriesRows } = await import("@/lib/report");
    const rows = seriesRows([
      { label: "KW 10", since_start: vol(400, 0), since_previous: vol(400, 0) },
      { label: "KW 12", since_start: vol(800, 0), since_previous: vol(400, 0) },
    ]);
    expect(rows.map((r) => [r.label, r.total, r.step, r.share])).toEqual([
      ["KW 10", 400, 400, 0.5],
      ["KW 12", 800, 400, 1],
    ]);
    expect(seriesRows([])).toEqual([]);
  });
});

describe("Zentimeter", () => {
  it("ohne negative Null", async () => {
    const { cm } = await import("@/components/report/compare-report");
    expect(cm(-0.0001)).toBe("0,0 cm");
    expect(cm(0.053)).toBe("5,3 cm");
    expect(cm(-0.079)).toBe("-7,9 cm");
  });
});
