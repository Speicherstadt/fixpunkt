import AxeBuilder from "@axe-core/playwright";
import { expect, type Page, test } from "@playwright/test";
import { apiFor } from "./api";
import { boundaries, terrainXyz } from "./fixtures";

/**
 * Aufmaß im Browser: Grube 20 × 20 × 2 m = 800 m³ fest. Lehm mit 2 t/m³ und Auflockerung 1,25 → 1.600 t,
 * 1.000 m³ lose, mit 15-t-Kipper 107 Fahrten; 12,50 €/t → 20.000 €. Lieferscheine 900 t + 250 m³ lose +
 * 20 Fahrten = 450 + 200 + 150 = 800 m³ fest → Abweichung nahe 0.
 */
test("Aufmaß mit Preis, Kennwerten und Lieferscheinen", async ({ page }) => {
  const api = await apiFor(page);
  const site = await api.call("POST", "/sites", { name: "Abrechnung" });
  const day = 86_400_000;
  const before = await api.upload(site.id, "u.xyz", "Urgelände", day, terrainXyz(false));
  const after = await api.upload(site.id, "a.xyz", "Januar", 31 * day, terrainXyz(true));
  await api.call("POST", `/sites/${site.id}/regions`, { geojson: boundaries() });
  await api.settled(site.id);
  const regionIds = ((await api.call("GET", `/sites/${site.id}/regions`)) as { id: number }[]).map(
    (r) => r.id,
  );
  await api.call("POST", `/sites/${site.id}/jobs`, {
    kind: "volume",
    params: { before, after, regionIds },
  });
  await api.settled(site.id);
  await page.goto(`/sites/${site.id}`);

  // Preis am Bereich
  const regions = page.getByRole("region", { name: "Bereiche" });
  await regions.getByRole("button", { name: "Material und Massenziel für Grube" }).click();
  const form = regions.getByRole("form", { name: "Massenziel für Grube" });
  await form.getByLabel("Material").fill("Lehm");
  await form.getByLabel("LV-Position").fill("01.02.0010");
  await form.getByLabel("Einheitspreis €").fill("12,50");
  await form.getByLabel("Preis gilt").selectOption({ label: "je t" });
  await form.getByRole("button", { name: "Ziel speichern" }).click();
  await expect(regions.getByRole("listitem").filter({ hasText: "Grube" })).toContainText(
    /Lehm · Pos\. 01\.02\.0010 · 12,50\s€\/t/,
  );

  // Zeitraum: vom Urgelände bis Januar; ohne Kennwerte nur m³
  const billing = page.getByRole("region", { name: "Abrechnung" });
  await expect(billing.getByLabel("Von")).toHaveValue(String(before));
  await expect(billing.getByLabel("Bis")).toHaveValue(String(after));
  const positions = billing.getByRole("table", { name: "Positionen des Aufmaßes" });
  const grube = positions.getByRole("row", { name: /Grube/ });
  await expect(grube).toContainText(/(799|800|801),\d/);
  await expect(billing).toContainText("1 Bereiche ohne Preis");
  await expect(billing).toContainText("1 Bereiche haben einen Preis je t");

  // Kennwerte
  await billing.getByRole("button", { name: "Kennwerte für die Umrechnung" }).click();
  const settings = billing.getByRole("form", { name: "Kennwerte für die Umrechnung" });
  await settings.getByLabel("Nutzlast eines LKW in t").fill("15");
  await settings.getByLabel("Dichte von Lehm").fill("2");
  await settings.getByLabel("Auflockerung von Lehm").fill("1,25");
  await settings.getByRole("button", { name: "Kennwerte speichern" }).click();
  await expect(settings.getByRole("status")).toHaveText("Kennwerte gespeichert.");
  await expect(grube).toContainText(/1\.(59\d|60\d),\d/);
  await expect(grube).toContainText(/(19\.9\d\d|20\.0\d\d),\d\d\s€/);
  await expect(grube.getByRole("cell", { name: /^10[67]$/ })).toBeVisible();

  // Lieferscheine aus CSV
  const csv = [
    "Datum;Material;Menge;Einheit;Lieferschein",
    "10.01.1970;Lehm;900;t;LS-1",
    "20.01.1970;Lehm;250;m³;LS-2",
    "25.01.1970;Lehm;20;Fahrten;LS-3",
  ].join("\n");
  await billing.getByLabel("CSV einlesen").setInputFiles({
    name: "lieferscheine.csv",
    mimeType: "text/csv",
    buffer: Buffer.from(csv),
  });
  await expect(billing.getByText("3 Lieferscheine eingelesen.")).toBeVisible();
  const check = billing
    .getByRole("table", { name: "Abgleich mit Lieferscheinen" })
    .getByRole("row", { name: /Lehm/ });
  await expect(check).toContainText("800,0");
  await expect(check).toContainText(/[+−±][01],\d %/);

  // einzeln erfassen und wieder löschen
  await billing.getByRole("button", { name: "Lieferscheine (3)" }).click();
  const add = billing.getByRole("form", { name: "Lieferschein erfassen" });
  await add.getByLabel("Datum").fill("1970-01-15");
  await add.getByLabel("Material").fill("Lehm");
  await add.getByLabel("Menge").fill("160");
  await add.getByLabel("Lieferschein-Nr.").fill("LS-4");
  await add.getByRole("button", { name: "Lieferschein erfassen" }).click();
  await expect(billing.getByRole("button", { name: "Lieferscheine (4)" })).toBeVisible();
  // 160 t = 80 m³ mehr → +10 %
  await expect(check).toContainText(/\+(9|10),\d %/);
  await billing.getByRole("button", { name: "Lieferschein LS-4 löschen" }).click();
  await expect(billing.getByRole("button", { name: "Lieferscheine (3)" })).toBeVisible();

  // PDF
  const href = await billing.getByRole("link", { name: "Aufmaß als PDF" }).getAttribute("href");
  const pdf = await page.request.get(href!);
  expect(pdf.status()).toBe(200);
  expect(pdf.headers()["content-type"]).toBe("application/pdf");
});

