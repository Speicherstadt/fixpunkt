import type { RegionReport, VolumeReport } from "@/lib/api";

/** Anteil der Fläche ohne Daten, ab dem der Bericht warnt (Lücken werden nicht mitgerechnet) */
export const MISSING_WARN = 0.05;

export type ReportRow = {
  name: string;
  area: number;
  cut: number;
  fill: number;
  net: number;
  /** Anteil 0…1 der Zellen im Bereich ohne Daten */
  missing: number;
};

export const missingShare = (r: VolumeReport): number => {
  const all = r.cells_used + r.cells_missing;
  return all === 0 ? 0 : r.cells_missing / all;
};

export const toRow = (name: string, r: VolumeReport, area?: number): ReportRow => ({
  name,
  area: area ?? r.area_m2,
  cut: r.cut_m3,
  fill: r.fill_m3,
  net: r.net_m3,
  missing: missingShare(r),
});

/** Zeilen eines Mengenberichts: je Bereich eine, ohne Bereiche nur die Summe */
export const reportRows = (regions: RegionReport[], total: VolumeReport, totalName: string) => ({
  rows: regions.map((r) => toRow(r.name, r, r.boundary_area_m2)),
  total: toRow(
    totalName,
    total,
    regions.length ? regions.reduce((s, r) => s + r.boundary_area_m2, 0) : undefined,
  ),
});

export const needsCoverageWarning = (rows: ReportRow[]): boolean =>
  rows.some((r) => r.missing > MISSING_WARN);

/** Bericht als CSV (Semikolon, deutsches Excel) */
export const reportCsv = (rows: ReportRow[], total: ReportRow, header: string[]): string => {
  const num = (v: number) => v.toFixed(2).replace(".", ",");
  const line = (r: ReportRow) =>
    [r.name, num(r.area), num(r.cut), num(r.fill), num(r.net)].join(";");
  return [header.join(";"), ...rows.map(line), line(total)].join("\n");
};
