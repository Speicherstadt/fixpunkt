import { expect, test } from "@playwright/test";
import { apiFor } from "./api";

/**
 * Drohnenfotos über NodeODM (Nachbau in e2e/nodeodm-stub.ts): Fotos wählen, hochladen, auswerten; die
 * Punktwolke erscheint als neue Aufnahme und wird geprüft.
 */
test("Drohnenfotos werden zur Aufnahme", async ({ page }) => {
  await page.goto("/");
  const api = await apiFor(page);
  const site = (await api.call("POST", "/sites", { name: "Flug über die Grube" })) as {
    id: number;
  };
  await page.goto(`/sites/${site.id}`);

  const section = page.getByRole("region", { name: "Drohnenfotos auswerten" });
  await expect(section.getByText(/NodeODM 2\.5\.2/)).toBeVisible();
  const submit = section.getByRole("button", { name: "Hochladen und auswerten" });
  await expect(submit).toBeDisabled();
  await expect(section.getByText("Mindestens 3 Fotos auswählen")).toBeVisible();

  const photo = (i: number) => ({
    name: `DJI_${String(i).padStart(4, "0")}.JPG`,
    mimeType: "image/jpeg",
    buffer: Buffer.from([0xff, 0xd8, 0xff, 0xe0, i]),
  });
  await section.getByLabel("Fotos", { exact: true }).setInputFiles([1, 2, 3, 4].map(photo));
  await expect(section.getByText("4 Fotos, 20 B")).toBeVisible();
  await section.getByLabel("Name der Aufnahme").fill("Flug 1");
  await section.getByLabel("Qualität").selectOption({ label: "Schnell (Vorschau)" });
  await section.getByLabel("Höhenversatz in m").fill("abc");
  await expect(section.getByText("Höhenversatz als Zahl in m, z. B. 47,25")).toBeVisible();
  await section.getByLabel("Höhenversatz in m").fill("0");
  await submit.click();
  await expect(section.getByRole("status")).toContainText("Läuft in NodeODM");

  // Auftrag in der Liste, danach die neue Aufnahme – fertig geprüft
  const calc = page.getByRole("region", { name: "Berechnungen" });
  const job = calc.getByRole("link", { name: /Drohnenfotos „Flug 1“/ });
  await expect(job.getByText("Fertig")).toBeVisible({ timeout: 30_000 });
  await expect(job).toContainText("4 Fotos");
  const row = page.getByRole("table").getByRole("row", { name: /Flug 1/ });
  await expect(row.getByText("Bereit")).toBeVisible({ timeout: 30_000 });
  await expect(row).toContainText("40.000");

  // Ergebnisseite
  await job.click();
  await expect(page.getByText("13 min")).toBeVisible();
  await expect(page.getByText("Unverändert übernommen.")).toBeVisible();
  await page.getByRole("link", { name: "Zur Aufnahme" }).click();
  await expect(page.getByRole("heading", { name: "Flug über die Grube", level: 1 })).toBeVisible();
});
