import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { boundaries, terrainXyz } from "./fixtures";
import { section } from "./site";

/**
 * Die Berechnungsarten neben dem Aushub über die Oberfläche: Haufwerk, Soll/Ist, Zeitreihe, Export.
 * Die Baustelle mit Urgelände (eben, 1 % Gefälle) und Grube (2 m tief) legt die API an.
 */
test("Haufwerk, Soll/Ist, Zeitreihe und Export", async ({ page }) => {
  test.setTimeout(120_000);
  const api = await apiFor(page);
  const site = await api.call("POST", "/sites", { name: "Rechenarten" });
  const day = 86_400_000;
  await api.upload(site.id, "u.xyz", "Urgelände", day, terrainXyz(false));
  await api.upload(site.id, "a.xyz", "Aushub", 2 * day, terrainXyz(true));
  await api.call("POST", `/sites/${site.id}/regions`, { geojson: boundaries() });
  await api.settled(site.id);
  await page.goto(`/sites/${site.id}`);
  const calc = await section(page, "Berechnungen");
  const kinds = calc.getByRole("group", { name: "Art der Berechnung" });
  const jobs = page.getByRole("link", { name: /·|Zeitreihe/ });
  /** Art wählen wie ein Nutzer: auf die Beschriftung klicken */
  const choose = async (kind: string) => {
    await kinds.getByText(kind, { exact: true }).click();
    await expect(kinds.getByRole("radio", { name: kind })).toBeChecked();
  };

  // Soll/Ist: Grubensohle bei 298,0 + 1 % Gefälle gegen Sollhöhe 298,2 im Bereich „Grube“
  await choose("Soll/Ist");
  await calc.getByLabel("Ist-Aufnahme").selectOption({ label: "Aushub" });
  await calc.getByLabel("Sollhöhe in m").fill("298,2");
  await calc
    .getByRole("combobox", { name: "Bereich", exact: true })
    .selectOption({ label: "Grube" });
  await calc.getByLabel("Toleranz in cm").fill("5");
  await calc.getByRole("button", { name: "Soll/Ist vergleichen" }).click();
  await expect(jobs.filter({ hasText: "Soll/Ist · Aushub" }).getByText("Fertig")).toBeVisible({
    timeout: 45_000,
  });

  // Haufwerk: Urgelände gegen feste Höhe 299 m im Bereich „Grube“ (30 × 30 m, 1,0–1,35 m darüber)
  await choose("Haufwerk");
  await calc
    .getByRole("combobox", { name: "Aufnahme", exact: true })
    .selectOption({ label: "Urgelände" });
  await calc.getByLabel("Basisfläche").selectOption("height");
  await calc.getByLabel("Höhe der Basis in m").fill("299");
  await calc.getByRole("checkbox", { name: "Bagger", exact: true }).uncheck();
  await calc.getByRole("button", { name: "Haufwerk berechnen" }).click();
  await expect(jobs.filter({ hasText: "Haufwerk · Urgelände" }).getByText("Fertig")).toBeVisible({
    timeout: 45_000,
  });

  // Zeitreihe aus beiden Aufnahmen
  await choose("Zeitreihe");
  await calc.getByRole("button", { name: "Zeitreihe berechnen" }).click();
  await expect(
    jobs.filter({ hasText: "Zeitreihe aus 2 Aufnahmen" }).getByText("Fertig"),
  ).toBeVisible({ timeout: 45_000 });

  // Export
  await choose("Export");
  await calc.getByLabel("Rasterpunkte (CSV)").check();
  await calc.getByRole("button", { name: "Exportieren" }).click();
  await expect(jobs.filter({ hasText: "Export · Aushub" }).getByText("Fertig")).toBeVisible({
    timeout: 45_000,
  });

  // Ergebnisse ansehen
  await jobs.filter({ hasText: "Soll/Ist" }).click();
  await expect(page.getByRole("heading", { name: "Soll/Ist · Aushub", level: 1 })).toBeVisible();
  // Bereich „Grube“ ist 30 × 30 m = 900 m² samt Rand; in ± 5 cm liegt nur die Sohle bei dx 15…25:
  // 10 × 20 m = 200 m² → 22 %
  const within = page
    .getByRole("term")
    .filter({ hasText: "In Toleranz" })
    .locator("xpath=following-sibling::dd");
  await expect(within).toHaveText(/^2[1-3] %$/);
  await expect(
    page.getByRole("list", { name: "Verteilung der Abweichung" }).getByRole("listitem"),
  ).toHaveCount(11);
  await page.goBack();

  await jobs.filter({ hasText: "Haufwerk" }).click();
  // Urgelände 300 + 0,01·dx (dx 5…35) gegen 299 auf 900 m²: 900 × 1,2 = 1.080 m³
  const volume = page
    .getByRole("term")
    .filter({ hasText: /^Volumen$/ })
    .locator("xpath=following-sibling::dd");
  await expect(volume).toHaveText(/^1\.0[78]\d,\d m³$/);
  await page.goBack();

  await jobs.filter({ hasText: "Zeitreihe" }).click();
  await expect(
    page.getByRole("table", { name: "Fortschritt" }).getByRole("row", { name: /Aushub/ }),
  ).toContainText("m³");
  await page.goBack();

  await jobs.filter({ hasText: "Export" }).click();
  const csv = page
    .getByRole("listitem")
    .filter({ hasText: "dgm.csv" })
    .getByRole("link", { name: "Herunterladen" });
  const res = await page.request.get((await csv.getAttribute("href"))!);
  expect((await res.text()).startsWith("x;y;z")).toBe(true);
  await expect(page.getByRole("listitem").filter({ hasText: "dgm.dxf" })).toBeVisible();
});
