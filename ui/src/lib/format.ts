import { getLocale } from "@/paraglide/runtime";

const nf = (digits: number) =>
  new Intl.NumberFormat(getLocale(), {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  });

/** Zahl mit fester Zahl Nachkommastellen in der Sprache der Oberfläche */
export const formatNumber = (v: number, digits: number): string => nf(digits).format(v);

/** „1.234,5 m³“ – eine Nachkommastelle, wie im Aufmaß üblich */
export const formatVolume = (m3: number): string => `${nf(1).format(m3)} m³`;

export const formatArea = (m2: number): string => `${nf(1).format(m2)} m²`;

/** Höhe auf Zentimeter */
export const formatHeight = (m: number): string => `${nf(2).format(m)} m`;

export const formatCount = (n: number): string => new Intl.NumberFormat(getLocale()).format(n);

const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** Dateigröße, 1000er-Schritte wie bei Speicherkarten und Downloads */
export const formatBytes = (bytes: number): string => {
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000;
    unit++;
  }
  return unit === 0 ? `${bytes} B` : `${nf(1).format(value)} ${UNITS[unit]}`;
};

export const formatDate = (ms: number): string =>
  new Intl.DateTimeFormat(getLocale(), { dateStyle: "medium" }).format(new Date(ms));

export const formatDateTime = (ms: number): string =>
  new Intl.DateTimeFormat(getLocale(), { dateStyle: "medium", timeStyle: "short" }).format(
    new Date(ms),
  );

/** Dauer auf Minuten: „12 min“, „1 h 05 min“ */
export const formatDuration = (ms: number): string => {
  const min = Math.round(ms / 60_000);
  return min >= 60
    ? `${Math.floor(min / 60)} h ${String(min % 60).padStart(2, "0")} min`
    : `${min} min`;
};

/** Tag ohne Uhrzeit, der als Mitternacht UTC gespeichert ist (Lieferscheine) */
export const formatDay = (ms: number): string =>
  new Intl.DateTimeFormat(getLocale(), { dateStyle: "medium", timeZone: "UTC" }).format(
    new Date(ms),
  );

/** Betrag in Euro: „12.740,00 €“ */
export const formatEuro = (v: number): string =>
  new Intl.NumberFormat(getLocale(), { style: "currency", currency: "EUR" }).format(v);

/**
 * Zahl aus einem Eingabefeld: „1.234,5“ (deutsch), „12,5“, „12.5“ und „1.000“ (Tausenderpunkt);
 * leer oder unlesbar ergibt `undefined`
 */
export const parseDecimal = (s: string): number | undefined => {
  const t = s.trim().replace(/\s/g, "");
  if (!t) return undefined;
  const normalized = t.includes(",")
    ? t.replace(/\./g, "").replace(",", ".")
    : /^-?\d{1,3}(\.\d{3})+$/.test(t)
      ? t.replace(/\./g, "")
      : t;
  const v = Number(normalized);
  return Number.isFinite(v) ? v : undefined;
};
