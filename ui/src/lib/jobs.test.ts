import { describe, expect, it } from "vitest";
import type { Job } from "@/lib/api";
import { chronological, jobSummary, jobTitle } from "@/lib/jobs";

const job = (kind: string, params: Record<string, unknown>, result: unknown = null): Job => ({
  id: 1,
  siteId: 1,
  kind,
  params,
  status: result ? "done" : "queued",
  result,
  error: null,
  createdAt: 0,
  startedAt: null,
  finishedAt: null,
});

const names = (id: unknown) => ({ 1: "Urgelände", 2: "Aushub" })[id as 1 | 2] ?? "–";
const vol = (cut: number, fill: number) => ({
  area_m2: 1,
  cut_m3: cut,
  fill_m3: fill,
  net_m3: fill - cut,
  cells_used: 1,
  cells_missing: 0,
  cell_size_m: 0.25,
});

describe("jobs", () => {
  it("Titel je Art", () => {
    expect(jobTitle(job("volume", { before: 1, after: 2 }), names)).toBe("Urgelände → Aushub");
    expect(jobTitle(job("stockpile", { captureId: 2 }), names)).toBe("Haufwerk · Aushub");
    expect(jobTitle(job("compare", { captureId: 2 }), names)).toBe("Soll/Ist · Aushub");
    expect(jobTitle(job("export", { captureId: 9 }), names)).toBe("Export · –");
    expect(jobTitle(job("series", { captureIds: [1, 2, 3] }), names)).toBe(
      "Zeitreihe aus 3 Aufnahmen",
    );
  });

  it("Kurzinfo je Art, erst mit Ergebnis", () => {
    expect(jobSummary(job("volume", {}))).toBeNull();
    const regions = { regions: [], cell: 0.25 };
    expect(jobSummary(job("volume", {}, { ...regions, total: vol(800, 0) }))).toBe(
      "Abtrag 800,0 m³",
    );
    expect(jobSummary(job("stockpile", {}, { ...regions, total: vol(0, 123.45) }))).toBe(
      "Volumen 123,5 m³",
    );
    const deviation = {
      min: 0,
      max: 0,
      mean: 0,
      rms: 0,
      std_dev: 0,
      tolerance: 0.03,
      within_tolerance_percent: 87.4,
      above_tolerance_percent: 10,
      below_tolerance_percent: 2.6,
      histogram: [{ from: null, to: -0.48, cells: 0, percent: 0 }],
      volume: vol(1, 2),
    };
    expect(jobSummary(job("compare", {}, { deviation, region: null, cell: 0.25, files: [] }))).toBe(
      "87 % in Toleranz",
    );
    const steps = [{ label: "KW 10", since_start: vol(400, 0), since_previous: vol(400, 0) }];
    expect(jobSummary(job("series", {}, { base: "U", region: null, steps, cell: 0.25 }))).toBe(
      "Abtrag 400,0 m³",
    );
    expect(jobSummary(job("export", {}, { files: ["dgm.tif", "dgm.dxf"] }))).toBe("2 Dateien");
  });

  it("Zeitreihe ordnet nach Aufnahmedatum", () => {
    const items = [
      { id: 3, capturedAt: 30 },
      { id: 1, capturedAt: 10 },
      { id: 2, capturedAt: 10 },
    ];
    expect(chronological(items).map((i) => i.id)).toEqual([1, 2, 3]);
  });
});
