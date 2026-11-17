import { afterEach, describe, expect, it } from "vitest";
import { applyTheme, isDark, storedTheme } from "@/lib/theme";

afterEach(() => localStorage.clear());

describe("theme", () => {
  it("Wahl schlägt Gerät", () => {
    expect(isDark("dark", false)).toBe(true);
    expect(isDark("light", true)).toBe(false);
    expect(isDark("system", true)).toBe(true);
  });

  it("speichert und setzt die Klasse", () => {
    window.matchMedia = ((q: string) => ({ matches: false, media: q })) as typeof window.matchMedia;
    applyTheme("dark");
    expect(storedTheme()).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    applyTheme("system");
    expect(storedTheme()).toBe("system");
    expect(document.documentElement.classList.contains("light")).toBe(true);
  });
});
