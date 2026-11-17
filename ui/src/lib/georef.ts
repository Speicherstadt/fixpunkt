/**
 * Passpunkte in der Web-UI: Liste aus Text lesen (wie `fixpunkt georef --gcp`: Name, x/y/z in der Aufnahme,
 * x/y/z Soll; Komma, Semikolon, Tab oder Leerzeichen), Eingabefelder prüfen, Ergebnis der Server-Vorschau.
 */
import { z } from "zod";

export type Gcp = { name: string; src: [number, number, number]; dst: [number, number, number] };

export const MODES = ["similarity", "rigid", "translation"] as const;
export type Mode = (typeof MODES)[number];

/** Mindestzahl der Punkte je Verfahren (wie im Rechenkern) */
export const MIN_POINTS: Record<Mode, number> = { similarity: 3, rigid: 3, translation: 1 };

/** Zahl aus einer Eingabe, Komma oder Punkt; null, wenn keine */
export const num = (s: string): number | null => {
  const t = s.trim().replace(",", ".");
  if (t === "") return null;
  const v = Number(t);
  return Number.isFinite(v) ? v : null;
};

/** Zahl für ein Eingabefeld, ohne Tausendertrennung, mit dem Dezimalzeichen der Sprache */
export const toInput = (v: number, decimal: "," | "."): string => String(v).replace(".", decimal);

/**
 * Zeilen „Name x y z X Y Z“ → Passpunkte. Dezimalkomma geht, wenn Felder mit Semikolon oder Tab getrennt sind.
 * Zeilen ohne sechs Zahlen (Kopfzeile, Leerzeilen) werden übersprungen.
 */
export const parseGcps = (text: string): Gcp[] =>
  text.split(/\r?\n/).flatMap((line) => {
    const sep = /[;\t]/.test(line) ? /\s*[;\t]\s*/ : /[\s,]+/;
    const f = line.trim().split(sep).filter(Boolean);
    if (f.length < 7) return [];
    const v = f.slice(1, 7).map(num);
    if (v.some((x) => x === null)) return [];
    const [a, b, c, d, e, g] = v as number[];
    return [{ name: f[0]!, src: [a!, b!, c!], dst: [d!, e!, g!] }];
  });

export const FitReport = z.object({
  mode: z.enum(MODES),
  points: z.number(),
  scale_ppm: z.number(),
  rotation_deg: z.tuple([z.number(), z.number(), z.number()]),
  translation: z.tuple([z.number(), z.number(), z.number()]),
  residuals: z.array(
    z.object({
      name: z.string(),
      dx: z.number(),
      dy: z.number(),
      dz: z.number(),
      d2: z.number(),
      d3: z.number(),
    }),
  ),
  rms_xy: z.number(),
  rms_z: z.number(),
  rms_3d: z.number(),
  max_3d: z.number(),
});
export type FitReport = z.infer<typeof FitReport>;

/** Ergebnis des Auftrags: Bericht und die neue Aufnahme */
export const GeorefResult = FitReport.extend({ captureId: z.number() });
export type GeorefResult = z.infer<typeof GeorefResult>;

/** Restklaffe, ab der ein Punkt auffällt: dreifaches RMS, mindestens 5 cm */
export const isOutlier = (d3: number, rms: number): boolean => d3 > Math.max(3 * rms, 0.05);
