import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { terrainXyz } from "./fixtures";

/** Übersichtskarte: Baustellen mit geprüfter Aufnahme stehen als Punkt darauf, ein Klick öffnet sie. */
test("Karte der Baustellen", async ({ page }) => {
  await page.goto("/");
  const api = await apiFor(page);
  const site = (await api.call("POST", "/sites", { name: "Auf der Karte" })) as { id: number };
  await api.upload(site.id, "gelaende.xyz", "Urgelände", Date.now(), terrainXyz(false));
  await api.settled(site.id);

  await page.goto("/");
  const map = page.getByRole("region", { name: "Karte der Baustellen" });
  await expect(map).toBeVisible();
  const markers = map.locator(".site-marker");
  await expect(markers.first()).toBeVisible();
  // Tests laufen ohne Kacheln (kein Netz nötig)
  await expect(map.locator(".leaflet-tile")).toHaveCount(0);

  // alle Testbaustellen liegen auf demselben Gelände; der oberste Punkt führt zu einer davon
  await markers.last().hover();
  await expect(map.locator(".leaflet-tooltip")).toBeVisible();
  const name = await map.locator(".leaflet-tooltip").textContent();
  await markers.last().click({ force: true });
  await expect(page).toHaveURL(/\/sites\/\d+$/);
  await expect(page.getByRole("heading", { name: name ?? "", level: 1 })).toBeVisible();
});
