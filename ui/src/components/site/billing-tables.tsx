/** Tabellen des Aufmaßes: Positionen mit Mengen in allen Einheiten und Betrag; Abgleich mit Lieferscheinen */
import type { Statement } from "@/lib/api";
import { DEVIATION_WARN, deliveredText, priceUnitLabel, signedPercent } from "@/lib/billing";
import { cn } from "@/lib/cn";
import { formatEuro, formatNumber } from "@/lib/format";
import { m } from "@/paraglide/messages";

const num = (v: number | null | undefined, digits = 1) =>
  v == null ? "–" : formatNumber(v, digits);

const th = "py-1.5 pr-3 font-medium";
const td = "tabular py-1.5 pr-3 text-right";

export function PositionsTable({ statement: s }: { statement: Statement }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[13px]" aria-label={m.billing_positions()}>
        <thead>
          <tr className="border-b border-border text-left text-muted">
            <th className={th}>{m.billing_position()}</th>
            <th className={th}>{m.billing_region()}</th>
            <th className={cn(th, "text-right")}>{m.billing_bank()}</th>
            <th className={cn(th, "text-right")}>{m.billing_unit_m3_loose()}</th>
            <th className={cn(th, "text-right")}>{m.billing_unit_t()}</th>
            <th className={cn(th, "text-right")}>{m.billing_loads()}</th>
            <th className={cn(th, "text-right")}>{m.billing_unit_price()}</th>
            <th className={cn(th, "pr-0 text-right")}>{m.billing_amount()}</th>
          </tr>
        </thead>
        <tbody>
          {s.positions.map((p) => (
            <tr key={p.regionId} className="border-b border-border">
              <td className="py-1.5 pr-3 whitespace-nowrap">{p.lvPosition ?? "–"}</td>
              <td className="py-1.5 pr-3">
                <span className="flex flex-col">
                  <span>{p.name}</span>
                  {p.material ? <span className="text-[12px] text-muted">{p.material}</span> : null}
                </span>
              </td>
              <td className={td}>{num(p.period?.bankM3)}</td>
              <td className={td}>{num(p.period?.looseM3)}</td>
              <td className={td}>{num(p.period?.tonnes)}</td>
              <td className={td}>{num(p.period?.loads, 0)}</td>
              <td className={td}>
                {p.unitPrice != null && p.unit
                  ? `${formatEuro(p.unitPrice)}/${priceUnitLabel(p.unit)}`
                  : "–"}
              </td>
              <td className={cn(td, "pr-0")}>{p.amount == null ? "–" : formatEuro(p.amount)}</td>
            </tr>
          ))}
        </tbody>
        <tfoot>
          <tr className="font-semibold">
            <td className="py-1.5 pr-3" colSpan={7}>
              {m.billing_total()}
            </td>
            <td className={cn(td, "pr-0")}>{s.amount == null ? "–" : formatEuro(s.amount)}</td>
          </tr>
        </tfoot>
      </table>
    </div>
  );
}

export function ChecksTable({ checks }: { checks: Statement["checks"] }) {
  return (
    <div className="flex flex-col gap-2">
      <h3 className="m-0 text-[14px] font-semibold">{m.billing_checks()}</h3>
      <p className="m-0 text-[13px] text-muted">{m.billing_checks_text()}</p>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[13px]" aria-label={m.billing_checks()}>
          <thead>
            <tr className="border-b border-border text-left text-muted">
              <th className={th}>{m.target_material()}</th>
              <th className={th}>{m.billing_delivered()}</th>
              <th className={cn(th, "text-right")}>{m.billing_delivered_bank()}</th>
              <th className={cn(th, "text-right")}>{m.billing_measured()}</th>
              <th className={cn(th, "pr-0 text-right")}>{m.billing_deviation()}</th>
            </tr>
          </thead>
          <tbody>
            {checks.map((c) => {
              const off =
                c.deviationPercent != null && Math.abs(c.deviationPercent) > DEVIATION_WARN;
              return (
                <tr key={c.material ?? ""} className="border-b border-border">
                  <td className="py-1.5 pr-3">
                    <span className="flex flex-col">
                      <span>{c.material ?? m.billing_no_material()}</span>
                      <span className="text-[12px] text-muted">
                        {m.billing_check_count({ count: c.count })}
                      </span>
                    </span>
                  </td>
                  <td className="tabular py-1.5 pr-3">{deliveredText(c.delivered)}</td>
                  <td className={td}>{num(c.deliveredBank)}</td>
                  <td className={td}>{num(c.measuredBank)}</td>
                  <td className={cn(td, "pr-0", off && "font-semibold text-warn")}>
                    {c.deviationPercent == null ? "–" : signedPercent(c.deviationPercent)}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
