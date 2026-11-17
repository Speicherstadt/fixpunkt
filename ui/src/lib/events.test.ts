import { describe, expect, it } from "vitest";
import { staleKeys } from "@/lib/events";

describe("staleKeys", () => {
  it("Auftrag: Auftrag, Liste der Baustelle, Übersicht", () => {
    expect(staleKeys({ type: "job", id: 7, siteId: 3, status: "done" })).toEqual([
      ["job", 7],
      ["jobs", 3],
      ["dashboard", 3],
      ["billing", 3],
      ["sites"],
    ]);
  });

  it("Änderungen je Thema", () => {
    expect(staleKeys({ type: "changed", topic: "captures", siteId: 3 })).toEqual([
      ["captures", 3],
      ["dashboard", 3],
      ["billing", 3],
      ["sites"],
    ]);
    expect(staleKeys({ type: "changed", topic: "sites", siteId: null })).toEqual([
      ["sites"],
      ["site"],
    ]);
    expect(staleKeys({ type: "changed", topic: "users" })).toEqual([["users"]]);
    expect(staleKeys({ type: "changed", topic: "regions" })).toEqual([
      ["regions"],
      ["dashboard"],
      ["billing"],
      ["sites"],
    ]);
    expect(staleKeys({ type: "changed", topic: "billing", siteId: 3 })).toEqual([
      ["billing", 3],
      ["dashboard", 3],
      ["sites"],
    ]);
  });
});