/** Baustelle mit Urgelände und Aushub, Bereichen und fertiger Aushub-Berechnung */
const pitSite = async (page: Page, name: string) => {
  const api = await apiFor(page);
  const site = await api.call("POST", "/sites", { name });
  const day = 86_400_000;
  const before = await api.upload(site.id, "u.xyz", "Urgelände", day, terrainXyz(false));
  const after = await api.upload(site.id, "a.xyz", "Januar", 31 * day, terrainXyz(true));
  await api.call("POST", `/sites/${site.id}/regions`, { geojson: boundaries() });
  await api.settled(site.id);
  const regions = (await api.call("GET", `/sites/${site.id}/regions`)) as {
    id: number;
    name: string;
  }[];
  const volume = async () => {
    await api.call("POST", `/sites/${site.id}/jobs`, {
      kind: "volume",
      params: { before, after, regionIds: regions.map((r) => r.id) },
    });
    await api.settled(site.id);
  };
  await volume();
  return { api, site, regions, volume };
};

/**
 * Schichten: Grube 2 m tief, Oberboden bis 0,5 m → 400 m² × 0,5 m = 200 m³, Lehm darunter 600 m³; je
 * 6 € und 9 € je m³.
 */
test("Aufmaß nach Schichten", async ({ page }) => {
  const { site, volume } = await pitSite(page, "Schichten");
  await page.goto(`/sites/${site.id}`);

  const billing = page.getByRole("region", { name: "Abrechnung" });
  await billing.getByRole("button", { name: "Schichten (0)" }).click();
  const form = billing.getByRole("form", { name: "Schichten (Homogenbereiche)" });
  await form.getByRole("button", { name: "Schicht hinzufügen" }).click();
  await form.getByRole("button", { name: "Schicht hinzufügen" }).click();
  await form.getByLabel("Schicht", { exact: true }).nth(0).fill("Oberboden");
  await form.getByLabel("Schicht", { exact: true }).nth(1).fill("Lehm");
  await form.getByLabel("Unterkante von Oberboden").fill("0,5");
  await expect(form.getByLabel("Unterkante von Lehm")).toBeDisabled();
  await form.getByLabel("Einheitspreis von Oberboden").fill("6");
  await form.getByLabel("Einheitspreis von Lehm").fill("9");
  await form.getByRole("button", { name: "Schichten speichern" }).click();
  await expect(form.getByRole("status")).toContainText("Schichten gespeichert");
  // die Berechnung ist älter als die Schichten
  await expect(billing).toContainText(
    "Schichten sind angelegt, aber die Aushub-Berechnungen sind älter",
  );

  await volume();
  const positions = billing.getByRole("table", { name: "Positionen des Aufmaßes" });
  const top = positions.getByRole("row", { name: /Grube Oberboden/ });
  const rest = positions.getByRole("row", { name: /Grube Lehm/ });
  await expect(top).toContainText(/(19\d|20\d),\d/);
  await expect(rest).toContainText(/(59\d|60\d),\d/);
  await expect(top).toContainText(/6,00\s€\/m³/);
  await expect(billing).not.toContainText("Aushub-Berechnungen sind älter");
});

