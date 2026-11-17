import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { terrainXyz } from "./fixtures";

/**
 * Punktwolke mit Intensität (x y z i, ohne Farbe): in 3D zuerst grau nach Intensität, umschaltbar auf Höhe.
 * Grau heißt R = G = B; die Höhenfarben sind bunt.
 */
test("Punktwolke nach Intensität färben", async ({ page }) => {
  const api = await apiFor(page);
  const site = await api.call("POST", "/sites", { name: "Laserscan" });
  // Intensität aus der Lage: Streifen alle 4 m, damit es viele Grautöne gibt
  const lines = terrainXyz(true)
    .split("\n")
    .slice(1)
    .map((l) => {
      const [x] = l.split(" ").map(Number);
      return `${l} ${Math.round(((x! % 4) / 4) * 255)}`;
    });
  await api.upload(site.id, "scan.xyz", "Scan", 1, lines.join("\n"));
  await api.settled(site.id);
  await page.goto(`/sites/${site.id}`);

  const view = page.getByRole("region", { name: "Ansicht" });
  await view.getByRole("group", { name: "Darstellung" }).getByText("3D", { exact: true }).click();
  await view.getByRole("group", { name: "Darstellung in 3D" }).getByText("Punktwolke").click();
  const terrain = view.getByRole("application", { name: /Gelände in 3D/ });
  await expect(terrain).toHaveAttribute("data-points", "40000");
  const coloring = view.getByRole("group", { name: "Färbung" });
  await expect(coloring.getByRole("radio", { name: "Intensität" })).toBeChecked();

  /** Anteil bunter Pixel unter den gezeichneten (nicht Hintergrund) */
  const colourful = () =>
    terrain.locator("canvas").evaluate((c: HTMLCanvasElement) => {
      const copy = document.createElement("canvas");
      [copy.width, copy.height] = [c.width, c.height];
      const ctx = copy.getContext("2d")!;
      ctx.drawImage(c, 0, 0);
      const px = ctx.getImageData(0, 0, c.width, c.height).data;
      const bg = [px[0]!, px[1]!, px[2]!];
      let [drawn, bunt] = [0, 0];
      for (let i = 0; i < px.length; i += 4 * 13) {
        const [r, g, b] = [px[i]!, px[i + 1]!, px[i + 2]!];
        if (Math.abs(r - bg[0]!) + Math.abs(g - bg[1]!) + Math.abs(b - bg[2]!) < 6) continue;
        drawn++;
        if (Math.max(r, g, b) - Math.min(r, g, b) > 24) bunt++;
      }
      return drawn > 500 ? bunt / drawn : -1;
    });
  await expect.poll(colourful).toBeGreaterThanOrEqual(0);
  expect(await colourful()).toBeLessThan(0.05);
  await coloring.getByText("Höhe").click();
  await expect.poll(colourful).toBeGreaterThan(0.5);
});
