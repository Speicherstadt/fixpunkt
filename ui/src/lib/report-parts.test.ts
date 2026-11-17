import { beforeEach, describe, expect, it } from "vitest";
import { defaultReport, loadReport, reportHref, saveReport } from "./report-parts";

describe("eigener Bericht", () => {
  beforeEach(() => localStorage.clear());

  it("baut die Adresse mit Bausteinen in Reihenfolge, Titel und Firma", () => {
    const s = defaultReport();
    s.parts = [
      { id: "stock", visible: true },
      { id: "kpis", visible: false },
      { id: "photos", visible: true },
    ];
    s.title = " Wochenbericht KW 47 ";
    s.company = "Muster & Söhne";
    expect(reportHref(7, s)).toBe(
      "/api/sites/7/report?parts=stock%2Cphotos&title=Wochenbericht+KW+47&company=Muster+%26+S%C3%B6hne",
    );
    s.parts = s.parts.map((p) => ({ ...p, visible: false }));
    expect(reportHref(7, s)).toBeNull();
  });

  it("merkt sich die Auswahl; unbekannte fallen weg, neue kommen dazu", () => {
    expect(loadReport()).toEqual(defaultReport());
    saveReport({
      parts: [
        { id: "photos", visible: false },
        { id: "gibt-es-nicht", visible: true },
      ],
      title: "T",
      company: "F",
    });
    const r = loadReport();
    expect(r.parts[0]).toEqual({ id: "photos", visible: false });
    expect(r.parts).toHaveLength(11);
    expect(r.parts.some((p) => p.id === "gibt-es-nicht")).toBe(false);
    expect([r.title, r.company]).toEqual(["T", "F"]);
    localStorage.setItem("fixpunkt.report", "kaputt");
    expect(loadReport()).toEqual(defaultReport());
  });
});