/** Freigabe: Link erzeugen, als Auftraggeber ohne Anmeldung anerkennen, Stand in der Baustelle */
test("Aufmaß zur Freigabe", async ({ page, browser }) => {
  const { api, site, regions } = await pitSite(page, "Freigabe");
  const grube = regions.find((r) => r.name === "Grube")!;
  await api.call("PATCH", `/regions/${grube.id}`, { unitPrice: 10, priceUnit: "m3" });
  await page.goto(`/sites/${site.id}`);
  const billing = page.getByRole("region", { name: "Abrechnung" });
  await billing.getByRole("button", { name: "Zur Freigabe senden" }).click();
  const dialog = page.getByRole("dialog", { name: "Zur Freigabe senden" });
  await dialog.getByLabel("Notiz für den Auftraggeber (optional)").fill("Abschlag Januar");
  await dialog.getByRole("button", { name: "Link erzeugen" }).click();
  const link = await dialog.getByLabel("Link zur Freigabe").inputValue();
  expect(link).toMatch(/\/freigabe\/[A-Za-z0-9_-]{20,}$/);
  await page.keyboard.press("Escape");

  // Auftraggeber ohne Anmeldung
  const ctx = await browser.newContext({ storageState: undefined });
  const guest = await ctx.newPage();
  await guest.goto(link);
  await expect(guest.getByRole("heading", { level: 1, name: "Aufmaß zur Freigabe" })).toBeVisible();
  await expect(guest.getByText("Abschlag Januar")).toBeVisible();
  await expect(
    guest
      .getByRole("table", { name: "Positionen des Aufmaßes" })
      .getByRole("row", { name: /Grube/ }),
  ).toContainText(/(7\.99\d|8\.0\d\d),\d\d\s€/);
  const axe = await new AxeBuilder({ page: guest })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"])
    .analyze();
  expect(axe.violations.map((v) => v.id)).toEqual([]);
  const decide = guest.getByRole("form", { name: "Freigabe" });
  await expect(decide.getByRole("button", { name: "Einwand erheben" })).toBeDisabled();
  await decide.getByLabel("Ihr Name").fill("M. Muster");
  await decide.getByLabel("Firma (optional)").fill("Bau GmbH");
  await decide.getByRole("button", { name: "Aufmaß anerkennen" }).click();
  await expect(guest.getByRole("status")).toContainText("Anerkannt von M. Muster (Bau GmbH)");
  const pdf = await guest.request.get(`${link.replace("/freigabe/", "/api/public/shares/")}/pdf`);
  expect(pdf.headers()["content-type"]).toBe("application/pdf");
  await ctx.close();

  // in der Baustelle: Stand live, zurückziehen
  await billing.getByRole("button", { name: "Freigaben (1)" }).click();
  const list = billing.getByRole("list", { name: "Freigaben" });
  await expect(list).toContainText("Anerkannt von M. Muster (Bau GmbH)");
  await list.getByRole("button", { name: "Freigabe bis Januar zurückziehen" }).click();
  await expect(list.getByRole("button", { name: /zurückziehen/ })).toHaveCount(0);
});
