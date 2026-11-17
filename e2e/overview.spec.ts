import { readFileSync } from "node:fs";
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

/** Überblick anpassen: Kennzahlen ausblenden, Aushub nach oben, bleibt nach dem Neuladen, Zurücksetzen */
test("Überblick anpassen", async ({ page }) => {
  await page.goto("/");
  const api = await apiFor(page);
  const site = (await api.call("POST", "/sites", { name: "Anpassen" })) as { id: number };
  const before = await api.upload(site.id, "u.xyz", "Urgelände", 1, terrainXyz(false));
  const after = await api.upload(site.id, "a.xyz", "Aushub", 2, terrainXyz(true));
  await api.settled(site.id);
  await api.call("POST", `/sites/${site.id}/jobs`, { kind: "volume", params: { before, after } });
  await api.settled(site.id);
  await page.goto(`/sites/${site.id}`);

  const overview = page.getByRole("region", { name: "Überblick" });
  await expect(overview.getByText("Aufnahmen", { exact: true })).toBeVisible();
  await overview.getByRole("button", { name: "Anpassen" }).click();
  const list = overview.getByRole("list", { name: "Anpassen" });
  // klicken und auf den Zustand warten (uncheck() prüft sofort, die Seite zeichnet daneben noch Höhenbild und 3D)
  const kpis = list.getByLabel("Kennzahlen", { exact: true });
  await kpis.click();
  await expect(kpis).not.toBeChecked();
  await list
    .getByRole("button", { name: "Aushub je Befliegung (gegen das Urgelände) nach oben" })
    .click();
  await expect(list.getByRole("listitem").first()).toContainText("Aushub je Befliegung");
  await overview.getByRole("button", { name: "Fertig" }).click();
  await expect(overview.getByText("Aufnahmen", { exact: true })).toHaveCount(0);

  // gespeichert: nach dem Neuladen gleich
  await page.reload();
  await expect(overview.getByRole("heading", { level: 3 }).first()).toHaveText(
    "Aushub je Befliegung (gegen das Urgelände)",
  );
  await expect(overview.getByText("Aufnahmen", { exact: true })).toHaveCount(0);

  await overview.getByRole("button", { name: "Anpassen" }).click();
  await overview.getByRole("button", { name: "Zurücksetzen" }).click();
  await expect(overview.getByText("Aufnahmen", { exact: true })).toBeVisible();
});

/** Eigener Bericht: Titel und Firma, nur Kennzahlen und Aushub, als PDF */
test("Bericht zusammenstellen", async ({ page }) => {
  await page.goto("/");
  const api = await apiFor(page);
  const site = (await api.call("POST", "/sites", { name: "Bericht" })) as { id: number };
  const before = await api.upload(site.id, "u.xyz", "Urgelände", 1, terrainXyz(false));
  const after = await api.upload(site.id, "a.xyz", "Aushub", 2, terrainXyz(true));
  await api.settled(site.id);
  await api.call("POST", `/sites/${site.id}/jobs`, { kind: "volume", params: { before, after } });
  await api.settled(site.id);
  await page.goto(`/sites/${site.id}`);

  await page
    .getByRole("region", { name: "Überblick" })
    .getByRole("button", { name: "Bericht" })
    .click();
  const dialog = page.getByRole("dialog", { name: "Bericht zusammenstellen" });
  await dialog.getByLabel("Titel").fill("Wochenbericht KW 47");
  await dialog.getByLabel("Erstellt von (Firma)").fill("Muster Tiefbau");
  const parts = dialog.getByRole("list", { name: "Bausteine" });
  for (const box of await parts.getByRole("checkbox").all()) {
    await box.click();
    await expect(box).not.toBeChecked();
  }
  await expect(dialog.getByText("Mindestens einen Baustein wählen.")).toBeVisible();
  for (const name of ["Kennzahlen", "Aushub je Befliegung (gegen das Urgelände)"]) {
    const box = parts.getByLabel(name, { exact: true });
    await box.click();
    await expect(box).toBeChecked();
  }

  const link = dialog.getByRole("link", { name: "PDF herunterladen" });
  await expect(link).toHaveAttribute(
    "href",
    `/api/sites/${site.id}/report?parts=kpis%2Cprogress&title=Wochenbericht+KW+47&company=Muster+Tiefbau`,
  );
  const [download] = await Promise.all([page.waitForEvent("download"), link.click()]);
  expect(download.suggestedFilename()).toBe(`Bericht-Bericht-${site.id}.pdf`);
  const pdf = readFileSync(await download.path());
  expect(pdf.subarray(0, 5).toString()).toBe("%PDF-");
  expect(pdf.includes("Wochenbericht KW 47")).toBe(true);
  expect(pdf.includes("Muster Tiefbau")).toBe(true);

  // die Auswahl merkt sich dieser Browser
  await page.reload();
  await page
    .getByRole("region", { name: "Überblick" })
    .getByRole("button", { name: "Bericht" })
    .click();
  await expect(page.getByRole("dialog").getByLabel("Titel")).toHaveValue("Wochenbericht KW 47");
});
