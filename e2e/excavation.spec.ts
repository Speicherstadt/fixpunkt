import { expect, test } from "@playwright/test";
import { PIT_M3, writeFixtures } from "./fixtures";

/** Der Kernablauf: Baustelle anlegen, zwei Aufnahmen und Grenzen hochladen, Aushub berechnen, Bericht lesen. */
test("Aushub einer Baugrube berechnen", async ({ page }, info) => {
  const files = writeFixtures(info.outputPath("daten"));
  await page.goto("/");
  await page.getByRole("button", { name: "Neue Baustelle" }).click();
  const dialog = page.getByRole("dialog", { name: "Neue Baustelle" });
  await dialog.getByLabel("Name").fill("Baugrube Kirchstraße");
  await dialog.getByRole("button", { name: "Baustelle anlegen" }).click();
  await expect(page.getByRole("heading", { name: "Baugrube Kirchstraße", level: 1 })).toBeVisible();

  // Aufnahmen hochladen – Prüfung läuft im Hintergrund, die Liste aktualisiert sich live (SSE)
  await page.getByLabel("Aufnahme hochladen").setInputFiles([files.before, files.after]);
  const table = page.getByRole("table");
  await expect(table.getByRole("row", { name: /Urgelaende/ }).getByText("Bereit")).toBeVisible();
  await expect(table.getByRole("row", { name: /Nach Aushub/ }).getByText("Bereit")).toBeVisible();
  await expect(table.getByRole("row", { name: /Urgelaende/ })).toContainText("40.000");

  // Grenzen
  await page.getByLabel("Grenze hochladen").setInputFiles(files.regions);
  await expect(page.getByText("900,0 m²")).toBeVisible();
  await expect(page.getByRole("checkbox", { name: "Bagger", exact: true })).toBeChecked();

  // Vorauswahl stimmt (älteste → jüngste), Bodenfilter ist an
  await expect(page.getByLabel("Urgelände (vorher)")).toHaveValue(/\d+/);
  await expect(page.getByLabel("Bagger, Container und Bewuchs herausfiltern")).toBeChecked();
  await page.getByLabel("Urgelände (vorher)").selectOption({ label: "Urgelaende" });
  await page.getByLabel("Nach dem Aushub").selectOption({ label: "Nach Aushub" });
  await page.getByRole("button", { name: "Aushub berechnen" }).click();

  // Ergebnis in der Liste, dann der Bericht
  const job = page.getByRole("link", { name: /Berechnung \d+/ });
  await expect(job.getByText("Fertig")).toBeVisible({ timeout: 45_000 });
  await job.click();
  await expect(page.getByRole("heading", { name: "Mengenbericht" })).toBeVisible();
  const report = page.getByRole("table", { name: "Massen je Bereich" });
  const grube = report.getByRole("row", { name: /Grube/ });
  const cut = await grube.getByRole("cell").nth(1).textContent();
  const value = Number(cut!.replace(/[^\d,]/g, "").replace(",", "."));
  expect(Math.abs(value - PIT_M3)).toBeLessThan(10);
  // der Bagger ist herausgefiltert: kein nennenswerter Auftrag in seinem Bereich
  const bagger = report.getByRole("row", { name: /Bagger/ });
  const fill = await bagger.getByRole("cell").nth(2).textContent();
  expect(Number(fill!.replace(/[^\d,]/g, "").replace(",", "."))).toBeLessThan(2);

  // unter dem Bagger wurde gemessen (nur eben der Bagger): aufgefüllt, also keine Lücken-Warnung
  await expect(page.getByRole("note")).toHaveCount(0);

  // Downloads
  const csv = page.waitForEvent("download");
  await page.getByRole("link", { name: "Als CSV" }).click();
  expect((await csv).suggestedFilename()).toMatch(/^mengen-\d+\.csv$/);
  const tif = await page.request.get(await page.getByRole("link", { name: /Differenzraster/ }).getAttribute("href") ?? "");
  expect(tif.status()).toBe(200);
  expect((await tif.body()).subarray(0, 2).toString()).toMatch(/II|MM/);
  await page.screenshot({ path: info.outputPath("bericht.png"), fullPage: true });
});
