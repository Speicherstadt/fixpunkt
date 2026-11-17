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
