/** Eigener Bericht: Bausteine (wie crates/server/src/report/custom.rs), Adresse des PDFs, gemerkte Auswahl */
import type { LayoutItem } from "@/lib/layout";

export const REPORT_PARTS = [
  "kpis",
  "progress",
  "targets",
  "stock",
  "history",
  "compare",
  "attention",
  "constraints",
  "lines",
  "notes",
  "photos",
] as const;

export type ReportSettings = { parts: LayoutItem[]; title: string; company: string };

export const defaultReport = (): ReportSettings => ({
  parts: REPORT_PARTS.map((id) => ({ id, visible: true })),
  title: "",
  company: "",
});

/** Adresse des PDFs mit den gewählten Bausteinen in ihrer Reihenfolge; `null` ohne Baustein */
export const reportHref = (siteId: number, s: ReportSettings): string | null => {
  const parts = s.parts.filter((p) => p.visible).map((p) => p.id);
  if (!parts.length) return null;
  const q = new URLSearchParams({ parts: parts.join(",") });
  if (s.title.trim()) q.set("title", s.title.trim());
  if (s.company.trim()) q.set("company", s.company.trim());
  return `/api/sites/${siteId}/report?${q}`;
};

const KEY = "fixpunkt.report";

/** Gemerkte Auswahl dieses Browsers (nur Bequemlichkeit: fehlt sie, gilt die Grundeinstellung) */
export const loadReport = (): ReportSettings => {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "null") as Partial<ReportSettings> | null;
    if (!raw) return defaultReport();
    const known = (raw.parts ?? []).filter((p) =>
      (REPORT_PARTS as readonly string[]).includes(p.id),
    );
    const missing = REPORT_PARTS.filter((id) => !known.some((p) => p.id === id)).map((id) => ({
      id,
      visible: true,
    }));
    return { parts: [...known, ...missing], title: raw.title ?? "", company: raw.company ?? "" };
  } catch {
    return defaultReport();
  }
};

export const saveReport = (s: ReportSettings) => {
  try {
    localStorage.setItem(KEY, JSON.stringify(s));
  } catch {
    // privates Fenster o. Ä.: dann eben nicht gemerkt
  }
};
