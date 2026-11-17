import type { Page } from "@playwright/test";

/** In welchem Reiter der Baustelle ein Abschnitt steht */
const TAB_OF: Record<string, string> = {
  Überblick: "Überblick",
  Aufnahmen: "Aufnahmen",
  "Drohnenfotos auswerten": "Aufnahmen",
  Flugplanung: "Aufnahmen",
  Ansicht: "Ansicht",
  Bereiche: "Massen",
  Berechnungen: "Massen",
  Abrechnung: "Abrechnung",
  Pläne: "Dokumentation",
  Dokumentation: "Dokumentation",
};

/** zugeklappte Abschnitte, die sich über ihre Überschrift öffnen */
const COLLAPSED = new Set(["Drohnenfotos auswerten", "Flugplanung"]);

/** Abschnitt einer Baustelle: passenden Reiter wählen, bei Bedarf aufklappen */
export const section = async (page: Page, name: string) => {
  const tab = TAB_OF[name];
  if (!tab) throw new Error(`kein Reiter für ${name}`);
  await page
    .getByRole("navigation", { name: "Bereiche der Baustelle" })
    .getByRole("link", { name: tab, exact: true })
    .click();
  const region = page.getByRole("region", { name });
  if (COLLAPSED.has(name)) {
    const toggle = region.getByRole("button", { name });
    if ((await toggle.getAttribute("aria-expanded")) === "false") await toggle.click();
  }
  return region;
};
