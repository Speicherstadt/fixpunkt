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
    <>
      {/* Am Handy eine Karte je Bereich statt einer Tabelle, die über den Rand ragt */}
      <ul
        className="m-0 flex list-none flex-col gap-3 p-0 md:hidden"
        aria-label={m.report_table_caption()}
      >
        {[...rows, total].map((r, i) => (
          <li
            key={r.name}
            className={`rounded-lg border p-3 ${i === rows.length ? "border-fg" : "border-border"}`}
          >
            <div className="mb-2 font-semibold">{r.name}</div>
            <dl className="m-0 grid grid-cols-2 gap-x-4 gap-y-1 text-[13px]">
              <dt className="text-muted">{m.report_area()}</dt>
              <dd className="tabular m-0 text-right">{formatArea(r.area)}</dd>
              <dt className="text-muted">{m.report_cut()}</dt>
              <dd className="tabular m-0 text-right text-cut">{formatVolume(r.cut)}</dd>
              <dt className="text-muted">{m.report_fill()}</dt>
              <dd className="tabular m-0 text-right text-fill">{formatVolume(r.fill)}</dd>
              <dt className="text-muted">{m.report_net()}</dt>
              <dd className="tabular m-0 text-right font-medium">{formatVolume(r.net)}</dd>
            </dl>
          </li>
        ))}
      </ul>
      <div className="hidden overflow-x-auto md:block">
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
    </>
  );
}
