import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { defaultPair } from "@/components/site/calculations";
import { captureName } from "@/components/site/captures";
import { StatusBadge } from "@/components/ui/status";
import type { Capture } from "@/lib/api";

const cap = (id: number, capturedAt: number): Capture => ({
  id,
  siteId: 1,
  name: `A${id}`,
  fileName: "a.laz",
  fileSize: 1,
  capturedAt,
  status: "ready",
  info: null,
  error: null,
  createdAt: 0,
});

describe("Baustelle", () => {
  it("Name aus dem Dateinamen", () => {
    expect(captureName("odm_georeferenced_model.laz")).toBe("odm_georeferenced_model");
    expect(captureName("Flug 2026-11-03.tif")).toBe("Flug 2026-11-03");
  });

  it("Vorauswahl älteste → jüngste Aufnahme", () => {
    expect(defaultPair([cap(2, 300), cap(1, 100), cap(3, 200)])).toEqual([1, 2]);
    expect(defaultPair([cap(1, 100)])).toEqual([1, undefined]);
    expect(defaultPair([])).toEqual([undefined, undefined]);
  });

  it("Status hat immer ein Wort, nicht nur eine Farbe", () => {
    render(<StatusBadge status="failed" />);
    expect(screen.getByText("Fehlgeschlagen")).toBeTruthy();
  });
});
