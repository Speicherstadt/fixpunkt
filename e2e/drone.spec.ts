import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { section } from "./site";

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

  const sec = await section(page, "Drohnenfotos auswerten");
  await expect(sec.getByText(/NodeODM 2\.5\.2/)).toBeVisible();
  const submit = sec.getByRole("button", { name: "Hochladen und auswerten" });
  await expect(submit).toBeDisabled();
  await expect(sec.getByText("Mindestens 3 Fotos auswählen")).toBeVisible();

  const photo = (i: number) => ({
    name: `DJI_${String(i).padStart(4, "0")}.JPG`,
    mimeType: "image/jpeg",
    buffer: Buffer.from([0xff, 0xd8, 0xff, 0xe0, i]),
  });
  await sec.getByLabel("Fotos", { exact: true }).setInputFiles([1, 2, 3, 4].map(photo));
  await expect(sec.getByText("4 Fotos, 20 B")).toBeVisible();
  await sec.getByLabel("Name der Aufnahme").fill("Flug 1");
  await sec.getByLabel("Qualität").selectOption({ label: "Schnell (Vorschau)" });
  await sec.getByLabel("Höhenversatz in m").fill("abc");
  await expect(sec.getByText("Höhenversatz als Zahl in m, z. B. 47,25")).toBeVisible();
  await sec.getByLabel("Höhenversatz in m").fill("0");
  await submit.click();
  await expect(sec.getByRole("status")).toContainText("Läuft in NodeODM");

  // Auftrag in der Liste, danach die neue Aufnahme – fertig geprüft
  const calc = await section(page, "Berechnungen");
  const job = calc.getByRole("link", { name: /Drohnenfotos „Flug 1“/ });
  await expect(job.getByText("Fertig")).toBeVisible({ timeout: 30_000 });
  await expect(job).toContainText("4 Fotos");
  await section(page, "Aufnahmen");
  const row = page.getByRole("table").getByRole("row", { name: /Flug 1/ });
  await expect(row.getByText("Bereit")).toBeVisible({ timeout: 30_000 });
  await expect(row).toContainText("40.000");

  // Ergebnisseite
  await section(page, "Berechnungen");
  await job.click();
  await expect(page.getByText("13 min")).toBeVisible();
  await expect(page.getByText("Unverändert übernommen.")).toBeVisible();
  await page.getByRole("link", { name: "Zur Aufnahme" }).click();
  await expect(page.getByRole("heading", { name: "Flug über die Grube", level: 1 })).toBeVisible();
});
