/**
 * Flugplanung: Bahnen über einem Abrechnungsbereich aus Bodenauflösung und Überlappung, Vorschau mit Kennzahlen,
 * Mission als KMZ für DJI Pilot 2 (Rechnung und WPML im Server, crates/processing/src/flightplan.rs).
 */
import { useQuery } from "@tanstack/react-query";
import { Download, Info, TriangleAlert } from "lucide-react";
import { useState } from "react";
import { EmptyState, ErrorText, Section } from "@/components/page";
import { Kpis } from "@/components/report/kpis";
import { buttonVariants } from "@/components/ui/button";
import { Input, Label, Select } from "@/components/ui/input";
import type { FlightPlan } from "@/lib/api";
import { errorText } from "@/lib/errors";
import {
  DEFAULT_FLIGHT,
  DRONES,
  type Drone,
  type FlightSettings,
  flightQuery,
  flightTime,
  gsdSpread,
  projector,
} from "@/lib/flight";
import { formatCount, formatDuration, formatNumber } from "@/lib/format";
import { flightPlanQuery, regionsQuery } from "@/lib/queries";
import { ringOf } from "@/lib/view/geometry";
import { m } from "@/paraglide/messages";

const NUMBERS = [
  ["gsd", () => m.flight_gsd()],
  ["front", () => m.flight_front()],
  ["side", () => m.flight_side()],
  ["direction", () => m.flight_direction()],
  ["speed", () => m.flight_speed()],
] as const;
type NumberKey = (typeof NUMBERS)[number][0];

const SIZE = 240;

export function FlightPlanning({ siteId }: { siteId: number }) {
  const { data: regions = [] } = useQuery(regionsQuery(siteId));
  const [picked, setPicked] = useState<number | undefined>();
  const region = regions.find((r) => r.id === picked) ?? regions[0];
  const [text, setText] = useState<Record<NumberKey, string>>(() => {
    const t = {} as Record<NumberKey, string>;
    for (const [k] of NUMBERS) t[k] = String(DEFAULT_FLIGHT[k]).replace(".", ",");
    return t;
  });
  const [drone, setDrone] = useState<Drone>(DEFAULT_FLIGHT.drone);
  const parsed = Object.fromEntries(
    NUMBERS.map(([k]) => [k, Number(text[k].replace(",", "."))]),
  ) as Record<NumberKey, number>;
  const valid = Object.values(parsed).every((v) => Number.isFinite(v)) && text.gsd.trim() !== "";
  const settings: FlightSettings = { ...parsed, drone };
  const query = flightQuery(settings);
  const plan = useQuery({
    ...flightPlanQuery(region?.id ?? 0, query),
    enabled: Boolean(region) && valid,
  });

  return (
    <Section title={m.flight_title()} description={m.flight_description()} collapsible>
      {region ? (
        <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,2fr)]">
          <form className="flex flex-col gap-4" onSubmit={(e) => e.preventDefault()}>
            <Label>
              {m.flight_region()}
              <Select value={region.id} onChange={(e) => setPicked(Number(e.target.value))}>
                {regions.map((r) => (
                  <option key={r.id} value={r.id}>
                    {r.name}
                  </option>
                ))}
              </Select>
            </Label>
            <Label>
              {m.flight_drone()}
              <Select value={drone} onChange={(e) => setDrone(e.target.value as Drone)}>
                {DRONES.map((d) => (
                  <option key={d} value={d}>
                    {d === "m4e" ? "DJI Matrice 4E" : "DJI Mavic 3 Enterprise"}
                  </option>
                ))}
              </Select>
            </Label>
            <div className="grid grid-cols-2 gap-4">
              {NUMBERS.map(([k, label]) => (
                <Label key={k}>
                  {label()}
                  <Input
                    inputMode="decimal"
                    value={text[k]}
                    onChange={(e) => setText({ ...text, [k]: e.target.value })}
                  />
                </Label>
              ))}
            </div>
          </form>
          <div className="flex flex-col gap-4">
            {plan.error ? <ErrorText>{errorText(plan.error)}</ErrorText> : null}
            {plan.data ? (
              <Result
                data={plan.data}
                ring={ringOf(region.geojson)}
                href={`/api/regions/${region.id}/flightplan.kmz?${query}`}
              />
            ) : null}
          </div>
        </div>
      ) : (
        <EmptyState>{m.flight_no_regions()}</EmptyState>
      )}
    </Section>
  );
}

