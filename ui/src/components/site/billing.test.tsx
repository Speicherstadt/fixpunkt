import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ChecksTable, PositionsTable } from "@/components/site/billing-tables";
import type { Statement } from "@/lib/api";

const at = (id: number, name: string) => ({ id, name, at: id * 86_400_000 });

const statement: Statement = {
  base: at(1, "Urgelände"),
  from: at(2, "Januar"),
  to: at(3, "Februar"),
  missing: [],
  truckT: 15,
  amount: 12740,
  unpriced: 1,
  unconverted: 0,
  layersOutdated: false,
  positions: [
    {
      regionId: 1,
      name: "Grube",
      layer: null,
      lvPosition: "01.02.0010",
      material: "Lehm",
      kind: "cut",
      previous: 300,
      total: 800,
      period: { bankM3: 500, looseM3: 625, tonnes: 1000, loads: 67 },
      unit: "t",
      unitPrice: 12.5,
      quantity: 1000,
      amount: 12500,
    },
    {
      regionId: 3,
      name: "Rand",
      layer: null,
      lvPosition: null,
      material: null,
      kind: "cut",
      previous: null,
      total: null,
      period: null,
      unit: null,
      unitPrice: null,
      quantity: null,
      amount: null,
    },
  ],
  checks: [
    {
      material: "Lehm",
      count: 3,
      delivered: [["t", 600]],
      deliveredBank: 575,
      measuredBank: 500,
      deviationPercent: 15,
    },
  ],
};

describe("Aufmaß", () => {
  it("Positionen mit Mengen, Preis und Summe", () => {
    render(<PositionsTable statement={statement} />);
    const table = screen.getByRole("table", { name: "Positionen des Aufmaßes" });
    const rows = within(table).getAllByRole("row");
    const grube = rows[1]!.textContent ?? "";
    for (const v of [
      "01.02.0010",
      "Grube",
      "Lehm",
      "500,0",
      "625,0",
      "1.000,0",
      "67",
      "12,50",
      "€/t",
    ])
      expect(grube).toContain(v);
    expect(grube).toContain("12.500,00");
    // ohne Berechnung und Preis: Striche statt Nullen
    expect(rows[2]!.textContent).toContain("–");
    expect(rows[3]!.textContent).toContain("Summe netto");
    expect(rows[3]!.textContent).toContain("12.740,00");
  });

  it("Abweichung über 10 % ist hervorgehoben", () => {
    render(<ChecksTable checks={statement.checks} />);
    const cell = screen.getByText("+15,0 %");
    expect(cell.className).toContain("text-warn");
    expect(screen.getByText("600,0 t")).toBeTruthy();
  });
});
