import { describe, expect, it } from "vitest";
import { move, toggle } from "./layout";

const items = ["a", "b", "c"].map((id) => ({ id, visible: true }));

describe("Überblick anpassen", () => {
  it("verschiebt um einen Platz, am Rand nicht", () => {
    expect(move(items, 2, -1).map((i) => i.id)).toEqual(["a", "c", "b"]);
    expect(move(items, 0, 1).map((i) => i.id)).toEqual(["b", "a", "c"]);
    expect(move(items, 0, -1)).toBe(items);
    expect(move(items, 2, 1)).toBe(items);
  });

  it("blendet einzeln aus und ein", () => {
    const t = toggle(items, "b");
    expect(t.map((i) => i.visible)).toEqual([true, false, true]);
    expect(toggle(t, "b")).toEqual(items);
  });
});
