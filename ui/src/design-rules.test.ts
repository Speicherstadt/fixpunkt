/**
 * Gestaltung nach festen Stufen wie in Kontext (src/design-rules.test.ts): Schriftgrößen, Symbolgrößen,
 * Farben nur als Token. Ein Wert außerhalb schleicht sich Klasse für Klasse ein; dieser Test nennt die Zeile.
 */
import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const FONT_SIZES = new Set([12, 13, 14, 16, 18, 22]);
const ICON_SIZES = new Set([14, 16, 20, 24]);

const lines = readdirSync("src", { recursive: true, encoding: "utf8" })
  .filter((f) => f.endsWith(".tsx") && !f.endsWith(".test.tsx"))
  .flatMap((f) =>
    readFileSync(`src/${f}`, "utf8")
      .split("\n")
      .map((text, i) => ({ where: `src/${f}:${i + 1}`, text })),
  );

const offenders = (pattern: RegExp, bad: (m: RegExpMatchArray) => boolean = () => true) =>
  lines
    .filter(({ text }) => [...text.matchAll(pattern)].some(bad))
    .map(({ where, text }) => `${where}: ${text.trim()}`);

describe("Gestaltungsregeln", () => {
  it("Schriftgrößen auf der Skala", () => {
    expect(offenders(/text-\[(\d+)px\]/g, (m) => !FONT_SIZES.has(Number(m[1])))).toEqual([]);
  });

  it("Symbolgrößen auf der Skala, Linienstärke nur aus styles.css", () => {
    expect(offenders(/\bsize=\{(\d+)\}/g, (m) => !ICON_SIZES.has(Number(m[1])))).toEqual([]);
    expect(offenders(/strokeWidth=/g)).toEqual([]);
  });

  it("Farben nur über Token (kein Hex, keine Tailwind-Palette)", () => {
    expect(offenders(/#[0-9a-fA-F]{3,8}\b/g)).toEqual([]);
    expect(
      offenders(/\b(?:text|bg|border)-(?:red|green|blue|amber|yellow|zinc|gray)-\d{2,3}\b/g),
    ).toEqual([]);
  });

  it("Symbole neben Text sind für Screenreader ausgeblendet", () => {
    const icons = lines.filter(
      ({ text }) => /<[A-Z]\w+ size=\{\d+\}/.test(text) && !text.includes("aria-hidden"),
    );
    expect(icons.map(({ where }) => where)).toEqual([]);
  });
});
