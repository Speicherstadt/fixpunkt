import AxeBuilder from "@axe-core/playwright";
import { expect, type Page, test } from "@playwright/test";

/** WCAG 2.2 AA mit axe auf jeder Seite, hell und dunkel – ein Verstoß macht den Test rot (wie Kontext). */
const PAGES = ["/", "/users", "/account"];

const check = async (page: Page, path: string) => {
  await page.goto(path);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  const result = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"]).analyze();
  expect(result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(", ")}`)).toEqual([]);
};

for (const scheme of ["light", "dark"] as const) {
  test.describe(`Barrierefreiheit ${scheme}`, () => {
    test.use({ colorScheme: scheme });
    for (const path of PAGES) test(`${path} ohne Verstöße`, async ({ page }) => check(page, path));
  });
}

test("Anmeldeseite ohne Verstöße", async ({ browser }) => {
  const ctx = await browser.newContext({ storageState: undefined });
  const page = await ctx.newPage();
  await check(page, "/login");
  await ctx.close();
});

test("Am Handy erreichbar @phone", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("navigation", { name: "Hauptnavigation" })).toBeVisible();
  const width = await page.evaluate(() => document.documentElement.scrollWidth);
  expect(width).toBeLessThanOrEqual(page.viewportSize()!.width);
});

test("Tastatur: erster Tab-Stopp springt zum Inhalt", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await page.keyboard.press("Tab");
  const skip = page.getByRole("link", { name: "Zum Inhalt springen" });
  await expect(skip).toBeFocused();
});
