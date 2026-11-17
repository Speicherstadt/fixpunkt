/**
 * 2D-Ansicht einer Baustelle: Höhenbild einer Aufnahme, wahlweise mit Differenz/Abweichung darüber, die Bereiche,
 * und Werkzeuge zum Messen und Zeichnen. Gerechnet wird im Browser (src/lib/view/*), gespeichert nur ein neuer Bereich.
 */
import { useQuery } from "@tanstack/react-query";
import { Eraser, Undo2 } from "lucide-react";
import { lazy, Suspense, useMemo, useState } from "react";
import { ErrorText, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Label, Select } from "@/components/ui/input";
import { PlaceholderRows } from "@/components/ui/placeholder";
import { Segmented } from "@/components/ui/segmented";
import { FilesResult, type Job } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { formatHeight } from "@/lib/format";
import { chronological, jobTitle } from "@/lib/jobs";
import { linesOf, pinsOf } from "@/lib/markers";
import {
  capturesQuery,
  jobsQuery,
  markersQuery,
  planGeometryQuery,
  plansQuery,
  regionsQuery,
  siteQuery,
} from "@/lib/queries";
import { deviationColor, deviationLimit, heightColor, terrainColor } from "@/lib/view/colors";
import { type Pt, ringOf } from "@/lib/view/geometry";
import { fetchGrid, robustRange, sampleAt } from "@/lib/view/grid";
import { buildMesh } from "@/lib/view/mesh";
import { type ColorBy, colorModes, fetchPoints, pointScene } from "@/lib/view/points";
import { m } from "@/paraglide/messages";
import { MapCanvas } from "./map-canvas";
import { MeasurePanel, type Tool } from "./measure-panel";
import { SaveShape } from "./save-shape";
import type { Content } from "./terrain-3d";

/** three.js nur laden, wenn jemand auf 3D schaltet */
const Terrain3D = lazy(() => import("./terrain-3d"));

const COLOR_TEXT: Record<ColorBy, () => string> = {
  file: m.view3d_color_file,
  intensity: m.view3d_color_intensity,
  height: m.view3d_color_height,
};

const TOOLS: { id: Tool; text: () => string }[] = [
  { id: "pan", text: m.tool_pan },
  { id: "point", text: m.tool_point },
  { id: "line", text: m.tool_line },
  { id: "area", text: m.tool_area },
  { id: "profile", text: m.tool_profile },
  { id: "draw", text: m.tool_draw },
  { id: "trace", text: m.tool_trace },
];

/** Ergebnis-Raster eines Auftrags, das sich überlagern lässt */
const overlayFile = (j: Job) => {
  const r = FilesResult.safeParse(j.result);
  return r.success
    ? r.data.files.find((f) => f === "diff.tif" || f === "abweichung.tif")
    : undefined;
};

const rgb = (c: [number, number, number]) => `rgb(${c.map(Math.round).join(" ")})`;

