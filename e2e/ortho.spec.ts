import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { terrainXyz } from "./fixtures";
import { section } from "./site";

/** Orthofoto an einer Aufnahme: in der Ansicht als Hintergrund, umschaltbar auf das Höhenbild. */
test("Orthofoto als Hintergrund", async ({ page }) => {
  await page.goto("/");
  const api = await apiFor(page);
  const site = (await api.call("POST", "/sites", { name: "Orthofoto" })) as { id: number };
  await api.upload(site.id, "aushub.xyz", "Aushub", Date.now(), terrainXyz(true));
  await api.settled(site.id);
  await page.goto(`/sites/${site.id}`);
  await section(page, "Aufnahmen");

  const row = page.getByRole("table").getByRole("row", { name: /Aushub/ });
  await row.getByLabel("Orthofoto zu Aushub hinzufügen").setInputFiles("e2e/data/ortho.tif");
  await expect(row).toContainText("Orthofoto 400 × 400 Pixel");

  const view = await section(page, "Ansicht");
  const canvas = view.getByRole("application", { name: /Höhenbild/ }).locator("canvas");
  /** Pixel im Gelb des Baggers auf dem Orthofoto (230, 180, 30) */
  const yellow = () =>
    canvas.evaluate((c: HTMLCanvasElement) => {
      const px = c.getContext("2d")!.getImageData(0, 0, c.width, c.height).data;
      let n = 0;
      for (let i = 0; i < px.length; i += 4)
        if (px[i]! > 200 && px[i + 1]! > 150 && px[i + 1]! < 210 && px[i + 2]! < 70) n++;
      return n;
    });
  await expect.poll(yellow).toBeGreaterThan(100);
  await view.getByRole("group", { name: "Hintergrund" }).getByText("Höhenbild").click();
  await expect.poll(yellow).toBe(0);

  // als GeoTIFF exportieren: erst mit Orthofoto wählbar
  const calc = await section(page, "Berechnungen");
  await calc
    .getByRole("group", { name: "Art der Berechnung" })
    .getByText("Export", { exact: true })
    .click();
  const ortho = calc.getByLabel(/Orthofoto als GeoTIFF/);
  await expect(ortho).toBeEnabled();
  for (const f of ["Höhenmodell", "DXF", "LandXML"]) {
    const box = calc.getByLabel(new RegExp(f));
    if (await box.isChecked()) await box.uncheck();
  }
  await ortho.check();
  await calc.getByRole("button", { name: "Exportieren" }).click();
  const job = page.getByRole("link", { name: /Export · Aushub/ });
  await expect(job.getByText("Fertig")).toBeVisible({ timeout: 30_000 });
  await job.click();
  const link = page
    .getByRole("listitem")
    .filter({ hasText: "orthofoto.tif" })
    .getByRole("link", { name: "Herunterladen" });
  const res = await page.request.get((await link.getAttribute("href"))!);
  // TIFF, little-endian: „II*\0“
  expect([...(await res.body()).subarray(0, 4)]).toEqual([0x49, 0x49, 0x2a, 0]);
  await page.goBack();

  await section(page, "Aufnahmen");
  await row.getByRole("button", { name: "Orthofoto von Aushub entfernen" }).click();
  await expect(row).not.toContainText("Orthofoto");
  // ohne Orthofoto nicht wählbar
  await section(page, "Berechnungen");
  await calc
    .getByRole("group", { name: "Art der Berechnung" })
    .getByText("Export", { exact: true })
    .click();
  await expect(ortho).toBeDisabled();
  await section(page, "Ansicht");
  await expect(view.getByRole("application", { name: /Höhenbild/ })).toBeVisible();
  await expect(view.getByRole("group", { name: "Hintergrund" })).toHaveCount(0);
});
