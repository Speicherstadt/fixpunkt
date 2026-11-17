import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { boundaries, terrainXyz } from "./fixtures";

/** 2D-Ansicht: messen mit Maus und Tastatur, Bereich zeichnen und speichern, Differenz darüberlegen */
test("Ansicht: messen, zeichnen, überlagern", async ({ page }) => {
  test.setTimeout(90_000);
  const api = await apiFor(page);
  const site = await api.call("POST", "/sites", { name: "Ansicht" });
  const before = await api.upload(site.id, "u.xyz", "Urgelände", 1, terrainXyz(false));
  const after = await api.upload(site.id, "a.xyz", "Aushub", 2, terrainXyz(true));
  await api.call("POST", `/sites/${site.id}/regions`, { geojson: boundaries() });
  await api.settled(site.id);
  await api.call("POST", `/sites/${site.id}/jobs`, { kind: "volume", params: { before, after } });
  await api.settled(site.id);

  await page.goto(`/sites/${site.id}`);
  const view = page.getByRole("region", { name: "Ansicht" });
  const canvas = view.getByRole("application", { name: /Höhenbild der Baustelle/ });
  await expect(canvas).toBeVisible();
  const box = (await canvas.boundingBox())!;
  const at = (fx: number, fy: number) => ({ x: box.width * fx, y: box.height * fy });
  const tools = view.getByRole("group", { name: "Werkzeug" });

  // Strecke quer über das Bild: Länge > 10 m, 3D-Länge ≥ 2D
  await tools.getByText("Strecke", { exact: true }).click();
  await canvas.click({ position: at(0.3, 0.5) });
  await canvas.click({ position: at(0.7, 0.5) });
  const results = view.getByRole("definition");
  await expect(view.getByRole("term").filter({ hasText: "Länge (horizontal)" })).toBeVisible();
  await expect(results.first()).toHaveText(/^\d{2},\d{2} m$/);

  // Punkt per Tastatur am Fadenkreuz in der Mitte: Höhe der Grube (≈ 298,2 m)
  await tools.getByText("Punkt", { exact: true }).click();
  await canvas.focus();
  await page.keyboard.press("Enter");
  const height = view
    .getByRole("term")
    .filter({ hasText: /^Höhe$/ })
    .locator("xpath=following-sibling::dd[1]");
  await expect(height).toHaveText(/^298,[12]\d m$/);

  // Bereich zeichnen und speichern
  await tools.getByText("Bereich zeichnen", { exact: true }).click();
  for (const [fx, fy] of [
    [0.4, 0.4],
    [0.6, 0.4],
    [0.6, 0.6],
  ] as const)
    await canvas.click({ position: at(fx, fy) });
  await expect(view.getByRole("term").filter({ hasText: "Fläche (horizontal)" })).toBeVisible();
  await page.getByLabel("Name des Bereichs").fill("Gezeichnet");
  await page.getByRole("button", { name: "Bereich speichern" }).click();
  await expect(page.getByRole("listitem").filter({ hasText: "Gezeichnet" })).toBeVisible();

  // Differenz darüberlegen
  await page.getByLabel("Darüber anzeigen").selectOption({ label: "Urgelände → Aushub" });
  await expect(page.getByText(/Material weg/)).toBeVisible();

  // 3D: Geländenetz mit Differenzfarben, mit der Tastatur bewegbar
  await view.getByRole("group", { name: "Darstellung" }).getByText("3D", { exact: true }).click();
  const terrain = view.getByRole("application", { name: /Gelände in 3D/ });
  await expect(terrain).toBeVisible();
  // 40 × 40 m bei 0,25 m: 160 × 160 Stützpunkte → 159 × 159 × 2 Dreiecke
  await expect(terrain).toHaveAttribute("data-triangles", String(159 * 159 * 2));
  const colours = () =>
    terrain.locator("canvas").evaluate((c: HTMLCanvasElement) => {
      const copy = document.createElement("canvas");
      copy.width = c.width;
      copy.height = c.height;
      const ctx = copy.getContext("2d")!;
      ctx.drawImage(c, 0, 0);
      const px = ctx.getImageData(0, 0, c.width, c.height).data;
      const seen = new Set<number>();
      for (let i = 0; i < px.length; i += 4 * 97)
        seen.add((px[i]! << 16) | (px[i + 1]! << 8) | px[i + 2]!);
      return seen.size;
    });
  // gezeichnet: viele Farbtöne (Licht und Farbskala), nicht nur der Hintergrund
  await expect.poll(colours).toBeGreaterThan(50);
  const still = await terrain.locator("canvas").screenshot();
  await terrain.focus();
  await page.keyboard.press("ArrowLeft");
  await expect
    .poll(async () => (await terrain.locator("canvas").screenshot()).equals(still))
    .toBe(false);
  await view.getByLabel("Überhöhung").selectOption("5");
  await expect(terrain).toBeVisible();
});
