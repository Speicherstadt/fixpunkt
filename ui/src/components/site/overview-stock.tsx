/** Bestand je Material in den Lagern (Haufwerk-Bereichen): aktuell, Änderung zur Befliegung davor, Stand */
import type { Dashboard } from "@/lib/api";
import { formatDate, formatNumber } from "@/lib/format";
import { m } from "@/paraglide/messages";

type Stock = Dashboard["stock"][number];
type History = Dashboard["stockHistory"];

/** „+40 m³“ / „−450 m³“ / „±0 m³“ */
export const signedM3 = (v: number) => {
  const r = Math.round(v);
  const abs = formatNumber(Math.abs(r), 0);
  return `${r > 0 ? "+" : r < 0 ? "−" : "±"}${abs} m³`;
};

export function StockTable({ stock }: { stock: Stock[] }) {
  const measured = stock.filter((s) => s.current !== null);
  const total = stock.reduce((n, s) => n + s.stockpiles, 0);
  const done = stock.reduce((n, s) => n + s.measured, 0);
  return (
    <div className="flex flex-col gap-2">
      <h3 className="m-0 text-[14px] font-semibold">{m.overview_stock()}</h3>
      {measured.length ? (
        <table className="w-full border-collapse text-[13px]" aria-label={m.overview_stock()}>
          <thead>
            <tr className="border-b border-border text-left text-muted">
              <th className="py-1.5 pr-3 font-medium">{m.overview_stock_material()}</th>
              <th className="py-1.5 pr-3 text-right font-medium">{m.overview_stock_current()}</th>
              <th className="py-1.5 pr-3 text-right font-medium">{m.overview_stock_change()}</th>
              <th className="py-1.5 text-right font-medium">{m.overview_stock_at()}</th>
            </tr>
          </thead>
          <tbody>
            {measured.map((s) => (
              <tr key={s.material} className="border-b border-border">
                <td className="py-1.5 pr-3">{s.material}</td>
                <td className="tabular py-1.5 pr-3 text-right">{formatNumber(s.current!, 0)} m³</td>
                <td className="tabular py-1.5 pr-3 text-right">
                  {s.change === null ? "–" : signedM3(s.change)}
                </td>
                <td className="tabular py-1.5 text-right">{s.at ? formatDate(s.at) : "–"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <p className="m-0 text-[13px] text-muted">{m.overview_stock_none()}</p>
      )}
      {measured.length && done < total ? (
        <p className="m-0 text-[12px] text-muted">
          {m.overview_stock_count({ measured: done, count: total })}
        </p>
      ) : null}
    </div>
  );
}

/** Verlauf je Material als Balken je Befliegung (gleicher Maßstab für alle Materialien) */
export function StockHistory({ history }: { history: History }) {
  const shown = history.filter((h) => h.points.length > 1);
  if (!shown.length) return null;
  const max = Math.max(1, ...shown.flatMap((h) => h.points.map((p) => p.value)));
  return (
    <div className="flex flex-col gap-2">
      <h3 className="m-0 text-[14px] font-semibold">{m.overview_stock_history()}</h3>
      {shown.map((h) => (
        <section
          key={h.material}
          aria-label={m.overview_stock_history_of({ material: h.material })}
        >
          <h4 className="m-0 mb-1 text-[13px] font-medium">{h.material}</h4>
          <ol className="m-0 flex list-none flex-col gap-1 p-0">
            {h.points.map((p) => (
              <li
                key={p.at}
                className="grid grid-cols-[6rem_1fr_auto] items-center gap-3 text-[13px]"
              >
                <span className="tabular text-muted">{formatDate(p.at)}</span>
                <span aria-hidden="true" className="h-2 rounded-sm bg-subtle">
                  <span
                    className="block h-2 rounded-sm bg-fill"
                    style={{ width: `${(p.value / max) * 100}%` }}
                  />
                </span>
                <span className="tabular text-right">{formatNumber(p.value, 0)} m³</span>
              </li>
            ))}
          </ol>
        </section>
      ))}
    </div>
  );
}
