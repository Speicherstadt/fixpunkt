import { formatArea, formatVolume } from "@/lib/format";
import type { ReportRow } from "@/lib/report";
import { m } from "@/paraglide/messages";

/** Mengenbericht als Tabelle: je Bereich Fläche, Abtrag, Auftrag, Bilanz */
export function VolumeTable({ rows, total }: { rows: ReportRow[]; total: ReportRow }) {
  const cell = "tabular py-2.5 pl-4 text-right whitespace-nowrap";
  const line = (r: ReportRow, strong = false) => (
    <tr
      key={r.name}
      className={strong ? "border-t-2 border-fg font-semibold" : "border-t border-border"}
    >
      <th scope="row" className="py-2.5 pr-3 text-left font-[inherit]">
        {r.name}
      </th>
      <td className={cell}>{formatArea(r.area)}</td>
      <td className={`${cell} text-cut`}>{formatVolume(r.cut)}</td>
      <td className={`${cell} text-fill`}>{formatVolume(r.fill)}</td>
      <td className={cell}>{formatVolume(r.net)}</td>
    </tr>
  );
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[14px]">
        <caption className="sr-only">{m.report_table_caption()}</caption>
        <thead>
          <tr className="text-left text-[12px] text-muted">
            <th scope="col" className="py-2 pr-3 font-medium">
              {m.report_region()}
            </th>
            <th scope="col" className="py-2 pl-4 text-right font-medium">
              {m.report_area()}
            </th>
            <th scope="col" className="py-2 pl-4 text-right font-medium">
              {m.report_cut()}
            </th>
            <th scope="col" className="py-2 pl-4 text-right font-medium">
              {m.report_fill()}
            </th>
            <th scope="col" className="py-2 pl-4 text-right font-medium">
              {m.report_net()}
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => line(r))}
          {line(total, true)}
        </tbody>
      </table>
    </div>
  );
}
