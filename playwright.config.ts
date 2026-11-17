import { defineConfig, devices } from "@playwright/test";

/**
 * E2E gegen die echte Programmdatei (`bun run e2e` baut UI und Server vorher): eigener Port, frisches
 * Datenverzeichnis je Lauf, damit ein laufender Entwicklungsserver nicht berührt wird.
 * Vorbild: firsttris/kontext playwright.config.ts
 */
const PORT = 28080;
const DATA = ".e2e-data";

const launchOptions = process.env.PW_CHROMIUM_PATH
  ? { executablePath: process.env.PW_CHROMIUM_PATH }
  : {};

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  workers: 1,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["github"], ["html", { open: "never" }]] : "list",
  timeout: 60_000,
  expect: { timeout: 15_000 },
  globalSetup: "./e2e/global-setup.ts",
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    locale: "de-DE",
    timezoneId: "Europe/Berlin",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    launchOptions,
  },
  projects: [
    { name: "setup", testMatch: /setup\.spec\.ts/ },
    {
      name: "desktop",
      dependencies: ["setup"],
      testIgnore: /setup\.spec\.ts/,
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1440, height: 900 },
        launchOptions,
        storageState: "e2e/.auth/admin.json",
        serviceWorkers: "block",
      },
      grepInvert: /@phone/,
    },
    {
      name: "phone",
      dependencies: ["setup"],
      testIgnore: /setup\.spec\.ts/,
      use: {
        ...devices["Pixel 7"],
        launchOptions,
        storageState: "e2e/.auth/admin.json",
        serviceWorkers: "block",
      },
      grep: /@phone/,
    },
  ],
  webServer: {
    command: `rm -rf ${DATA} && FIXPUNKT_DATA_DIR=${DATA} FIXPUNKT_LISTEN=127.0.0.1:${PORT} ./target/debug/fixpunkt`,
    url: `http://127.0.0.1:${PORT}/api/health`,
    reuseExistingServer: false,
    timeout: 60_000,
    stdout: "pipe",
  },
});
