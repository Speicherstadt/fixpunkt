/**
 * Texte: beide Sprachen haben dieselben Schlüssel und Platzhalter, und jeder Text wird genutzt
 * (im Code als m.schlüssel oder als Fehlerschlüssel des Servers server_…).
 */
import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const load = (lang: string): Record<string, string> => {
  const { $schema: _, ...rest } = JSON.parse(readFileSync(`messages/${lang}.json`, "utf8"));
  return rest;
};
const de = load("de");
const en = load("en");
const placeholders = (t: string) => [...t.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

const code = readdirSync("src", { recursive: true, encoding: "utf8" })
  .filter((f) => /\.tsx?$/.test(f) && !f.includes("paraglide") && !f.endsWith(".test.ts"))
  .map((f) => readFileSync(`src/${f}`, "utf8"))
  .join("\n");

describe("messages", () => {
  it("de und en haben dieselben Schlüssel", () => {
    expect(Object.keys(en).sort()).toEqual(Object.keys(de).sort());
  });

  it("gleiche Platzhalter in beiden Sprachen", () => {
    for (const key of Object.keys(de))
      expect([key, placeholders(en[key] ?? "")]).toEqual([key, placeholders(de[key]!)]);
  });

  it("jeder Text wird verwendet", () => {
    const unused = Object.keys(de).filter(
      (k) => !k.startsWith("server_") && !code.includes(`m.${k}`),
    );
    expect(unused).toEqual([]);
  });

  it("Server-Fehlertexte gibt es für alle Schlüssel aus crates/server/src/error.rs", () => {
    const rust = readFileSync("../crates/server/src/error.rs", "utf8");
    const codes = [...rust.matchAll(/pub const \w+: &str = "([\w.]+)";/g)].map(
      (m) => `server_${m[1]!.replace(".", "_")}`,
    );
    expect(codes.length).toBeGreaterThan(10);
    expect(codes.filter((k) => !(k in de))).toEqual([]);
  });
});
