import { formatArea, formatHeight } from "@/lib/format";
import { measureArea, measureLine, type Pt, profile } from "@/lib/view/geometry";
import { type HeightGrid, sampleAt } from "@/lib/view/grid";
import { m } from "@/paraglide/messages";
import { ProfileChart } from "./profile-chart";

export type Tool = "pan" | "point" | "line" | "area" | "profile" | "draw";

const metres = (v: number) => `${v.toFixed(2).replace(".", ",")} m`;
const coord = (v: number) => v.toFixed(2).replace(".", ",");

/** Ergebnisse der Messung als Liste (vorgelesen, sobald sie sich ändern) */
export function MeasurePanel({
  tool,
  grid,
  points,
}: {
  tool: Tool;
  grid: HeightGrid;
  points: Pt[];
}) {
  const rows = results(tool, grid, points);
  return (
    <div className="flex flex-col gap-3">
      <dl
        aria-live="polite"
        className="m-0 grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 text-[14px] sm:grid-cols-[auto_1fr_auto_1fr]"
      >
        {rows.length ? (
          rows.map(([k, v]) => (
            <div key={k} className="contents">
              <dt className="text-muted">{k}</dt>
              <dd className="tabular m-0">{v}</dd>
            </div>
          ))
        ) : (
          <dd className="m-0 text-[13px] text-muted">{m.view_pick_hint()}</dd>
        )}
      </dl>
      {tool === "profile" && points.length > 1 ? (
        <ProfileChart points={profile(grid, points, grid.cell)} />
      ) : null}
    </div>
  );
}

export const results = (tool: Tool, grid: HeightGrid, points: Pt[]): [string, string][] => {
  if (tool === "point" && points.length) {
    const [x, y] = points[0]!;
    const z = sampleAt(grid, x, y);
    return [
      [m.measure_east(), coord(x)],
      [m.measure_north(), coord(y)],
      [m.measure_height(), z === null ? "–" : formatHeight(z)],
    ];
  }
  if ((tool === "line" || tool === "profile") && points.length > 1) {
    const l = measureLine(grid, points);
    return [
      [m.measure_length2d(), metres(l.length2d)],
      [m.measure_length3d(), metres(l.length3d)],
      [m.measure_dz(), l.dz === null ? "–" : metres(l.dz)],
      [m.measure_slope(), l.slope === null ? "–" : `${l.slope.toFixed(2).replace(".", ",")} %`],
    ];
  }
  if ((tool === "area" || tool === "draw") && points.length > 2) {
    const a = measureArea(grid, points);
    return [
      [m.measure_area2d(), formatArea(a.area2d)],
      [m.measure_area3d(), formatArea(a.area3d)],
      [m.measure_perimeter(), metres(a.perimeter)],
      [
        m.measure_zrange(),
        a.zMin === null ? "–" : `${formatHeight(a.zMin)} … ${formatHeight(a.zMax!)}`,
      ],
    ];
  }
  return [];
};
