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

/**
 * Lager: „Grube“ (30 × 30 m) als Haufwerk-Bereich, Basis 299 m. Vorher 900 m² · 1,2 m = 1.080 m³; nachher fehlen in
 * der Grube 400 m² · 1,2 m = 480 m³ über der Basis → 600 m³, −480 m³.
 */
test("Bestand je Material im Überblick", async ({ page }) => {
  const api = await apiFor(page);
  const site = await api.call("POST", "/sites", { name: "Lagerbestand" });
  const day = 86_400_000;
  const caps = [
    await api.upload(site.id, "u.xyz", "Woche 1", day, terrainXyz(false)),
    await api.upload(site.id, "a.xyz", "Woche 2", 8 * day, terrainXyz(true)),
  ];
  await api.call("POST", `/sites/${site.id}/regions`, { geojson: boundaries() });
  await api.settled(site.id);
  await page.goto(`/sites/${site.id}`);

  const regions = page.getByRole("region", { name: "Bereiche" });
  await regions.getByRole("button", { name: "Material und Massenziel für Grube" }).click();
  const form = regions.getByRole("form", { name: "Massenziel für Grube" });
  await form.getByLabel("Material").fill("Boden Klasse 4");
  await form.getByLabel("Zählt").selectOption({ label: "Lager (Haufwerk)" });
  await expect(form.getByLabel("Soll-Menge in m³")).toHaveCount(0);
  await form.getByRole("button", { name: "Ziel speichern" }).click();
  await expect(regions.getByRole("listitem").filter({ hasText: "Grube" })).toContainText(
    "Boden Klasse 4 · Lager",
  );
  const overview = page.getByRole("region", { name: "Überblick" });
  await expect(overview).toContainText("Noch keine Haufwerk-Berechnung für die Lager.");

  const grube = (
    (await api.call("GET", `/sites/${site.id}/regions`)) as { id: number; name: string }[]
  ).find((r) => r.name === "Grube")!;
  for (const captureId of caps)
    await api.call("POST", `/sites/${site.id}/jobs`, {
      kind: "stockpile",
      params: { captureId, regionIds: [grube.id], base: 299 },
    });
  await api.settled(site.id);
  const row = overview.getByRole("table", { name: "Bestand je Material" }).getByRole("row", {
    name: /Boden Klasse 4/,
  });
  await expect(row).toContainText(/(59\d|60\d) m³/);
  await expect(row).toContainText(/−(47\d|48\d) m³/);
  await expect(row).toContainText("09.01.1970");
  // Verlauf: Woche 1 und Woche 2
  const history = overview
    .getByRole("region", { name: "Verlauf Boden Klasse 4" })
    .getByRole("listitem");
  await expect(history).toHaveCount(2);
  await expect(history.nth(0)).toContainText(/02\.01\.1970.*1\.0(7\d|8\d) m³/);
  await expect(history.nth(1)).toContainText(/09\.01\.1970.*(59\d|60\d) m³/);
});
