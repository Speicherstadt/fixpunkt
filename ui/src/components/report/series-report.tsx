import { Section } from "@/components/page";
import type { SeriesResult } from "@/lib/api";
import { formatVolume } from "@/lib/format";
import { seriesRows } from "@/lib/report";
import { m } from "@/paraglide/messages";
import { Kpis } from "./kpis";

/** Zeitreihe: Abtrag je Aufnahme seit dem Urgelände und seit der vorigen */
export function SeriesReport({ result }: { result: SeriesResult }) {
  const rows = seriesRows(result.steps);
  const last = rows.at(-1);
  return (
    <div className="flex flex-col gap-6">
      <Kpis
        items={[
          { label: m.series_total(), value: formatVolume(last?.total ?? 0), className: "text-cut" },
          { label: m.series_last_step(), value: formatVolume(last?.step ?? 0) },
          { label: m.series_flights(), value: String(rows.length + 1) },
        ]}
      />
      <Section title={m.series_progress()} description={m.series_base({ name: result.base })}>
        <div className="overflow-x-auto">
          <table className="w-full border-collapse text-[14px]">
            <caption className="sr-only">{m.series_progress()}</caption>
            <thead>
              <tr className="text-left text-[12px] text-muted">
                <th scope="col" className="py-2 pr-3 font-medium">
                  {m.series_capture()}
                </th>
                <th scope="col" className="w-2/5 py-2 pr-3 font-medium">
                  <span className="sr-only">{m.series_bar()}</span>
                </th>
                <th scope="col" className="py-2 pl-4 text-right font-medium">
                  {m.series_step()}
                </th>
                <th scope="col" className="py-2 pl-4 text-right font-medium">
                  {m.series_total()}
                </th>
                <th scope="col" className="py-2 pl-4 text-right font-medium">
                  {m.report_net()}
                </th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.label} className="border-t border-border">
                  <th scope="row" className="py-2.5 pr-3 text-left font-[inherit]">
                    {r.label}
                  </th>
                  <td className="py-2.5 pr-3">
                    <span className="block h-3 rounded-sm bg-subtle">
                      <span
                        className="block h-3 rounded-sm bg-cut"
                        style={{ width: `${r.share * 100}%` }}
                      />
                    </span>
                  </td>
                  <td className="tabular py-2.5 pl-4 text-right whitespace-nowrap">
                    {formatVolume(r.step)}
                  </td>
                  <td className="tabular py-2.5 pl-4 text-right whitespace-nowrap text-cut">
                    {formatVolume(r.total)}
                  </td>
                  <td className="tabular py-2.5 pl-4 text-right whitespace-nowrap">
                    {formatVolume(r.net)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Section>
    </div>
  );
}
