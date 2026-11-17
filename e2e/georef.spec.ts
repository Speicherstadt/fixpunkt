import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { terrainXyz, X0, Y0 } from "./fixtures";
import { section } from "./site";

/** Scan im lokalen System: 100 m östlich, 50 m südlich, 2 m zu hoch */
const SHIFT = [100, -50, 2] as const;

const shifted = () =>
  terrainXyz(false)
    .split("\n")
    .map((l, i) => {
      if (i === 0) return l;
      const v = l.split(" ").map(Number);
      return v.map((x, k) => (x + SHIFT[k]!).toFixed(3)).join(" ");
    })
    .join("\n");

/** Passpunkte an drei Ecken, Soll aus dem Gelände (z = 300 + 0,01 · dx), mit Dezimalkomma */
const list = [
  [0.1, 0.1],
  [39.9, 0.1],
  [39.9, 39.9],
  [0.1, 39.9],
]
  .map(([dx, dy], i) => {
    const dst = [X0 + dx!, Y0 + dy!, 300 + 0.01 * dx!];
    const src = dst.map((v, k) => v + SHIFT[k]!);
    return [`P${i + 1}`, ...src, ...dst].map((v) => String(v).replace(".", ",")).join(";");
  })
  .join("\n");

test("Passpunkte: Scan georeferenzieren", async ({ page }) => {
  await page.goto("/");
  const api = await apiFor(page);
  const site = (await api.call("POST", "/sites", { name: "Passpunkte" })) as { id: number };
  await api.upload(site.id, "scan.xyz", "Scan lokal", Date.now(), shifted());
  await api.settled(site.id);
  await page.goto(`/sites/${site.id}`);

  const calc = await section(page, "Berechnungen");
  await calc.getByText("Passpunkte", { exact: true }).click();
  await calc.getByText("Liste einfügen").click();
  await calc.getByLabel(/Eine Zeile je Punkt/).fill(list);
  await calc.getByRole("button", { name: "4 Punkte übernehmen" }).click();
  await expect(calc.getByLabel("Ost (Soll) P3")).toHaveValue("512039,9");

  // Vorschau: Verschiebung zurück, keine Restklaffen
  await expect(
    calc.getByText(/Verschiebung Ost \/ Nord \/ Höhe: [-−]100,000 \/ 50,000 \/ [-−]2,000 m/),
  ).toBeVisible();
  const residuals = calc.getByRole("table", { name: "Restklaffen je Punkt" });
  await expect(residuals.getByRole("row")).toHaveCount(5);
  await expect(calc.getByText("RMS Lage").locator("xpath=following-sibling::dd[1]")).toHaveText(
    "0,0 cm",
  );

  await calc.getByRole("button", { name: "Georeferenzieren" }).click();
  const job = calc.getByRole("link", { name: /Passpunkte · Scan lokal/ });
  await expect(job.getByText("Fertig")).toBeVisible({ timeout: 30_000 });
  await expect(job).toContainText("RMS 0,0 cm");
  await section(page, "Aufnahmen");
  const row = page.getByRole("table").getByRole("row", { name: /Scan lokal \(georeferenziert\)/ });
  await expect(row.getByText("Bereit")).toBeVisible({ timeout: 30_000 });
});
