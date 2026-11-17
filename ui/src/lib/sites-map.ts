/** Baustellen auf der Übersichtskarte: nur die mit Lage, und der Ausschnitt, der alle zeigt. */
import type { SiteItem } from "@/lib/api";

export type MapSite = { id: number; name: string; lat: number; lon: number; captures: number };

export const mapSites = (sites: SiteItem[]): MapSite[] =>
  sites.flatMap((s) =>
    s.center
      ? [{ id: s.id, name: s.name, lat: s.center[0], lon: s.center[1], captures: s.captures }]
      : [],
  );

/** [[Süd, West], [Nord, Ost]]; null ohne Baustellen */
export const bounds = (sites: MapSite[]): [[number, number], [number, number]] | null => {
  if (!sites.length) return null;
  const lats = sites.map((s) => s.lat);
  const lons = sites.map((s) => s.lon);
  return [
    [Math.min(...lats), Math.min(...lons)],
    [Math.max(...lats), Math.max(...lons)],
  ];
};
