import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { VolumeTable } from "@/components/report/volume-table";
import type { ReportRow } from "@/lib/report";

const row = (name: string, cut: number, fill: number): ReportRow => ({
  name,
  area: 900,
  cut,
  fill,
  net: fill - cut,
  missing: 0,
});

describe("VolumeTable", () => {
  it("zeigt je Bereich eine Zeile und die Summe, deutsch formatiert", () => {
    render(
      <VolumeTable
        rows={[row("Grube", 800, 0.5), row("Rand", 0, 0)]}
        total={row("Summe", 800, 0.5)}
      />,
    );
    const table = screen.getByRole("table", { name: "Massen je Bereich" });
    const grube = within(table).getByRole("row", { name: /Grube/ });
    expect(
      within(grube)
        .getAllByRole("cell")
        .map((c) => c.textContent),
    ).toEqual(["900,0 m²", "800,0 m³", "0,5 m³", "-799,5 m³"]);
    expect(within(table).getAllByRole("row")).toHaveLength(4);
    expect(within(table).getByRole("rowheader", { name: "Summe" })).toBeTruthy();
  });
});
