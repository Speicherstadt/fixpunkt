import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { terrainXyz } from "./fixtures";

/** Leitung in der Ansicht nachzeichnen, in der Dokumentation wiederfinden, als DXF für CAD herunterladen */
test("Leitung zeichnen und als DXF exportieren", async ({ page }) => {
  const api = await apiFor(page);
  const site = await api.call("POST", "/sites", { name: "Leitungsbau" });
  await api.upload(site.id, "a.xyz", "Aushub", 1, terrainXyz(true));
  await api.settled(site.id);
  await page.goto(`/sites/${site.id}`);

  const view = page.getByRole("region", { name: "Ansicht" });
  const canvas = view.getByRole("application", { name: /Höhenbild der Baustelle/ });
  await expect(canvas).toBeVisible();
  const box = (await canvas.boundingBox())!;
  await view.getByRole("group", { name: "Werkzeug" }).getByText("Leitung zeichnen").click();
  await expect(view.getByText(/mindestens zwei/)).toBeVisible();
  for (const [fx, fy] of [
    [0.3, 0.3],
    [0.5, 0.5],
    [0.7, 0.5],
  ] as const)
    await canvas.click({ position: { x: box.width * fx, y: box.height * fy } });
  // dieselbe Länge wie gemessen (horizontal)
  const measured = await view
    .getByRole("term")
    .filter({ hasText: "Länge (horizontal)" })
    .locator("xpath=following-sibling::dd[1]")
    .textContent();
  await view.getByLabel("Name der Leitung").fill("Kanal Bestand");
  await view.getByLabel("Notiz (Material, DN …)").fill("Steinzeug DN 300");
  await view.getByRole("button", { name: "Leitung speichern" }).click();

  const doc = page.getByRole("region", { name: "Dokumentation" });
  const item = doc.getByRole("listitem").filter({ hasText: "Kanal Bestand" });
  await expect(item).toContainText("Leitung");
  await expect(item).toContainText(`Länge ${measured} · 3 Punkte`);
  await expect(item).toContainText("Steinzeug DN 300");

  const [download] = await Promise.all([
    page.waitForEvent("download"),
    doc.getByRole("link", { name: "DXF für CAD" }).click(),
  ]);
  expect(download.suggestedFilename()).toMatch(/^Dokumentation-Leitungsbau-\d+\.dxf$/);
  const dxf = readFileSync(await download.path(), "utf8");
  expect(dxf).toContain("FP_LEITUNGEN");
  expect(dxf).toContain("Kanal Bestand");
  expect(dxf.match(/\nVERTEX\n/g)).toHaveLength(3);
});
