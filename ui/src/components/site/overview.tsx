/**
 * Überblick einer Baustelle: Aufnahmen, Aushub bisher, letzter Soll/Ist-Vergleich, Zwangspunkte – darunter der
 * Fortschritt je Befliegung und, was Aufmerksamkeit braucht (verletzte Zwangspunkte, fehlgeschlagene Aufträge).
 */
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { CircleAlert, TriangleAlert } from "lucide-react";
import { Section } from "@/components/page";
import { cm } from "@/components/report/compare-report";
import { Kpis } from "@/components/report/kpis";
import { formatDate, formatVolume } from "@/lib/format";
import { dashboardQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";

export function Overview({ siteId }: { siteId: number }) {
  const { data: d } = useQuery(dashboardQuery(siteId));
  if (!d || d.captures.count === 0) return null;
  const last = d.progress.at(-1);
  const max = Math.max(1, ...d.progress.map((p) => p.cut));
  const k = d.constraints;
  return (
    <Section title={m.overview_title()}>
      <Kpis
        items={[
          {
            label: m.overview_captures(),
            value: String(d.captures.count),
          },
          {
            label: m.overview_cut(),
            value: last ? formatVolume(last.cut) : "–",
            className: last ? "text-cut" : undefined,
          },
          {
            label: m.overview_compare(),
            value:
              d.latestCompare?.within != null ? `${Math.round(d.latestCompare.within)} %` : "–",
          },
          {
            label: m.overview_constraints(),
            value: k.total ? `${k.checked - k.violated.length} / ${k.total}` : "–",
            className: k.violated.length ? "text-warn" : k.total ? "text-ok" : undefined,
          },
        ]}
      />
      <p className="m-0 text-[12px] text-muted">
        {d.captures.last
          ? m.overview_last({ name: d.captures.last.name, date: formatDate(d.captures.last.at) })
          : null}
        {` · ${m.overview_more({
          photos: d.documentation.photos,
          notes: d.documentation.notes,
          plans: d.plans,
        })}`}
      </p>
      {d.progress.length ? (
        <div className="flex flex-col gap-2">
          <h3 className="m-0 text-[14px] font-semibold">{m.overview_progress()}</h3>
          <ol className="m-0 flex list-none flex-col gap-1.5 p-0">
            {d.progress.map((p) => (
              <li
                key={p.captureId}
                className="grid grid-cols-[minmax(0,10rem)_1fr_auto] items-center gap-3"
              >
                <Link
                  to="/jobs/$jobId"
                  params={{ jobId: String(p.jobId) }}
                  className="truncate text-[13px] hover:underline"
                >
                  {p.name} · {formatDate(p.at)}
                </Link>
                <span aria-hidden="true" className="h-3 rounded-sm bg-subtle">
                  <span
                    className="block h-3 rounded-sm bg-cut"
                    style={{ width: `${(p.cut / max) * 100}%` }}
                  />
                </span>
                <span className="tabular text-right text-[13px]">{formatVolume(p.cut)}</span>
              </li>
            ))}
          </ol>
        </div>
      ) : null}
      {k.violated.length || d.jobs.failed ? (
        <ul className="m-0 flex list-none flex-col gap-1.5 p-0" aria-label={m.overview_attention()}>
          {k.violated.map((v) => (
            <li key={v.id} className="flex items-center gap-2 text-[13px]">
              <TriangleAlert size={16} aria-hidden="true" className="shrink-0 text-warn" />
              {m.overview_violated({ name: v.title, diff: cm(v.diff), capture: k.against ?? "–" })}
            </li>
          ))}
          {d.jobs.failed ? (
            <li className="flex items-center gap-2 text-[13px]">
              <CircleAlert size={16} aria-hidden="true" className="shrink-0 text-danger" />
              {m.overview_failed({ count: d.jobs.failed })}
            </li>
          ) : null}
        </ul>
      ) : null}
    </Section>
  );
}
