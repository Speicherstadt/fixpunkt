import { spawnSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { expect, type Page, test } from "@playwright/test";
import { type Api, apiFor } from "./api";
import { boundaries, terrainXyz, X0, Y0 } from "./fixtures";

/**
 * Die Bilder in README und Doku (docs/screenshot-*.png, hell und dunkel) und das Vorschaubild
 * (docs/social-preview.png). Läuft nur mit DOKU_SCREENSHOTS=1 (bun run docs:screenshots, Workflow
 * „Update screenshots“). Legt über die API eine Baustelle mit drei Aufnahmen an, wie sie über zwei
 * Wochen entstehen. Vorbild: firsttris/haben apps/web/e2e/zz-doku-screenshots.spec.ts
 */
test.describe.configure({ mode: "serial" });
test.skip(!process.env.DOKU_SCREENSHOTS, "nur für bun run docs:screenshots");

const DOCS = "docs";
const DESKTOP = { width: 1440, height: 900 };
const DAY = 86_400_000;
const START = Date.UTC(2026, 10, 3, 9);

let page: Page;
let api: Api["call"];
let upload: Api["upload"];
let waitForJobs: Api["settled"];

/** Zwischenstand: Grube erst zur Hälfte ausgehoben */
const halfway = () =>
  terrainXyz(false)
    .split("\n")
    .map((line, i) => {
      if (i === 0) return line;
      const [x, y, z] = line.split(" ").map(Number) as [number, number, number];
      const dx = x - X0;
      const dy = y - Y0;
      const deep = dx >= 10 && dx < 20 && dy >= 10 && dy < 30;
      return `${x.toFixed(3)} ${y.toFixed(3)} ${(z - (deep ? 2 : 0)).toFixed(3)}`;
    })
    .join("\n");

let reportJob = 0;
let compareJob = 0;
let siteId = 0;

test.beforeAll(async ({ browser }) => {
  mkdirSync(DOCS, { recursive: true });
  page = await browser.newPage({
    viewport: DESKTOP,
    locale: "de-DE",
    timezoneId: "Europe/Berlin",
    storageState: "e2e/.auth/admin.json",
    // ohne Farbübergänge: sonst fotografiert der Wechsel hell → dunkel halbe Farben (styles.css)
    reducedMotion: "reduce",
  });
  await page.goto("/");
  ({ call: api, upload, settled: waitForJobs } = await apiFor(page));

  const site = await api("POST", "/sites", {
    name: "Baugrube Kirchstraße 12",
    description: "Neubau MFH mit Tiefgarage · Aushub bis −2,0 m",
  });
  siteId = site.id;
  const before = await upload(siteId, "flug-2026-11-03.xyz", "Urgelände", START, terrainXyz(false));
  await upload(siteId, "flug-2026-11-10.xyz", "Zwischenstand", START + 7 * DAY, halfway());
  const after = await upload(
    siteId,
    "flug-2026-11-17.xyz",
    "Aushub fertig",
    START + 14 * DAY,
    terrainXyz(true),
  );
  await api("POST", `/sites/${siteId}/regions`, { geojson: boundaries() });
  const regions = (await api("GET", `/sites/${siteId}/regions`)) as { id: number }[];
  await waitForJobs(siteId);
  const job = await api("POST", `/sites/${siteId}/jobs`, {
    kind: "volume",
    params: {
      before,
      after,
      regionIds: regions.map((r) => r.id),
      surface: { cell: 0.25, classifyGround: true },
    },
  });
  reportJob = job.id;
  // Soll/Ist prüft man auf der Sohle, nicht über den Grubenrand
  const [sohle] = await api("POST", `/sites/${siteId}/regions`, {
    name: "Sohle",
    geojson: {
      type: "Polygon",
      crs: { type: "name", properties: { name: "urn:ogc:def:crs:EPSG::25832" } },
      coordinates: [
        [
          [X0 + 12, Y0 + 12],
          [X0 + 28, Y0 + 12],
          [X0 + 28, Y0 + 28],
          [X0 + 12, Y0 + 28],
          [X0 + 12, Y0 + 12],
        ],
      ],
    },
  });
  compareJob = (
    await api("POST", `/sites/${siteId}/jobs`, {
      kind: "compare",
      params: {
        captureId: after,
        regionId: sohle.id,
        tolerance: 0.05,
        design: { type: "height", height: 298.2 },
      },
    })
  ).id;
  // weitere Baustellen für die Übersicht
  const others = [
    ["Kanal Bahnhofstraße", "Leitungsgraben DN 300, 140 m", 1],
    ["Haufwerk Bauhof Nord", "Oberboden-Mieten, monatliche Bestandsaufnahme", 2],
    ["Regenrückhaltebecken Am Wiesengrund", "Erdbecken 1.800 m³", 1],
  ] as const;
  for (const [name, description, flights] of others) {
    const other = await api("POST", "/sites", { name, description });
    for (let i = 0; i < flights; i++) {
      const day = START - (20 - 7 * i) * DAY;
      await upload(other.id, `flug-${i + 1}.xyz`, `Befliegung ${i + 1}`, day, terrainXyz(i > 0));
    }
  }
  await waitForJobs(siteId);
  const all = (await api("GET", "/sites")) as { id: number }[];
  for (const s of all) await waitForJobs(s.id);
});

test.afterAll(async () => {
  await page.close();
});

/**
 * Farbschema wie beim Nutzer: Browser-Einstellung, Seite neu laden (public/theme.js setzt die Klasse vor dem
 * ersten Zeichnen). Nur die Klasse umzuschalten ließ Symbole in Links ungezeichnet.
 */
const setTheme = async (mode: "light" | "dark") => {
  await page.emulateMedia({ colorScheme: mode });
  await page.reload();
  await page.waitForLoadState("networkidle");
  await expect(page.getByRole("heading", { level: 1 }).first()).toBeVisible();
};

const go = async (path: string) => {
  await page.goto(path);
  await page.waitForLoadState("networkidle");
  await expect(page.getByRole("heading", { level: 1 }).first()).toBeVisible();
};

/** Jedes Bild hell und dunkel; README und Doku zeigen das passende zum Farbschema */
const shot = async (name: string, fullPage = false) => {
  for (const mode of ["light", "dark"] as const) {
    await setTheme(mode);
    await page.screenshot({
      path: `${DOCS}/screenshot-${name}${mode === "dark" ? "-dunkel" : ""}.png`,
      fullPage,
    });
  }
  await setTheme("light");
};

test("Baustellen @screenshots", async () => {
  await go("/");
  await expect(page.getByText("Baugrube Kirchstraße 12")).toBeVisible();
  await shot("baustellen");
});

test("Baustelle @screenshots", async () => {
  await go(`/sites/${siteId}`);
  await expect(
    page
      .getByRole("link", { name: /Urgelände → Aushub fertig/ })
      .getByText("Fertig", { exact: true }),
  ).toBeVisible();
  await shot("baustelle", true);
});

test("Ansicht mit Differenz @screenshots", async () => {
  await go(`/sites/${siteId}`);
  const view = page.getByRole("region", { name: "Ansicht" });
  await view.getByLabel("Darüber anzeigen").selectOption({ index: 1 });
  await view.getByRole("group", { name: "Werkzeug" }).getByText("Profil", { exact: true }).click();
  const canvas = view.getByRole("application");
  const box = (await canvas.boundingBox())!;
  await canvas.click({ position: { x: box.width * 0.3, y: box.height * 0.55 } });
  await canvas.click({ position: { x: box.width * 0.7, y: box.height * 0.55 } });
  await expect(view.getByRole("img", { name: /Profil über/ })).toBeVisible();
  await view.scrollIntoViewIfNeeded();
  await view.screenshot({ path: `${DOCS}/screenshot-ansicht.png` });
});

test("Mengenbericht und Vorschaubild @screenshots", async () => {
  await go(`/jobs/${reportJob}`);
  await expect(page.getByRole("table", { name: "Massen je Bereich" })).toBeVisible();
  await shot("bericht");
  await page.setViewportSize({ width: 1280, height: 640 });
  await setTheme("light");
  await page.screenshot({ path: `${DOCS}/social-preview.png` });
  await page.setViewportSize(DESKTOP);
});

/**
 * PDF-Bericht als Beispiel zum Herunterladen und Seite 1 als Vorschaubild (pdftoppm aus poppler-utils;
 * fehlt es, bleibt das alte Bild)
 */
test("PDF-Bericht @screenshots", async () => {
  await go(`/jobs/${reportJob}`);
  const pdf = page.waitForEvent("download");
  await page.getByRole("link", { name: "PDF-Bericht" }).click();
  const file = `${DOCS}/beispiel-mengenbericht.pdf`;
  await (await pdf).saveAs(file);
  const render = spawnSync("pdftoppm", [
    "-png",
    "-singlefile",
    "-r",
    "110",
    file,
    `${DOCS}/screenshot-pdf`,
  ]);
  if (render.error) console.warn("pdftoppm fehlt, Vorschaubild nicht erneuert");
});

test("Soll/Ist @screenshots", async () => {
  await go(`/jobs/${compareJob}`);
  await expect(page.getByRole("list", { name: "Verteilung der Abweichung" })).toBeVisible();
  await shot("soll-ist", true);
});

test("Anmeldung @screenshots", async ({ browser }) => {
  const ctx = await browser.newContext({ viewport: { width: 1024, height: 700 }, locale: "de-DE" });
  const p = await ctx.newPage();
  await p.goto("/login");
  await expect(p.getByRole("heading", { level: 1 })).toBeVisible();
  await p.screenshot({ path: `${DOCS}/screenshot-anmeldung.png` });
  await ctx.close();
});

test("Am Handy @screenshots", async ({ browser }) => {
  const ctx = await browser.newContext({
    viewport: { width: 390, height: 844 },
    deviceScaleFactor: 2,
    isMobile: true,
    hasTouch: true,
    locale: "de-DE",
    storageState: "e2e/.auth/admin.json",
    reducedMotion: "reduce",
  });
  const p = await ctx.newPage();
  await p.goto(`/jobs/${reportJob}`);
  await expect(p.getByRole("heading", { name: "Mengenbericht" })).toBeVisible();
  await p.screenshot({ path: `${DOCS}/screenshot-handy.png` });
  await ctx.close();
});
