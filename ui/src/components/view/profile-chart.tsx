import { formatHeight } from "@/lib/format";
import type { ProfilePoint } from "@/lib/view/geometry";
import { m } from "@/paraglide/messages";

/** Längsschnitt als SVG: Station (m) nach rechts, Höhe nach oben; Lücken unterbrechen die Linie */
export function ProfileChart({ points }: { points: ProfilePoint[] }) {
  const zs = points.map((p) => p.z).filter((z): z is number => z !== null);
  if (zs.length < 2) return null;
  const w = 600;
  const h = 160;
  const pad = { l: 56, r: 12, t: 10, b: 24 };
  const lo = Math.min(...zs);
  const hi = Math.max(...zs);
  const span = Math.max(hi - lo, 0.1);
  const len = points.at(-1)!.station || 1;
  const x = (s: number) => pad.l + (s / len) * (w - pad.l - pad.r);
  const y = (z: number) => pad.t + (1 - (z - lo) / span) * (h - pad.t - pad.b);
  const segments: string[] = [];
  let current = "";
  for (const p of points) {
    if (p.z === null) {
      if (current) segments.push(current);
      current = "";
      continue;
    }
    current += `${current ? "L" : "M"}${x(p.station).toFixed(1)},${y(p.z).toFixed(1)}`;
  }
  if (current) segments.push(current);
  return (
    <figure className="m-0 flex flex-col gap-1">
      <svg
        viewBox={`0 0 ${w} ${h}`}
        className="h-auto w-full max-w-[600px]"
        role="img"
        aria-label={m.view_profile_label({
          from: formatHeight(lo),
          to: formatHeight(hi),
          length: Math.round(len),
        })}
      >
        <line x1={pad.l} x2={w - pad.r} y1={h - pad.b} y2={h - pad.b} className="stroke-border" />
        <line x1={pad.l} x2={pad.l} y1={pad.t} y2={h - pad.b} className="stroke-border" />
        <text x={pad.l - 6} y={y(hi) + 4} textAnchor="end" className="fill-muted text-[12px]">
          {hi.toFixed(2).replace(".", ",")}
        </text>
        <text x={pad.l - 6} y={y(lo)} textAnchor="end" className="fill-muted text-[12px]">
          {lo.toFixed(2).replace(".", ",")}
        </text>
        <text
          x={w - pad.r}
          y={h - 6}
          textAnchor="end"
          className="fill-muted text-[12px]"
        >{`${len.toFixed(1).replace(".", ",")} m`}</text>
        {segments.map((d) => (
          <path
            key={d}
            d={d}
            className="fill-none stroke-accent"
            strokeLinejoin="round"
            style={{ strokeWidth: 2 }}
          />
        ))}
      </svg>
      <figcaption className="text-[12px] text-muted">{m.view_profile_caption()}</figcaption>
    </figure>
  );
}
