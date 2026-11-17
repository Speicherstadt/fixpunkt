/** Dokumentation: Zwangspunkte gegen das Gelände prüfen, Markierungen für die 2D-Ansicht. */
import type { Marker } from "@/lib/api";
import { type HeightGrid, sampleAt } from "@/lib/view/grid";

/** Abstand, ab dem ein Zwangspunkt als verletzt gilt (über oder unter der Sollhöhe) */
export const CONSTRAINT_TOLERANCE = 0.03;

export type ConstraintCheck = {
  /** Gelände an der Stelle in der Aufnahme */
  actual: number;
  /** Ist − Soll in m: positiv heißt, das Gelände liegt höher */
  diff: number;
  ok: boolean;
};

/** Zwangspunkt gegen das Höhenraster; null, wenn er außerhalb liegt oder kein Zwangspunkt ist */
export const checkConstraint = (
  m: Marker,
  grid: HeightGrid | undefined,
): ConstraintCheck | null => {
  if (m.kind !== "constraint" || m.x === null || m.y === null || m.z === null || !grid) return null;
  const actual = sampleAt(grid, m.x, m.y);
  if (actual === null || Number.isNaN(actual)) return null;
  const diff = actual - m.z;
  return { actual, diff, ok: Math.abs(diff) <= CONSTRAINT_TOLERANCE };
};

/** Markierungen mit Lage als Punkte für die Karte der Ansicht (Linien zeichnet `linesOf`) */
export const pinsOf = (markers: Marker[]) =>
  markers.flatMap((m) =>
    m.kind !== "line" && m.x !== null && m.y !== null
      ? [{ label: m.title, at: [m.x, m.y] as [number, number], diamond: m.kind === "constraint" }]
      : [],
  );

/** Linien der Dokumentation für die Ansicht */
export const linesOf = (markers: Marker[]) =>
  markers.flatMap((m) =>
    m.kind === "line" && m.points
      ? [{ name: m.title, points: m.points.map((p) => [p[0]!, p[1]!] as [number, number]) }]
      : [],
  );

/** Länge einer Linie in der Ebene, in m */
export const lineLength = (points: number[][]) =>
  points
    .slice(1)
    .reduce((sum, p, i) => sum + Math.hypot(p[0]! - points[i]![0]!, p[1]! - points[i]![1]!), 0);

/**
 * Stützpunkte einer neuen Linie mit Gelände darunter; ohne Höhe an der Stelle nur Ost und Nord.
 * Höhen auf Millimeter gerundet.
 */
export const tracePoints = (pts: [number, number][], grid: HeightGrid | undefined): number[][] =>
  pts.map(([x, y]) => {
    const z = grid ? sampleAt(grid, x, y) : null;
    return z === null || Number.isNaN(z) ? [x, y] : [x, y, Math.round(z * 1000) / 1000];
  });
