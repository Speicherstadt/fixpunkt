/**
 * Server-Fehlertexte: jeder Fehlerschlüssel aus crates/server/src/error.rs hat einen Text server_….
 * Beide Sprachen vollständig, gleiche Platzhalter und jeder Text genutzt prüft `checks i18n`
 * (@firsttris/checks, checks.json) in `bun run lint`.
 */
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const { $schema: _, ...de } = JSON.parse(readFileSync("messages/de.json", "utf8")) as Record<
  string,
  string
>;

describe("messages", () => {
  it("Server-Fehlertexte gibt es für alle Schlüssel aus crates/server/src/error.rs", () => {
    const rust = readFileSync("../crates/server/src/error.rs", "utf8");
    const codes = [...rust.matchAll(/pub const \w+: &str = "([\w.]+)";/g)].map(
      (m) => `server_${m[1]!.replace(".", "_")}`,
    );
    expect(codes.length).toBeGreaterThan(10);
    expect(codes.filter((k) => !(k in de))).toEqual([]);
  });
});
