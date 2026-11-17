/**
 * Flugplanung in der UI: Einstellungen als Query für /api/regions/{id}/flightplan(.kmz), Vorschau als SVG
 * (Nord oben) und der Hinweis zum Höhenunterschied.
 */
import type { Pt } from "@/lib/view/geometry";

export const DRONES = ["m4e", "m3e"] as const;
export type Drone = (typeof DRONES)[number];

export type FlightSettings = {
  /** Bodenauflösung in cm */
  gsd: number;
  front: number;
  side: number;
  /** Flugrichtung in Grad von Nord */
  direction: number;
  speed: number;
  drone: Drone;
};

export const DEFAULT_FLIGHT: FlightSettings = {
  gsd: 2,
  front: 80,
  side: 70,
  direction: 0,
  speed: 10,
  drone: "m4e",
};

export const flightQuery = (s: FlightSettings): string =>
  new URLSearchParams({
    gsd: String(s.gsd),
    front: String(s.front),
    side: String(s.side),
    direction: String(s.direction),
    speed: String(s.speed),
    drone: s.drone,
  }).toString();

/** Lage (UTM) → SVG-Koordinaten in einem Quadrat der Seite `size`, mit Rand, Nord oben, Maßstab gleich */
export const projector = (pts: Pt[], size: number, pad = 8) => {
  const xs = pts.map((p) => p[0]);
  const ys = pts.map((p) => p[1]);
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const span = Math.max(x1 - x0, y1 - y0, 1e-9);
  const k = (size - 2 * pad) / span;
  // mittig in der kürzeren Richtung
  const ox = pad + (size - 2 * pad - (x1 - x0) * k) / 2;
  const oy = pad + (size - 2 * pad - (y1 - y0) * k) / 2;
  return ([x, y]: Pt): Pt => [ox + (x - x0) * k, oy + (y1 - y) * k];
};

/** Flugzeit: unter einer Minute in Sekunden, sonst wie formatDuration */
export const flightTime = (seconds: number, minutes: (ms: number) => string): string =>
  seconds < 60 ? `${Math.max(1, Math.round(seconds))} s` : minutes(seconds * 1000);

/**
 * Höhenunterschied im Bereich: Die Höhe gilt über dem Startpunkt; bei Start in mittlerer Höhe weicht die
 * Bodenauflösung an den Rändern um ± halben Unterschied / Flughöhe ab.
 */
export const gsdSpread = (terrain: { min: number; max: number }, altitude: number) => {
  const relief = terrain.max - terrain.min;
  return { relief, percent: (relief / 2 / altitude) * 100 };
};
