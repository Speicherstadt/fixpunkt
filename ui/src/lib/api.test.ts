import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiError, api, errorOf, requestHeaders, setCsrf } from "@/lib/api";

afterEach(() => {
  vi.restoreAllMocks();
  setCsrf(null);
});

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

describe("api", () => {
  it("schickt das CSRF-Token nur bei ändernden Anfragen", () => {
    setCsrf("abc");
    expect(requestHeaders("GET", false)).toEqual({});
    expect(requestHeaders("POST", true)).toEqual({
      "content-type": "application/json",
      "x-csrf-token": "abc",
    });
    expect(requestHeaders("DELETE", false)).toEqual({ "x-csrf-token": "abc" });
  });

  it("liest Fehlerschlüssel, sonst den HTTP-Status", async () => {
    const e = await errorOf(json({ error: "auth.wrongCredentials" }, 401));
    expect(e).toBeInstanceOf(ApiError);
    expect([e.status, e.code]).toEqual([401, "auth.wrongCredentials"]);
    expect((await errorOf(new Response("Bad Gateway", { status: 502 }))).code).toBe("http.502");
  });

  it("prüft Antworten mit Zod und sendet JSON", async () => {
    const fetch = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValue(
        json({ id: 1, name: "Baugrube", description: "", epsg: 25832, createdAt: 1 }, 201),
      );
    setCsrf("t");
    const site = await api.createSite("Baugrube", "");
    expect(site.name).toBe("Baugrube");
    const [url, init] = fetch.mock.calls[0]!;
    expect(url).toBe("/api/sites");
    expect(init?.method).toBe("POST");
    expect(JSON.parse(String(init?.body))).toEqual({ name: "Baugrube", description: "" });
    const headers = (init?.headers ?? {}) as Record<string, string>;
    expect(headers["x-csrf-token"]).toBe("t");
  });

  it("204 ohne Körper und ungültige Antworten", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce(new Response(null, { status: 204 }));
    await expect(api.logout()).resolves.toBeUndefined();
    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce(json({ unerwartet: true }));
    await expect(api.site(1)).rejects.toThrow();
  });
});
