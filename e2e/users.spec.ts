import { expect, test } from "@playwright/test";

test("Admin legt einen Leser an, der nur lesen darf", async ({ page, browser }) => {
  await page.goto("/users");
  await page.getByLabel("Name").fill("leser");
  await page.getByLabel("Erstes Passwort").fill("nurlesen1");
  await page.getByLabel("Rolle").selectOption("viewer");
  await page.getByRole("button", { name: "Benutzer anlegen" }).click();
  await expect(page.getByRole("listitem").filter({ hasText: "leser" })).toContainText("Nur lesen");

  const ctx = await browser.newContext({ storageState: undefined });
  const viewer = await ctx.newPage();
  await viewer.goto("/login");
  await viewer.getByLabel("Name").fill("leser");
  await viewer.getByLabel("Passwort").fill("falsch123");
  await viewer.getByRole("button", { name: "Anmelden", exact: true }).click();
  await expect(viewer.getByRole("alert")).toHaveText("Name oder Passwort stimmt nicht.");
  await viewer.getByLabel("Passwort").fill("nurlesen1");
  await viewer.getByRole("button", { name: "Anmelden", exact: true }).click();
  await expect(viewer.getByRole("heading", { name: "Baustellen", level: 1 })).toBeVisible();
  await expect(viewer.getByRole("button", { name: "Neue Baustelle" })).toHaveCount(0);
  await expect(viewer.getByRole("link", { name: "Benutzer" })).toHaveCount(0);
  await ctx.close();
});

test("Abmelden beendet die Sitzung", async ({ browser }) => {
  // eigene Sitzung – die gemeinsame des Admins bleibt für die anderen Tests gültig
  const ctx = await browser.newContext({ storageState: undefined });
  const page = await ctx.newPage();
  await page.goto("/login");
  await page.getByLabel("Name").fill("admin");
  await page.getByLabel("Passwort").fill("geheim123");
  await page.getByRole("button", { name: "Anmelden", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Baustellen", level: 1 })).toBeVisible();
  await page.getByRole("button", { name: "Abmelden" }).click();
  await expect(page).toHaveURL(/\/login$/);
  await page.goto("/");
  await expect(page).toHaveURL(/\/login$/);
  await ctx.close();
});
