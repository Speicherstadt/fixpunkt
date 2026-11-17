import { writeFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import { apiFor } from "./api";
import { terrainXyz } from "./fixtures";
import { section } from "./site";

/** Upload reißt ab und setzt sich fort (tus): erster PATCH bricht ab, danach HEAD und Rest. */
test("Upload setzt nach Abbruch fort", async ({ page }, info) => {
  await page.goto("/");
  const api = await apiFor(page);
  const site = (await api.call("POST", "/sites", { name: "Wackliges Netz" })) as { id: number };
  const file = info.outputPath("gelaende.xyz");
  writeFileSync(file, terrainXyz(true));

  const calls: string[] = [];
  let failed = false;
  await page.route("**/api/uploads/*", async (route) => {
    const m = route.request().method();
    calls.push(m);
    if (m === "PATCH" && !failed) {
      failed = true;
      await route.abort("connectionreset");
    } else await route.continue();
  });

  await page.goto(`/sites/${site.id}`);
  await section(page, "Aufnahmen");
  await page.getByLabel("Aufnahme hochladen").setInputFiles(file);
  const row = page.getByRole("table").getByRole("row", { name: /gelaende/ });
  await expect(row.getByText("Bereit")).toBeVisible({ timeout: 30_000 });
  await expect(row).toContainText("40.000");
  // nach dem Abbruch: nachfragen (HEAD), dann weiter (PATCH)
  expect(calls[0]).toBe("PATCH");
  expect(calls.slice(1)).toContain("HEAD");
  expect(calls.filter((c) => c === "PATCH").length).toBeGreaterThanOrEqual(2);
});
