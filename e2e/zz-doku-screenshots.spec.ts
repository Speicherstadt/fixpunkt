import { spawnSync } from "node:child_process";
import { mkdirSync, readFileSync } from "node:fs";
import { expect, type Page, test } from "@playwright/test";
import { type Api, apiFor } from "./api";
import { boundaries, coloredTerrainXyz, terrainXyz, X0, Y0 } from "./fixtures";

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
  const middle = await upload(
    siteId,
    "flug-2026-11-10.xyz",
    "Zwischenstand",
    START + 7 * DAY,
    halfway(),
  );
  const after = await upload(
    siteId,
    "flug-2026-11-17.xyz",
    "Aushub fertig",
    START + 14 * DAY,
    coloredTerrainXyz(true),
  );
  await api("POST", `/sites/${siteId}/regions`, { geojson: boundaries() });
  const regions = (await api("GET", `/sites/${siteId}/regions`)) as { id: number; name: string }[];
  // Massenziel für den Überblick: 1.000 m³ Boden Klasse 4 aus der Grube
  const grube = regions.find((r) => r.name === "Grube")!;
  await api("PATCH", `/regions/${grube.id}`, {
    material: "Boden Klasse 4",
    targetM3: 1000,
    targetKind: "cut",
  });
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
  // Zwischenstand für den Fortschritt im Überblick
  await api("POST", `/sites/${siteId}/jobs`, {
    kind: "volume",
    params: { before, after: middle, regionIds: regions.map((r) => r.id), surface: { cell: 0.25 } },
  });
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
  // Dokumentation: Zwangspunkt am Kanal, Notiz, Foto mit GPS (Stelle E 512 025, N 5 403 015)
  await api("POST", `/sites/${siteId}/markers`, {
    kind: "constraint",
    title: "Kanal Bestand",
    note: "DN 300, Sohle nicht unterschreiten",
    x: X0 + 32,
    y: Y0 + 20,
    z: 300.3,
  });
  // Leitung am Grubenrand entlang, Höhen vom Urgelände
  await api("POST", `/sites/${siteId}/markers`, {
    kind: "line",
    title: "Kanal DN 300",
    note: "Steinzeug, Bestand",
    points: [
      [X0 + 32, Y0 + 2, 300.0],
      [X0 + 32, Y0 + 20, 300.0],
      [X0 + 32, Y0 + 38, 300.0],
    ],
  });
  await api("POST", `/sites/${siteId}/markers`, {
    kind: "note",
    title: "Bodenprobe BP 2",
    note: "Labor, Ergebnis Z0",
    x: X0 + 15,
    y: Y0 + 34,
  });
  await page.request.put(`/api/sites/${siteId}/photos?file=Schacht.jpg`, {
    data: readFileSync("e2e/data/foto-gps.jpg"),
    headers: {
      "content-type": "application/octet-stream",
      "x-csrf-token": (await (await page.request.get("/api/auth/state")).json()).csrf,
    },
  });
  // Orthofoto zum Stand „Aushub fertig“
  await page.request.put(`/api/captures/${after}/ortho?file=orthophoto.tif`, {
    data: readFileSync("e2e/data/ortho.tif"),
    headers: {
      "content-type": "application/octet-stream",
      "x-csrf-token": (await (await page.request.get("/api/auth/state")).json()).csrf,
    },
  });
  // Lageplan der Grube (DXF mit Zonenkennung, Höhenpunkte)
  await page.request.put(`/api/sites/${siteId}/plans?file=Lageplan.dxf`, {
    data: readFileSync("e2e/data/lageplan.dxf"),
    headers: {
      "content-type": "application/octet-stream",
      "x-csrf-token": (await (await page.request.get("/api/auth/state")).json()).csrf,
    },
  });
  // weitere Baustellen für die Übersicht
  // je eine eigene Lage (Ost, Nord in m), damit die Karte vier Punkte zeigt
  const others = [
    ["Kanal Bahnhofstraße", "Leitungsgraben DN 300, 140 m", 1, 1400, 900],
    ["Haufwerk Bauhof Nord", "Oberboden-Mieten, monatliche Bestandsaufnahme", 2, -900, 1300],
    ["Regenrückhaltebecken Am Wiesengrund", "Erdbecken 1.800 m³", 1, 600, -1100],
  ] as const;
  for (const [name, description, flights, east, north] of others) {
    const other = await api("POST", "/sites", { name, description });
    for (let i = 0; i < flights; i++) {
      const day = START - (20 - 7 * i) * DAY;
      await upload(
        other.id,
        `flug-${i + 1}.xyz`,
        `Befliegung ${i + 1}`,
        day,
        terrainXyz(i > 0, east, north),
      );
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

test("Flugplanung @screenshots", async () => {
  await go(`/sites/${siteId}`);
  const flight = page.getByRole("region", { name: "Flugplanung" });
  await flight.getByLabel("Bereich").selectOption({ label: "Grube" });
  await expect(flight.getByRole("img", { name: /Bahnen über „Grube“/ })).toBeVisible();
  await flight.scrollIntoViewIfNeeded();
  await flight.screenshot({ path: `${DOCS}/screenshot-flugplanung.png` });
});

test("Passkeys und Tokens @screenshots", async () => {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("WebAuthn.enable");
  await cdp.send("WebAuthn.addVirtualAuthenticator", {
    options: {
      protocol: "ctap2",
      transport: "internal",
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });
  await go("/account");
  const passkeys = page.getByRole("region", { name: "Passkeys" });
  await passkeys.getByLabel("Bezeichnung").fill("iPhone Bauleitung");
  await passkeys.getByRole("button", { name: "Passkey hinzufügen" }).click();
  await expect(passkeys.getByRole("listitem")).toHaveCount(1);
  const tokens = page.getByRole("region", { name: "API-Tokens" });
  await tokens.getByLabel("Wofür").fill("iPad Polier");
  await tokens.getByRole("button", { name: "Token anlegen" }).click();
  await expect(tokens.getByRole("status")).toBeVisible();
  // boundingBox misst im Fenster, clip mit fullPage im Dokument
  const [a, b] = [await passkeys.boundingBox(), await tokens.boundingBox()];
  const scroll = await page.evaluate(() => window.scrollY);
  await page.screenshot({
    path: `${DOCS}/screenshot-passkeys.png`,
    fullPage: true,
    clip: { x: a!.x, y: a!.y + scroll, width: a!.width, height: b!.y + b!.height - a!.y },
  });
  await cdp.detach();
});

test("Überblick @screenshots", async () => {
  await go(`/sites/${siteId}`);
  const overview = page.getByRole("region", { name: "Überblick" });
  await expect(overview.getByRole("list").first().getByRole("listitem")).toHaveCount(2);
  await overview.screenshot({ path: `${DOCS}/screenshot-ueberblick.png` });
});

test("Überblick anpassen @screenshots", async () => {
  await go(`/sites/${siteId}`);
  const overview = page.getByRole("region", { name: "Überblick" });
  await overview.getByRole("button", { name: "Anpassen" }).click();
  await expect(overview.getByRole("list", { name: "Anpassen" })).toBeVisible();
  await overview.screenshot({ path: `${DOCS}/screenshot-ueberblick-anpassen.png` });
  await overview.getByRole("button", { name: "Fertig" }).click();
});

test("Bericht zusammenstellen @screenshots", async () => {
  await go(`/sites/${siteId}`);
  await page
    .getByRole("region", { name: "Überblick" })
    .getByRole("button", { name: "Bericht" })
    .click();
  const dialog = page.getByRole("dialog", { name: "Bericht zusammenstellen" });
  await dialog.getByLabel("Titel").fill("Wochenbericht KW 47");
  await dialog.getByLabel("Erstellt von (Firma)").fill("Muster Tiefbau GmbH");
  // Fotos und Notizen ans Ende, Leitungen raus
  const parts = dialog.getByRole("list", { name: "Bausteine" });
  const lines = parts.getByLabel("Leitungen", { exact: true });
  await lines.click();
  await expect(lines).not.toBeChecked();
  await dialog.screenshot({ path: `${DOCS}/screenshot-bericht-zusammenstellen.png` });
  const pdf = page.waitForEvent("download");
  await dialog.getByRole("link", { name: "PDF herunterladen" }).click();
  const file = `${DOCS}/beispiel-baustellenbericht.pdf`;
  await (await pdf).saveAs(file);
  const render = spawnSync("pdftoppm", [
    "-png",
    "-singlefile",
    "-r",
    "110",
    file,
    `${DOCS}/screenshot-pdf-baustellenbericht`,
  ]);
  if (render.error) console.warn("pdftoppm fehlt, Vorschaubild nicht erneuert");
  await page.keyboard.press("Escape");
});

test("Abrechnung @screenshots", async () => {
  const regions = (await api("GET", `/sites/${siteId}/regions`)) as { id: number; name: string }[];
  const grube = regions.find((r) => r.name === "Grube")!;
  await api("PATCH", `/regions/${grube.id}`, {
    material: "Boden Klasse 4",
    targetM3: 1000,
    targetKind: "cut",
    lvPosition: "02.01.0010",
    unitPrice: 14.5,
    priceUnit: "t",
  });
  await api("PUT", `/sites/${siteId}/billing/settings`, {
    truckT: 15,
    materials: [{ name: "Boden Klasse 4", density: 2, bulking: 1.25 }],
  });
  // Zeitraum Zwischenstand → Aushub fertig: 400 m³ fest = 800 t; geliefert 420 + 12 × 15 + 190 = 790 t
  for (const [date, quantity, unit, reference] of [
    ["2026-11-12", 420, "t", "WS-2211"],
    ["2026-11-14", 12, "loads", "LS-0815"],
    ["2026-11-16", 190, "t", "WS-2247"],
  ] as const)
    await api("POST", `/sites/${siteId}/deliveries`, {
      date,
      material: "Boden Klasse 4",
      quantity,
      unit,
      reference,
    });
  await go(`/sites/${siteId}`);
  const billing = page.getByRole("region", { name: "Abrechnung" });
  await expect(billing.getByRole("table", { name: "Abgleich mit Lieferscheinen" })).toBeVisible();
  await billing.screenshot({ path: `${DOCS}/screenshot-abrechnung.png` });
});

test("Freigabe @screenshots", async ({ browser }) => {
  const caps = (await api("GET", `/sites/${siteId}/captures`)) as { id: number; name: string }[];
  const from = caps.find((c) => c.name === "Zwischenstand")!.id;
  const to = caps.find((c) => c.name === "Aushub fertig")!.id;
  const share = await api("POST", `/sites/${siteId}/billing/shares`, {
    from,
    to,
    note: "Abschlagsrechnung 2 – bitte bis 24.11. prüfen",
  });
  const ctx = await browser.newContext({
    viewport: DESKTOP,
    locale: "de-DE",
    timezoneId: "Europe/Berlin",
    reducedMotion: "reduce",
  });
  const guest = await ctx.newPage();
  await guest.goto(`/freigabe/${share.token}`);
  const form = guest.getByRole("form", { name: "Freigabe" });
  await form.getByLabel("Ihr Name").fill("M. Muster");
  await form.getByLabel("Firma (optional)").fill("Bauherr GmbH");
  await guest.screenshot({ path: `${DOCS}/screenshot-freigabe.png`, fullPage: true });
  await ctx.close();
});

test("Orthofoto @screenshots", async () => {
  await go(`/sites/${siteId}`);
  const view = page.getByRole("region", { name: "Ansicht" });
  await expect(view.getByRole("group", { name: "Hintergrund" })).toBeVisible();
  await page.waitForTimeout(400);
  await view.getByRole("application").scrollIntoViewIfNeeded();
  await view.getByRole("application").screenshot({ path: `${DOCS}/screenshot-orthofoto.png` });
});

test("Ansicht mit Differenz @screenshots", async () => {
  await go(`/sites/${siteId}`);
  const view = page.getByRole("region", { name: "Ansicht" });
  await view.getByRole("group", { name: "Hintergrund" }).getByText("Höhenbild").click();
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

test("3D-Ansicht @screenshots", async () => {
  await go(`/sites/${siteId}`);
  const view = page.getByRole("region", { name: "Ansicht" });
  await view.getByLabel("Darüber anzeigen").selectOption({ index: 1 });
  await view.getByRole("group", { name: "Darstellung" }).getByText("3D", { exact: true }).click();
  const terrain = view.getByRole("application", { name: /Gelände in 3D/ });
  await expect(terrain).toHaveAttribute("data-triangles", /\d+/);
  await page.waitForTimeout(500);
  await view.scrollIntoViewIfNeeded();
  await view.screenshot({ path: `${DOCS}/screenshot-3d.png` });
  await view.getByRole("group", { name: "Darstellung in 3D" }).getByText("Punktwolke").click();
  await expect(terrain).toHaveAttribute("data-points", /\d+/);
  await page.waitForTimeout(500);
  await terrain.screenshot({ path: `${DOCS}/screenshot-punktwolke.png` });
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

/** Dokumentation als PDF (Zwangspunkt, Notiz, Foto) wie der Mengenbericht: Beispiel und Vorschau von Seite 1 */
test("PDF-Dokumentation @screenshots", async () => {
  await go(`/sites/${siteId}`);
  const pdf = page.waitForEvent("download");
  await page
    .getByRole("region", { name: "Dokumentation" })
    .getByRole("link", { name: "Bericht (PDF)" })
    .click();
  const file = `${DOCS}/beispiel-dokumentation.pdf`;
  await (await pdf).saveAs(file);
  const render = spawnSync("pdftoppm", [
    "-png",
    "-singlefile",
    "-r",
    "110",
    file,
    `${DOCS}/screenshot-pdf-dokumentation`,
  ]);
  if (render.error) console.warn("pdftoppm fehlt, Vorschaubild nicht erneuert");
});

test("Drohnenfotos @screenshots", async () => {
  await go(`/sites/${siteId}`);
  const section = page.getByRole("region", { name: "Drohnenfotos auswerten" });
  await expect(section.getByText(/NodeODM 2\.5\.2/)).toBeVisible();
  // Platzhalter statt echter Fotos; nur Anzahl und Größe erscheinen im Bild
  const photos = Array.from({ length: 40 }, (_, i) => ({
    name: `DJI_${String(i + 1).padStart(4, "0")}.JPG`,
    mimeType: "image/jpeg",
    buffer: Buffer.alloc(250_000),
  }));
  await section.getByLabel("Fotos", { exact: true }).setInputFiles(photos);
  await section.getByLabel("Name der Aufnahme").fill("Befliegung 4");
  await section.scrollIntoViewIfNeeded();
  await section.screenshot({ path: `${DOCS}/screenshot-drohnenfotos.png` });
  await page.reload();
});

test("Passpunkte @screenshots", async () => {
  await go(`/sites/${siteId}`);
  const calc = page.getByRole("region", { name: "Berechnungen" });
  await calc.getByText("Passpunkte", { exact: true }).click();
  // Scan-Koordinaten mit Messrauschen von wenigen Millimetern, P4 mit einem Fehler von 8 cm
  const noise = [
    [0.004, -0.003, 0.006],
    [-0.002, 0.005, -0.004],
    [0.003, 0.002, -0.005],
    [0.06, -0.05, 0.02],
  ];
  const list = [
    [2, 3],
    [37, 4],
    [36, 38],
    [3, 37],
  ]
    .map(([dx, dy], i) => {
      const dst = [X0 + dx!, Y0 + dy!, 300 + 0.01 * dx!];
      const src = dst.map((v, k) => v - [X0, Y0, 0][k]! + noise[i]![k]!);
      return [`P${i + 1}`, ...src.map((v) => v.toFixed(3)), ...dst.map((v) => v.toFixed(3))]
        .join(";")
        .replaceAll(".", ",");
    })
    .join("\n");
  await calc.getByText("Liste einfügen").click();
  await calc.getByLabel(/Eine Zeile je Punkt/).fill(list);
  await calc.getByRole("button", { name: "4 Punkte übernehmen" }).click();
  await expect(calc.getByRole("table", { name: "Restklaffen je Punkt" })).toBeVisible();
  await calc.scrollIntoViewIfNeeded();
  await calc.screenshot({ path: `${DOCS}/screenshot-passpunkte.png` });
  await page.reload();
});

test("Pläne @screenshots", async () => {
  await go(`/sites/${siteId}`);
  const plans = page.getByRole("region", { name: "Pläne" });
  await expect(plans.getByRole("table", { name: "Plan/Ist" })).toBeVisible();
  await plans.scrollIntoViewIfNeeded();
  await plans.screenshot({ path: `${DOCS}/screenshot-plaene.png` });
  const view = page.getByRole("region", { name: "Ansicht" });
  await view.getByLabel("Plan darüber").selectOption({ label: "Lageplan" });
  await page.waitForTimeout(300);
  await view.getByRole("application").scrollIntoViewIfNeeded();
  await view.getByRole("application").screenshot({ path: `${DOCS}/screenshot-ansicht-plan.png` });
  await page.reload();
});

test("Dokumentation @screenshots", async () => {
  await go(`/sites/${siteId}`);
  const doc = page.getByRole("region", { name: "Dokumentation" });
  await expect(doc.getByRole("img", { name: "Schacht" })).toBeVisible();
  await doc.scrollIntoViewIfNeeded();
  await doc.screenshot({ path: `${DOCS}/screenshot-dokumentation.png` });
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
