import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { terrainXyz } from "./fixtures";

/** DXF-Plan der Grube: hochladen, Plan/Ist gegen den Aushub, als Hintergrund in der Ansicht. */
test("Plan hochladen und Plan/Ist", async ({ page }) => {
  await page.goto("/");
  const api = await apiFor(page);
  const site = (await api.call("POST", "/sites", { name: "Plan und Ist" })) as { id: number };
  await api.upload(site.id, "aushub.xyz", "Aushub", Date.now(), terrainXyz(true));
  await api.settled(site.id);
  await page.goto(`/sites/${site.id}`);

  const plans = page.getByRole("region", { name: "Pläne" });
  await plans.getByLabel("DXF hochladen").setInputFiles("e2e/data/lageplan.dxf");
  const row = plans.getByRole("listitem").filter({ hasText: "lageplan" });
  await expect(row).toContainText("5 Ebenen · 4 Linien · 2 Höhenpunkte");
  await expect(row).toContainText("Zonenkennung 32 abgezogen");

  // Sohle Mitte genau getroffen (298,20 m), Kote „OK 298,25“ bei Ost +15 m: Gelände 298,15 m → −10 cm
  const table = plans.getByRole("table", { name: "Plan/Ist" });
  await expect(table.getByRole("row")).toHaveCount(3);
  await expect(table.getByRole("row").nth(1)).toContainText("298,20 m");
  await expect(table.getByRole("row").nth(1)).toContainText("0,0 cm");
  await expect(table.getByRole("row").nth(2)).toContainText("298,25 m");
  await expect(table.getByRole("row").nth(2)).toContainText(/[-−]10,0 cm/);

  // als Hintergrund
  const view = page.getByRole("region", { name: "Ansicht" });
  await view.getByLabel("Plan darüber").selectOption({ label: "lageplan" });
  await expect(view.getByRole("application", { name: /Höhenbild/ })).toBeVisible();
  await row.getByRole("button", { name: "lageplan löschen" }).click();
  await expect(row).toHaveCount(0);
});
