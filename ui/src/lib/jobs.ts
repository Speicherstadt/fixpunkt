/**
 * Was die UI über einen Rechenauftrag zeigt: Titel und Kurzinfo je Art. Reine Funktionen, damit Liste und
 * Ergebnisseite dieselben Worte nutzen.
 */
import { CompareResult, FilesResult, type Job, SeriesResult, VolumeResult } from "@/lib/api";
import { formatVolume } from "@/lib/format";
import { m } from "@/paraglide/messages";

export const JOB_KINDS = ["volume", "stockpile", "compare", "series", "export"] as const;
export type JobKind = (typeof JOB_KINDS)[number];

export const KIND_TEXT: Record<JobKind, () => string> = {
  volume: m.kind_volume,
  stockpile: m.kind_stockpile,
  compare: m.kind_compare,
  series: m.kind_series,
  export: m.kind_export,
};

type Names = (id: unknown) => string;

/** Titel eines Auftrags in Liste und Bericht */
export const jobTitle = (job: Job, names: Names): string => {
  const p = job.params;
  switch (job.kind) {
    case "volume":
      return m.report_compare({ before: names(p.before), after: names(p.after) });
    case "series": {
      const ids = Array.isArray(p.captureIds) ? p.captureIds : [];
      return m.job_title_series({ count: ids.length });
    }
    case "stockpile":
    case "compare":
    case "export":
    case "dem":
      return `${(KIND_TEXT[job.kind as JobKind] ?? m.kind_dem)()} · ${names(p.captureId)}`;
    default:
      return job.kind;
  }
};

/** Kurzinfo zum Ergebnis für die Liste; null, solange es keins gibt */
export const jobSummary = (job: Job): string | null => {
  switch (job.kind) {
    case "volume": {
      const r = VolumeResult.safeParse(job.result);
      return r.success ? m.calc_cut_short({ value: formatVolume(r.data.total.cut_m3) }) : null;
    }
    case "stockpile": {
      const r = VolumeResult.safeParse(job.result);
      return r.success
        ? m.job_summary_stockpile({ value: formatVolume(r.data.total.fill_m3) })
        : null;
    }
    case "compare": {
      const r = CompareResult.safeParse(job.result);
      return r.success
        ? m.job_summary_compare({ percent: Math.round(r.data.deviation.within_tolerance_percent) })
        : null;
    }
    case "series": {
      const r = SeriesResult.safeParse(job.result);
      const last = r.success ? r.data.steps.at(-1) : undefined;
      return last ? m.calc_cut_short({ value: formatVolume(last.since_start.cut_m3) }) : null;
    }
    default: {
      const r = FilesResult.safeParse(job.result);
      return r.success ? m.job_summary_files({ count: r.data.files.length }) : null;
    }
  }
};

/** Aufnahmen für eine Zeitreihe in zeitlicher Reihenfolge, die älteste zuerst */
export const chronological = <T extends { id: number; capturedAt: number }>(items: T[]): T[] =>
  [...items].sort((a, b) => a.capturedAt - b.capturedAt || a.id - b.id);
