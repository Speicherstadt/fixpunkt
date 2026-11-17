import { getLocale } from "@/paraglide/runtime";

const nf = (digits: number) =>
  new Intl.NumberFormat(getLocale(), {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  });

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