export function SiteView({ siteId, canEdit }: { siteId: number; canEdit: boolean }) {
  const captures = useQuery(capturesQuery(siteId));
  const regions = useQuery(regionsQuery(siteId));
  const jobs = useQuery(jobsQuery(siteId));
  const site = useQuery(siteQuery(siteId));
  const ready = chronological((captures.data ?? []).filter((c) => c.status === "ready"));
  const [picked, setCapture] = useState<number | undefined>();
  const captureId = picked ?? ready.at(-1)?.id;
  const overlays = (jobs.data ?? []).filter((j) => j.status === "done" && overlayFile(j));
  const [overlayId, setOverlay] = useState<number | null>(null);
  const overlayJob = overlays.find((j) => j.id === overlayId);
  const [tool, setTool] = useState<Tool>("pan");
  const [mode, setMode] = useState<"2d" | "3d">("2d");
  const [background, setBackground] = useState<"ortho" | "heights">("ortho");
  const [exaggeration, setExaggeration] = useState(2);
  const [points, setPoints] = useState<Pt[]>([]);
  const [hover, setHover] = useState<Pt | null>(null);

  const grid = useQuery({
    queryKey: ["grid", "capture", captureId],
    queryFn: () => fetchGrid(`/api/captures/${captureId}/grid`),
    enabled: captureId !== undefined,
    staleTime: Number.POSITIVE_INFINITY,
  });
  const overlay = useQuery({
    queryKey: ["grid", "job", overlayJob?.id],
    queryFn: () => fetchGrid(`/api/jobs/${overlayJob!.id}/grid/${overlayFile(overlayJob!)}`),
    enabled: overlayJob !== undefined,
    staleTime: Number.POSITIVE_INFINITY,
  });
  const range = useMemo(() => (grid.data ? robustRange(grid.data) : null), [grid.data]);
  const limit = useMemo(() => (overlay.data ? deviationLimit(overlay.data) : 1), [overlay.data]);
  const overlayProp = useMemo(
    () =>
      overlay.data && overlayJob
        ? {
            grid: overlay.data,
            limit,
            removedIsNegative: overlayFile(overlayJob) === "diff.tif",
          }
        : null,
    [overlay.data, overlayJob, limit],
  );
  const [shape3d, setShape3d] = useState<"mesh" | "points">("mesh");
  const isCloud = ready.find((c) => c.id === captureId)?.info?.type === "pointcloud";
  const showPoints = mode === "3d" && shape3d === "points" && isCloud;
  const cloud = useQuery({
    queryKey: ["points", captureId],
    queryFn: () => fetchPoints(`/api/captures/${captureId}/points`),
    enabled: showPoints && captureId !== undefined,
    staleTime: Number.POSITIVE_INFINITY,
  });
  const modes = cloud.data ? colorModes(cloud.data) : [];
  const [pickedColor, setColorBy] = useState<ColorBy | null>(null);
  const colorBy = pickedColor && modes.includes(pickedColor) ? pickedColor : modes[0];
  const content = useMemo((): Content | null => {
    if (mode !== "3d") return null;
    if (showPoints)
      return cloud.data
        ? { kind: "points", cloud: pointScene(cloud.data, exaggeration, colorBy) }
        : null;
    return grid.data && range
      ? {
          kind: "mesh",
          mesh: buildMesh(grid.data, terrainColor(range, overlayProp), exaggeration),
        }
      : null;
  }, [mode, showPoints, cloud.data, grid.data, range, overlayProp, exaggeration, colorBy]);
  const capture = ready.find((c) => c.id === captureId);
  const orthoMeta = capture?.ortho ?? null;
  const orthoImage = useQuery({
    queryKey: ["ortho", captureId, orthoMeta?.width, orthoMeta?.corners[0][0]],
    queryFn: async () => {
      const img = new Image();
      img.src = `/api/captures/${captureId}/ortho`;
      await img.decode();
      return img;
    },
    enabled: orthoMeta !== null && mode === "2d" && background === "ortho",
    staleTime: Number.POSITIVE_INFINITY,
  });
  const orthoProp = useMemo(
    () =>
      orthoMeta && orthoImage.data && background === "ortho"
        ? { image: orthoImage.data, ...orthoMeta }
        : null,
    [orthoMeta, orthoImage.data, background],
  );
  const markers = useQuery(markersQuery(siteId));
  const plans = useQuery(plansQuery(siteId));
  const [planId, setPlanId] = useState<number | null>(null);
  const planGeometry = useQuery({ ...planGeometryQuery(planId ?? 0), enabled: planId !== null });
  const pins = useMemo(() => pinsOf(markers.data ?? []), [markers.data]);
  const lines = useMemo(() => linesOf(markers.data ?? []), [markers.data]);
  const outlines = useMemo(
    () => (regions.data ?? []).map((r) => ({ name: r.name, points: ringOf(r.geojson) })),
    [regions.data],
  );

  if (!ready.length) return null;
  const chooseTool = (t: Tool) => {
    setTool(t);
    setPoints([]);
  };
  const names = (id: unknown) => captures.data?.find((c) => c.id === id)?.name ?? "–";
  const hoverZ = hover && grid.data ? sampleAt(grid.data, hover[0], hover[1]) : null;

  return (
    <Section title={m.view_title()} description={m.view_text()}>
      <div className="grid gap-4 md:grid-cols-2">
        <Label>
          {m.calc_capture()}
          <Select value={captureId ?? ""} onChange={(e) => setCapture(Number(e.target.value))}>
            {ready.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </Select>
        </Label>
        <Label>
          {m.view_overlay()}
          <Select
            value={overlayId ?? ""}
            onChange={(e) => setOverlay(e.target.value ? Number(e.target.value) : null)}
          >
            <option value="">{m.view_overlay_none()}</option>
            {overlays.map((j) => (
              <option key={j.id} value={j.id}>
                {jobTitle(j, names)}
              </option>
            ))}
          </Select>
        </Label>
      </div>
      {plans.data?.length ? (
        <Label className="max-w-sm">
          {m.view_plan()}
          <Select
            value={planId ?? ""}
            onChange={(e) => setPlanId(e.target.value ? Number(e.target.value) : null)}
          >
            <option value="">{m.view_plan_none()}</option>
            {plans.data.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </Select>
        </Label>
      ) : null}
      <div className="flex flex-wrap items-center gap-3">
        <Segmented
          name="mode"
          legend={m.view3d_mode()}
          options={[
            { id: "2d", text: m.view3d_2d() },
            { id: "3d", text: m.view3d_3d() },
          ]}
          value={mode}
          onChange={setMode}
        />
        {mode === "2d" && orthoMeta ? (
          <Segmented
            name="background"
            legend={m.view_background()}
            options={[
              { id: "ortho", text: m.view_background_ortho() },
              { id: "heights", text: m.view_background_heights() },
            ]}
            value={background}
            onChange={setBackground}
          />
        ) : null}
        {mode === "2d" ? (
          <Segmented
            name="tool"
            legend={m.view_tools()}
            options={TOOLS.filter((t) => canEdit || (t.id !== "draw" && t.id !== "trace")).map(
              (t) => ({
                id: t.id,
                text: t.text(),
              }),
            )}
            value={tool}
            onChange={chooseTool}
          />
        ) : (
          <>
            {isCloud ? (
              <Segmented
                name="shape3d"
                legend={m.view3d_shape()}
                options={[
                  { id: "mesh", text: m.view3d_mesh() },
                  { id: "points", text: m.view3d_points() },
                ]}
                value={shape3d}
                onChange={setShape3d}
              />
            ) : null}
            {showPoints && modes.length > 1 && colorBy ? (
              <Segmented
                name="colorBy"
                legend={m.view3d_color()}
                options={modes.map((id) => ({ id, text: COLOR_TEXT[id]() }))}
                value={colorBy}
                onChange={setColorBy}
              />
            ) : null}
            <Label className="flex-row items-center gap-2">
              {m.view3d_exaggeration()}
              <Select
                className="h-8 w-24"
                value={String(exaggeration)}
                onChange={(e) => setExaggeration(Number(e.target.value))}
              >
                {[1, 2, 5].map((x) => (
                  <option key={x} value={x}>
                    {x} ×
                  </option>
                ))}
              </Select>
            </Label>
          </>
        )}
      </div>
      {grid.error ? <ErrorText>{errorText(grid.error)}</ErrorText> : null}
      {grid.data && range && mode === "3d" ? (
        <Suspense fallback={<PlaceholderRows count={2} />}>
          {content ? <Terrain3D content={content} label={m.view3d_label()} /> : null}
        </Suspense>
      ) : null}
      {grid.data && range && mode === "2d" ? (
        <>
          <MapCanvas
            grid={grid.data}
            range={range}
            overlay={overlayProp}
            outlines={outlines}
            lines={lines}
            pins={pins}
            plan={planId !== null ? (planGeometry.data ?? null) : null}
            ortho={orthoProp}
            shape={{ points, closed: tool === "area" || tool === "draw" }}
            picking={tool !== "pan"}
            onPick={(p) => setPoints((ps) => (tool === "point" ? [p] : [...ps, p]))}
            onHover={setHover}
            label={m.view_canvas_label()}
          />
          <div className="flex flex-wrap items-center justify-between gap-3 text-[12px] text-muted">
            <span className="tabular" aria-live="off">
              {hover
                ? m.view_cursor({
                    e: hover[0].toFixed(2).replace(".", ","),
                    n: hover[1].toFixed(2).replace(".", ","),
                    h: hoverZ === null ? "–" : formatHeight(hoverZ),
                  })
                : m.view_keys()}
            </span>
            <span className="flex items-center gap-2">
              <span className="tabular">{formatHeight(range[0])}</span>
              <span
                aria-hidden="true"
                className="block h-2 w-24 rounded-sm"
                style={{
                  background: `linear-gradient(to right, ${[0, 0.25, 0.5, 0.75, 1].map((t) => rgb(heightColor(t))).join(", ")})`,
                }}
              />
              <span className="tabular">{formatHeight(range[1])}</span>
            </span>
            {overlay.data ? (
              <span className="flex items-center gap-2">
                <span>{m.view_legend_fill()}</span>
                <span
                  aria-hidden="true"
                  className="block h-2 w-24 rounded-sm"
                  style={{
                    background: `linear-gradient(to right, ${[-1, -0.5, 0, 0.5, 1].map((t) => rgb(deviationColor(t, 1))).join(", ")})`,
                  }}
                />
                <span>{m.view_legend_cut({ cm: (limit * 100).toFixed(0) })}</span>
              </span>
            ) : null}
          </div>
          {tool !== "pan" ? (
            <div className="flex flex-col gap-3">
              <MeasurePanel tool={tool} grid={grid.data} points={points} />
              <div className="flex flex-wrap items-end gap-2">
                <Button
                  size="sm"
                  onClick={() => setPoints((ps) => ps.slice(0, -1))}
                  disabled={!points.length}
                >
                  <Undo2 size={14} aria-hidden="true" />
                  {m.view_undo()}
                </Button>
                <Button size="sm" onClick={() => setPoints([])} disabled={!points.length}>
                  <Eraser size={14} aria-hidden="true" />
                  {m.view_clear()}
                </Button>
                {tool === "draw" || tool === "trace" ? (
                  <SaveShape
                    key={tool}
                    kind={tool === "draw" ? "region" : "line"}
                    siteId={siteId}
                    epsg={site.data?.epsg ?? 25832}
                    grid={grid.data}
                    points={points}
                    onSaved={() => setPoints([])}
                  />
                ) : null}
              </div>
            </div>
          ) : null}
        </>
      ) : null}
      {grid.isPending ? (
        <p role="status" className="m-0 text-[13px] text-muted">
          {m.view_loading()}
        </p>
      ) : null}
    </Section>
  );
}
