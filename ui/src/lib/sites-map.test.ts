import { describe, expect, it } from "vitest";
import type { SiteItem } from "@/lib/api";
import { bounds, mapSites } from "@/lib/sites-map";

const site = (id: number, center: [number, number] | null): SiteItem => ({
  id,
  name: `Baustelle ${id}`,
  description: "",
  epsg: 25832,
  createdAt: 0,
  captures: 2,
  jobs: 0,
  lastCaptureAt: null,
  center,
});

describe("sites-map", () => {
  it("zeigt nur Baustellen mit Lage", () => {
    const sites = mapSites([site(1, [48.78, 9.16]), site(2, null), site(3, [49.0, 8.4])]);
    expect(sites.map((s) => s.id)).toEqual([1, 3]);
    expect(sites[0]).toMatchObject({ lat: 48.78, lon: 9.16, captures: 2 });
  });

  it("Ausschnitt über alle Baustellen", () => {
    expect(bounds(mapSites([site(1, [48.78, 9.16]), site(3, [49.0, 8.4])]))).toEqual([
      [48.78, 8.4],
      [49.0, 9.16],
    ]);
    expect(bounds([])).toBeNull();
  });
});
