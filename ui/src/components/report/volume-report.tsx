import { TriangleAlert } from "lucide-react";
import { Section } from "@/components/page";
import type { VolumeResult } from "@/lib/api";
import { formatVolume } from "@/lib/format";
import { needsCoverageWarning, reportRows } from "@/lib/report";
import { m } from "@/paraglide/messages";
import { Kpis } from "./kpis";
import { VolumeTable } from "./volume-table";

/** Aushub (Abtrag, Auftrag, Bilanz) oder Haufwerk (Volumen über und unter der Basis) */
export function VolumeReport({ result, stockpile }: { result: VolumeResult; stockpile: boolean }) {
  const { rows, total } = reportRows(result.regions, result.total, m.report_total());
  const labels = stockpile
    ? { cut: m.report_below_base(), fill: m.report_stockpile_volume() }
    : { cut: m.report_cut(), fill: m.report_fill() };
  const kpis = stockpile
    ? [
        { label: labels.fill, value: formatVolume(total.fill), className: "text-fill" },
        { label: labels.cut, value: formatVolume(total.cut), className: "text-cut" },
        { label: m.report_area(), value: `${Math.round(total.area)} m²` },
      ]
    : [
        { label: m.report_cut(), value: formatVolume(total.cut), className: "text-cut" },
        { label: m.report_fill(), value: formatVolume(total.fill), className: "text-fill" },
        { label: m.report_net(), value: formatVolume(total.net) },
      ];
  return (
    <div className="flex flex-col gap-6">
      <Kpis items={kpis} />
      {needsCoverageWarning([...rows, total]) ? (
        <p
          role="note"
          className="m-0 flex items-start gap-2 rounded-lg border border-warn px-4 py-3 text-[13px]"
        >
          <TriangleAlert size={16} aria-hidden="true" className="mt-0.5 shrink-0 text-warn" />
          {m.report_coverage_warning()}
        </p>
      ) : null}
      <Section title={m.report_by_region()}>
        <VolumeTable rows={rows} total={total} labels={labels} />
        <p className="m-0 text-[12px] text-muted">
          {stockpile
            ? m.report_method_stockpile({ cm: Math.round(result.cell * 100) })
            : m.report_method({ cm: Math.round(result.cell * 100) })}
        </p>
      </Section>
    </div>
  );
}
