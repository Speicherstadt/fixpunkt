import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { PIT_M3, terrainXyz, X0, Y0 } from "./fixtures";

/**
 * Überblick einer Baustelle: Aushub gegen das Urgelände (Grube 800 m³ aus den Testdaten), Zwangspunkte gegen die
 * letzte Aufnahme (Sohle bei x = 20 m auf 300 + 0,01·20 − 2 = 298,2 m: ein Punkt dort hält, einer 50 cm tiefer nicht).
 */
test("Überblick zeigt Aushub und verletzte Zwangspunkte", async ({ page }) => {
  await page.goto("/");
  const api = await apiFor(page);
  const site = (await api.call("POST", "/sites", { name: "Überblick" })) as { id: number };
  const day = 86_400_000;
  const start = Date.UTC(2026, 10, 3);
  const before = await api.upload(site.id, "u.xyz", "Urgelände", start, terrainXyz(false));
  const after = await api.upload(site.id, "a.xyz", "Aushub", start + 7 * day, terrainXyz(true));
  await api.settled(site.id);
  await api.call("POST", `/sites/${site.id}/jobs`, { kind: "volume", params: { before, after } });
  for (const [title, z] of [
    ["Sohle Mitte", 298.2],
    ["Kanal", 297.7],
  ] as const) {
    await api.call("POST", `/sites/${site.id}/markers`, {
      kind: "constraint",
      title,
      x: X0 + 20,
      y: Y0 + 20,
      z,
    });
  }
  await api.settled(site.id);
  await page.goto(`/sites/${site.id}`);

  const overview = page.getByRole("region", { name: "Überblick" });
  await expect(overview).toContainText("Aufnahmen2");
  await expect(overview.getByRole("link", { name: /Aushub/ })).toBeVisible();
  const m3 = new Intl.NumberFormat("de-DE", { maximumFractionDigits: 0 }).format(PIT_M3);
  await expect(overview).toContainText(new RegExp(`${m3.replace(".", "\\.")}(,\\d)? m³`));
  await expect(overview).toContainText("Zwangspunkte eingehalten1 / 2");
  await expect(
    overview.getByRole("list", { name: "Braucht Aufmerksamkeit" }).getByRole("listitem"),
  ).toHaveText([/Zwangspunkt „Kanal“: \+?50,0 cm/]);
});
