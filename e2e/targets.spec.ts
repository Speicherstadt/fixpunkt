import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { boundaries, terrainXyz } from "./fixtures";

/**
 * Massenziel: im Bereich „Grube“ (30 × 30 m) liegt die Grube mit 20 × 20 × 2 m = 800 m³. Ziel 1.000 m³ Abtrag →
 * 80 %, offen 200 m³.
 */
test("Massenziel je Bereich im Überblick", async ({ page }) => {
  const api = await apiFor(page);
  const site = await api.call("POST", "/sites", { name: "Massenziele" });
  const before = await api.upload(site.id, "u.xyz", "Urgelände", 1, terrainXyz(false));
  const after = await api.upload(site.id, "a.xyz", "Aushub", 2, terrainXyz(true));
  await api.call("POST", `/sites/${site.id}/regions`, { geojson: boundaries() });
  await api.settled(site.id);
  await page.goto(`/sites/${site.id}`);

  const regions = page.getByRole("region", { name: "Bereiche" });
  await regions.getByRole("button", { name: "Material und Massenziel für Grube" }).click();
  const form = regions.getByRole("form", { name: "Massenziel für Grube" });
  await form.getByLabel("Material").fill("Boden Klasse 4");
  await form.getByLabel("Soll-Menge in m³").fill("1.000");
  await form.getByRole("button", { name: "Ziel speichern" }).click();
  await expect(regions.getByRole("listitem").filter({ hasText: "Grube" })).toContainText(
    "Boden Klasse 4 · Ziel 1.000 m³ Abtrag",
  );

  const overview = page.getByRole("region", { name: "Überblick" });
  const targets = overview.getByRole("list", { name: "Massenziele" });
  await expect(targets).toContainText("noch nicht berechnet");

  const regionIds = ((await api.call("GET", `/sites/${site.id}/regions`)) as { id: number }[]).map(
    (r) => r.id,
  );
  await api.call("POST", `/sites/${site.id}/jobs`, {
    kind: "volume",
    params: { before, after, regionIds },
  });
  await api.settled(site.id);
  await expect(targets.getByRole("listitem").filter({ hasText: "Grube" })).toContainText(
    /(799|800|801) von 1\.000 m³ Abtrag · 80 % · offen (199|200|201) m³/,
  );
  await expect(overview).toContainText(/Je Material: Boden Klasse 4 (799|800|801) \/ 1\.000 m³/);
});
