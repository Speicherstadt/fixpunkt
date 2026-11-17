import { expect, request, test } from "@playwright/test";

/**
 * Passkey anlegen und damit anmelden – mit Chromes virtuellem Authenticator (CDP WebAuthn, Nutzerprüfung
 * automatisch bestätigt). Dazu ein API-Token, das ohne Cookie und CSRF liest und nach dem Widerruf nicht mehr.
 */
test("Passkey und API-Token", async ({ browser, baseURL }) => {
  const ctx = await browser.newContext({ storageState: undefined });
  const page = await ctx.newPage();
  const cdp = await ctx.newCDPSession(page);
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

  await page.goto("/login");
  await page.getByLabel("Name").fill("admin");
  await page.getByLabel("Passwort").fill("geheim123");
  await page.getByRole("button", { name: "Anmelden", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Baustellen", level: 1 })).toBeVisible();

  await page.goto("/account");
  const passkeys = page.getByRole("region", { name: "Passkeys" });
  await expect(passkeys).toContainText("Noch kein Passkey angelegt.");
  await passkeys.getByLabel("Bezeichnung").fill("Testgerät");
  await passkeys.getByRole("button", { name: "Passkey hinzufügen" }).click();
  await expect(passkeys.getByRole("listitem")).toContainText(["Testgerät"]);
  await expect(passkeys.getByRole("listitem")).toContainText(["noch nie benutzt"]);

  // Token anlegen, einmal sichtbar
  const tokens = page.getByRole("region", { name: "API-Tokens" });
  await tokens.getByLabel("Wofür").fill("Skript");
  await tokens.getByRole("button", { name: "Token anlegen" }).click();
  const token = (await tokens.getByRole("status").locator("code").textContent())!;
  expect(token).toMatch(/^fxp_[0-9a-f]{64}$/);

  // ohne Sitzung: sonst übernimmt der Kontext die des Admins aus der Konfiguration
  const api = await request.newContext({ baseURL, storageState: { cookies: [], origins: [] } });
  const auth = { authorization: `Bearer ${token}` };
  expect((await api.get("/api/sites", { headers: auth })).status()).toBe(200);
  expect((await api.get("/api/sites")).status()).toBe(401);

  // abmelden und mit Passkey wieder anmelden
  await page.getByRole("button", { name: "Abmelden" }).click();
  await expect(page.getByRole("heading", { name: "Anmelden" })).toBeVisible();
  await page.getByRole("button", { name: "Mit Passkey anmelden" }).click();
  await expect(page.getByRole("alert")).toHaveText(/zuerst den Namen/);
  await page.getByLabel("Name").fill("admin");
  await page.getByRole("button", { name: "Mit Passkey anmelden" }).click();
  await expect(page.getByRole("heading", { name: "Baustellen", level: 1 })).toBeVisible();

  await page.goto("/account");
  await expect(passkeys.getByRole("listitem")).toContainText([/zuletzt benutzt/]);
  // Widerruf: Token gilt nicht mehr, Passkey weg
  await tokens.getByRole("button", { name: "„Skript“ löschen" }).click();
  await expect(tokens).toContainText("Noch kein Token angelegt.");
  expect((await api.get("/api/sites", { headers: auth })).status()).toBe(401);
  await passkeys.getByRole("button", { name: "„Testgerät“ löschen" }).click();
  await expect(passkeys).toContainText("Noch kein Passkey angelegt.");
  await api.dispose();
  await ctx.close();
});
