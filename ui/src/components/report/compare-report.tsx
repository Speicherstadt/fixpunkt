import { Section } from "@/components/page";
import type { CompareResult } from "@/lib/api";
import { cn } from "@/lib/cn";
import { formatVolume } from "@/lib/format";
import { binLabel, binSide } from "@/lib/report";
import { m } from "@/paraglide/messages";
import { Kpis } from "./kpis";

/** Zentimeter mit einer Stelle; −0,0 wird 0,0 */
export const cm = (v: number) =>
  `${(Math.abs(v) < 0.0005 ? 0 : v * 100).toFixed(1).replace(".", ",")} cm`;
const pct = (v: number) => `${Math.round(v)} %`;

const SIDE_CLASS = { low: "bg-fill", ok: "bg-ok", high: "bg-cut" } as const;

/** Soll/Ist: Anteil in Toleranz, Abweichungen, Verteilung, noch ab- und aufzutragende Massen */
export function CompareReport({ result }: { result: CompareResult }) {
  const d = result.deviation;
  const max = Math.max(...d.histogram.map((b) => b.percent), 1);
  return (
    <div className="flex flex-col gap-6">
      <Kpis
        items={[
          {
            label: m.compare_within({ tol: cm(d.tolerance) }),
            value: pct(d.within_tolerance_percent),
            className: "text-ok",
          },
          {
            label: m.compare_too_high(),
            value: pct(d.above_tolerance_percent),
            className: "text-cut",
          },
          {
            label: m.compare_too_low(),
            value: pct(d.below_tolerance_percent),
            className: "text-fill",
          },
          { label: m.compare_mean(), value: cm(d.mean) },
        ]}
      />
      <Section title={m.compare_distribution()} description={m.compare_distribution_text()}>
        <ul
          className="m-0 flex list-none flex-col gap-1.5 p-0"
          aria-label={m.compare_distribution()}
        >
          {d.histogram.map((b) => {
            const side = binSide(b.from, b.to, d.tolerance);
            return (
              <li
                key={`${b.from}-${b.to}`}
                className="grid grid-cols-[7rem_1fr_3.5rem] items-center gap-3 text-[13px]"
              >
                <span className="tabular text-right text-muted">{binLabel(b.from, b.to)}</span>
                <span className="h-3 rounded-sm bg-subtle">
                  <span
                    className={cn("block h-3 rounded-sm", SIDE_CLASS[side])}
                    style={{ width: `${(b.percent / max) * 100}%` }}
                  />
                </span>
                <span className="tabular text-right">{pct(b.percent)}</span>
              </li>
            );
          })}
        </ul>
        <p className="m-0 text-[12px] text-muted">{m.compare_axis()}</p>
      </Section>
      <Section title={m.compare_volumes()}>
        <Kpis
          items={[
            {
              label: m.compare_to_cut(),
              value: formatVolume(d.volume.fill_m3),
              className: "text-cut",
            },
            {
              label: m.compare_to_fill(),
              value: formatVolume(d.volume.cut_m3),
              className: "text-fill",
            },
            { label: m.compare_rms(), value: cm(d.rms) },
            {
              label: m.compare_range(),
              value: cm(Math.abs(d.max) >= Math.abs(d.min) ? d.max : d.min),
            },
          ]}
        />
      </Section>
    </div>
  );
}
