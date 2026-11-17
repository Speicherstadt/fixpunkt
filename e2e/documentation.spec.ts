import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { terrainXyz } from "./fixtures";

/** Dokumentation: Foto mit GPS landet an seiner Stelle, Zwangspunkt wird gegen das Gelände geprüft. */
test("Fotos und Zwangspunkte dokumentieren", async ({ page }) => {
  await page.goto("/");
  const api = await apiFor(page);
  const site = (await api.call("POST", "/sites", { name: "Dokumentation" })) as { id: number };
  await api.upload(site.id, "aushub.xyz", "Aushub", Date.now(), terrainXyz(true));
  await api.settled(site.id);
  await page.goto(`/sites/${site.id}`);

  const doc = page.getByRole("region", { name: "Dokumentation" });
  await expect(doc.getByText(/Noch nichts dokumentiert/)).toBeVisible();
  await doc.getByLabel("Fotos hochladen").setInputFiles("e2e/data/foto-gps.jpg");
  const photo = doc.getByRole("listitem").filter({ hasText: "foto-gps" });
  await expect(photo).toContainText("E 512025,00 · N 5403015,00");
  // GPSImgDirection 135,5° geografisch, im Gitter 135,38°
  await expect(photo).toContainText("Blickrichtung 135° (SO)");
  await expect(photo.getByRole("img", { name: "foto-gps" })).toBeVisible();

  // Zwangspunkt mitten in der Grube: Gelände 298,20 m, Soll 298,15 m → 5 cm zu hoch
  await doc.getByText("Zwangspunkt oder Notiz anlegen").click();
  await doc.getByLabel("Bezeichnung").fill("Kanalsohle Bestand");
  await doc.getByLabel("Ost (E) in m").fill("512020");
  await doc.getByLabel("Nord (N) in m").fill("5403020");
  await doc.getByLabel("Sollhöhe in m").fill("298,15");
  await doc.getByRole("button", { name: "Speichern" }).click();
  const point = doc.getByRole("listitem").filter({ hasText: "Kanalsohle Bestand" });
  await expect(point).toContainText("Soll 298,15 m");
  await expect(point).toContainText("Ist 298,20 m (5,0 cm)");

  // Bericht als PDF mit dem Zwangspunkt
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    doc.getByRole("link", { name: "Bericht (PDF)" }).click(),
  ]);
  expect(download.suggestedFilename()).toMatch(/^Dokumentation-.*\.pdf$/);
  expect(
    readFileSync(await download.path())
      .subarray(0, 5)
      .toString(),
  ).toBe("%PDF-");

  // löschen
  await photo.getByRole("button", { name: "foto-gps löschen" }).click();
  await expect(photo).toHaveCount(0);
});
