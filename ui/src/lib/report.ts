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

/** Beschriftung einer Histogrammklasse in cm: „< −48“, „−3 … 3“, „> 48“ (offene Ränder kommen als null) */
export const binLabel = (from: number | null, to: number | null): string => {
  const cm = (v: number) => {
    const r = Math.round(v * 1000) / 10;
    return `${r < 0 ? "−" : ""}${Math.abs(r)}`.replace(".", ",");
  };
  if (from === null && to !== null) return `< ${cm(to)}`;
  if (to === null && from !== null) return `> ${cm(from)}`;
  if (from !== null && to !== null) return `${cm(from)} … ${cm(to)}`;
  return "–";
};

/** Lage einer Klasse zur Toleranz: Mitte = in Toleranz, links = zu tief, rechts = zu hoch */
export const binSide = (
  from: number | null,
  to: number | null,
  tolerance: number,
): "low" | "ok" | "high" => {
  const mid = from === null ? (to ?? 0) - 1 : to === null ? from + 1 : (from + to) / 2;
  if (Math.abs(mid) <= tolerance) return "ok";
  return mid < 0 ? "low" : "high";
};

export type SeriesRow = { label: string; total: number; step: number; net: number; share: number };

/** Zeitreihe: Abtrag seit Beginn und seit der vorigen Aufnahme; `share` 0…1 für den Balken */
export const seriesRows = (
  steps: { label: string; since_start: VolumeReport; since_previous: VolumeReport }[],
): SeriesRow[] => {
  const max = Math.max(0, ...steps.map((s) => s.since_start.cut_m3));
  return steps.map((s) => ({
    label: s.label,
    total: s.since_start.cut_m3,
    step: s.since_previous.cut_m3,
    net: s.since_start.net_m3,
    share: max > 0 ? s.since_start.cut_m3 / max : 0,
  }));
};
