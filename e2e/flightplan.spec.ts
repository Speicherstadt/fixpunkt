import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { X0, Y0 } from "./fixtures";
import { section } from "./site";

/**
 * Flugplan über einem 40 × 40 m großen Bereich: M4E bei 2 cm → 75,0 m Höhe, Bahnabstand 31,68 m,
 * ⌈40 / 31,68⌉ + 1 = 3 Bahnen; bei 1 cm halbe Höhe (37,5 m) und ⌈40 / 15,84⌉ + 1 = 4 Bahnen.
 */
test("Flugplan als KMZ für DJI Pilot 2", async ({ page }) => {
  await page.goto("/");
  const api = await apiFor(page);
  const site = (await api.call("POST", "/sites", { name: "Flugplanung" })) as { id: number };
  await api.call("POST", `/sites/${site.id}/regions`, {
    name: "Feld",
    geojson: {
      type: "Polygon",
      crs: { type: "name", properties: { name: "urn:ogc:def:crs:EPSG::25832" } },
      coordinates: [
        [
          [X0, Y0],
          [X0 + 40, Y0],
          [X0 + 40, Y0 + 40],
          [X0, Y0 + 40],
          [X0, Y0],
        ],
      ],
    },
  });
  await page.goto(`/sites/${site.id}`);

  const flight = await section(page, "Flugplanung");
  await expect(flight).toContainText("Flughöhe über Start75,0 m");
  await expect(flight.getByRole("img", { name: /3 Bahnen über „Feld“/ })).toBeVisible();

  await flight.getByLabel("Bodenauflösung in cm").fill("1");
  await expect(flight).toContainText("Flughöhe über Start37,5 m");
  await expect(flight.getByRole("img", { name: /4 Bahnen/ })).toBeVisible();

  const [download] = await Promise.all([
    page.waitForEvent("download"),
    flight.getByRole("link", { name: "KMZ für DJI Pilot 2" }).click(),
  ]);
  expect(download.suggestedFilename()).toMatch(/^flugplan-Flugplanung-Feld-\d+\.kmz$/);
  const bytes = readFileSync(await download.path());
  expect(bytes.subarray(0, 4).toString("hex")).toBe("504b0304");
  expect(bytes.includes("wpmz/waylines.wpml")).toBe(true);

  await flight.getByLabel("Bodenauflösung in cm").fill("50");
  await expect(flight.getByText("Die Eingabe ist ungültig.")).toBeVisible();
});
