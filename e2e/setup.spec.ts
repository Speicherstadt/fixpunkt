import { expect, test } from "@playwright/test";

/** Erster Start: Fixpunkt einrichten; die Sitzung nutzen danach alle anderen Tests. */
test("Fixpunkt einrichten", async ({ page }) => {
  await page.goto("/");
  await expect(page).toHaveURL(/\/login$/);
  await expect(page.getByRole("heading", { name: "Willkommen bei Fixpunkt" })).toBeVisible();
  await page.getByLabel("Name").fill("admin");
  await page.getByLabel("Passwort").fill("kurz");
  // die Prüfung im Browser verhindert zu kurze Passwörter
  await page.getByRole("button", { name: "Fixpunkt einrichten" }).click();
  await expect(page).toHaveURL(/\/login$/);
  await page.getByLabel("Passwort").fill("geheim123");
  await page.getByRole("button", { name: "Fixpunkt einrichten" }).click();
  await expect(page.getByRole("heading", { name: "Baustellen", level: 1 })).toBeVisible();
  await page.context().storageState({ path: "e2e/.auth/admin.json" });
});