function Result({
  data,
  ring,
  href,
}: {
  data: FlightPlan;
  ring: [number, number][];
  href: string;
}) {
  const p = data.plan;
  const t = data.terrain;
  const spread = t ? gsdSpread(t, p.altitude) : null;
  return (
    <>
      <Kpis
        items={[
          { label: m.flight_altitude(), value: `${formatNumber(p.altitude, 1)} m` },
          { label: m.flight_lines(), value: formatCount(p.lines.length) },
          { label: m.flight_photos(), value: formatCount(p.photos) },
          { label: m.flight_duration(), value: flightTime(p.duration, formatDuration) },
        ]}
      />
      <div className="flex flex-wrap items-start gap-6">
        <Preview ring={ring} lines={p.lines} region={data.region} />
        <div className="flex min-w-48 flex-1 flex-col gap-3 text-[13px]">
          <dl className="m-0 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
            <dt className="text-muted">{m.flight_line_spacing()}</dt>
            <dd className="tabular m-0">{formatNumber(p.lineSpacing, 1)} m</dd>
            <dt className="text-muted">{m.flight_photo_spacing()}</dt>
            <dd className="tabular m-0">{formatNumber(p.photoSpacing, 1)} m</dd>
            <dt className="text-muted">{m.flight_speed_used()}</dt>
            <dd className="tabular m-0">{formatNumber(p.speed, 1)} m/s</dd>
            <dt className="text-muted">{m.flight_length()}</dt>
            <dd className="tabular m-0">{formatNumber(p.length / 1000, 2)} km</dd>
          </dl>
          {t && spread ? (
            <p className="m-0 flex items-start gap-2">
              {spread.percent > 10 ? (
                <TriangleAlert size={16} aria-hidden="true" className="mt-0.5 shrink-0 text-warn" />
              ) : (
                <Info size={16} aria-hidden="true" className="mt-0.5 shrink-0 text-muted" />
              )}
              {m.flight_terrain({
                relief: formatNumber(spread.relief, 1),
                capture: t.capture,
                percent: formatNumber(spread.percent, 0),
              })}
            </p>
          ) : null}
          <p className="m-0 text-muted">{m.flight_start_hint()}</p>
          <a href={href} className={`${buttonVariants({ variant: "primary" })} self-start`}>
            <Download size={16} aria-hidden="true" />
            {m.flight_download()}
          </a>
        </div>
      </div>
    </>
  );
}

function Preview({
  ring,
  lines,
  region,
}: {
  ring: [number, number][];
  lines: FlightPlan["plan"]["lines"];
  region: string;
}) {
  const all = [...ring, ...lines.flatMap((l) => [l.start, l.end])];
  if (all.length < 2) return null;
  const at = projector(all, SIZE);
  const path = lines.flatMap((l) => [at(l.start), at(l.end)]);
  const first = path[0];
  return (
    <svg
      role="img"
      aria-label={m.flight_preview({ region, lines: lines.length })}
      width={SIZE}
      height={SIZE}
      viewBox={`0 0 ${SIZE} ${SIZE}`}
      className="shrink-0 rounded-lg bg-subtle"
    >
      <polygon
        points={ring.map((q) => at(q).join(",")).join(" ")}
        className="fill-cut/15 stroke-cut"
        style={{ strokeWidth: 1.5 }}
      />
      <polyline
        points={path.map((q) => q.join(",")).join(" ")}
        className="fill-none stroke-fg"
        style={{ strokeWidth: 1.25 }}
        strokeLinejoin="round"
      />
      {first ? <circle cx={first[0]} cy={first[1]} r={4} className="fill-fg" /> : null}
    </svg>
  );
}
