/**
 * Testdaten über die API anlegen (schneller als über die Oberfläche): mit der Sitzung der Seite und ihrem
 * CSRF-Token. Für Tests, die eine fertige Baustelle brauchen, und für die Screenshots.
 */
import { expect, type Page } from "@playwright/test";

/** Antworten der eigenen API; die Tests lesen gezielt einzelne Felder */
// biome-ignore lint/suspicious/noExplicitAny: Testdaten, die Form prüft die UI-Schicht mit Zod
type Data = any;

export type Api = {
  call: (method: string, path: string, data?: unknown) => Promise<Data>;
  upload: (
    siteId: number,
    file: string,
    name: string,
    capturedAt: number,
    body: string,
  ) => Promise<number>;
  /** Wartet, bis alle Aufnahmen geprüft und alle Aufträge der Baustelle fertig sind */
  settled: (siteId: number) => Promise<void>;
};

export const apiFor = async (page: Page): Promise<Api> => {
  if (!page.url().startsWith("http")) await page.goto("/");
  const state = await (await page.request.get("/api/auth/state")).json();
  const csrf: string = state.csrf;
  const call = async (method: string, path: string, data?: unknown) => {
    const res = await page.request.fetch(`/api${path}`, {
      method,
      headers: { "x-csrf-token": csrf, "content-type": "application/json" },
      data: data === undefined ? undefined : JSON.stringify(data),
    });
    expect(res.ok(), `${method} ${path}: ${await res.text()}`).toBe(true);
    return res.status() === 204 ? null : res.json();
  };
  const upload = async (
    siteId: number,
    file: string,
    name: string,
    capturedAt: number,
    body: string,
  ) => {
    const q = new URLSearchParams({ file, name, capturedAt: String(capturedAt) });
    const res = await page.request.put(`/api/sites/${siteId}/captures?${q}`, {
      headers: { "x-csrf-token": csrf, "content-type": "application/octet-stream" },
      data: Buffer.from(body),
    });
    expect(res.ok()).toBe(true);
    return (await res.json()).id as number;
  };
  const settled = async (siteId: number) => {
    await expect
      .poll(
        async () => {
          const caps = (await call("GET", `/sites/${siteId}/captures`)) as { status: string }[];
          const jobs = (await call("GET", `/sites/${siteId}/jobs`)) as { status: string }[];
          return [...caps, ...jobs].every((x) => x.status === "ready" || x.status === "done");
        },
        { timeout: 60_000 },
      )
      .toBe(true);
  };
  return { call, upload, settled };
};
