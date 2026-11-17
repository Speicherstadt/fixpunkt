/**
 * Ein Formular je Berechnungsart. Jedes baut nur die Parameter; abschicken und Fehler zeigen macht
 * `Calculations` (calculations.tsx).
 */
import { type FormEvent, type ReactNode, useState } from "react";
import { Input, Label, Select } from "@/components/ui/input";
import type { Capture, Region } from "@/lib/api";
import { chronological } from "@/lib/jobs";
import { m } from "@/paraglide/messages";
import {
  CaptureSelect,
  RegionChecks,
  RegionSelect,
  SurfaceFields,
  type SurfaceInput,
} from "./fields";

export type FormProps = {
  ready: Capture[];
  regions: Region[];
  /** Parameter fertig: Auftrag anlegen */
  onSubmit: (params: Record<string, unknown>) => void;
  /** Knopfzeile mit Fehler und Absenden */
  footer: (disabledReason: string | null) => ReactNode;
};

const defaultSurface: SurfaceInput = { cell: 0.25, classifyGround: true };

/** Vorauswahl: älteste Aufnahme als Urgelände, jüngste als Stand nach dem Aushub */
export const defaultPair = (ready: Capture[]): [number | undefined, number | undefined] => {
  const sorted = chronological(ready);
  return [sorted[0]?.id, sorted.length > 1 ? sorted[sorted.length - 1]?.id : undefined];
};

const latest = (ready: Capture[]) => chronological(ready).at(-1)?.id;

const grid = "grid gap-4 md:grid-cols-2";

const submitWith =
  (onSubmit: FormProps["onSubmit"], build: () => Record<string, unknown>) => (e: FormEvent) => {
    e.preventDefault();
    onSubmit(build());
  };

export function VolumeForm({ ready, regions, onSubmit, footer }: FormProps) {
  const [first, last] = defaultPair(ready);
  const [before, setBefore] = useState(first);
  const [after, setAfter] = useState(last);
  const [surface, setSurface] = useState(defaultSurface);
  // null: alle Bereiche, auch wenn sie erst nach dem Formular geladen sind
  const [picked, setRegionIds] = useState<number[] | null>(null);
  const regionIds = picked ?? regions.map((r) => r.id);
  const b = before ?? first;
  const a = after ?? last;
  return (
    <form
      onSubmit={submitWith(onSubmit, () => ({ before: b, after: a, regionIds, surface }))}
      className="flex flex-col gap-4"
    >
      <div className={grid}>
        <CaptureSelect label={m.calc_before()} captures={ready} value={b} onChange={setBefore} />
        <CaptureSelect label={m.calc_after()} captures={ready} value={a} onChange={setAfter} />
        <SurfaceFields value={surface} onChange={setSurface} />
      </div>
      <RegionChecks
        regions={regions}
        value={regionIds}
        onChange={setRegionIds}
        emptyText={m.calc_no_regions()}
      />
      {footer(ready.length < 2 ? m.calc_need_two() : b === a ? m.calc_same() : null)}
    </form>
  );
}

export function StockpileForm({ ready, regions, onSubmit, footer }: FormProps) {
  const [capture, setCapture] = useState(latest(ready));
  const [surface, setSurface] = useState({ ...defaultSurface, classifyGround: false });
  const [picked, setRegionIds] = useState<number[] | null>(null);
  const regionIds = picked ?? regions.map((r) => r.id);
  const [base, setBase] = useState<"plane" | "lowest" | "height">("plane");
  const [height, setHeight] = useState("");
  const c = capture ?? latest(ready);
  return (
    <form
      onSubmit={submitWith(onSubmit, () => ({
        captureId: c,
        regionIds,
        base: base === "height" ? Number(height.replace(",", ".")) : base,
        surface,
      }))}
      className="flex flex-col gap-4"
    >
      <div className={grid}>
        <CaptureSelect label={m.calc_capture()} captures={ready} value={c} onChange={setCapture} />
        <Label>
          {m.calc_base()}
          <Select value={base} onChange={(e) => setBase(e.target.value as typeof base)}>
            <option value="plane">{m.calc_base_plane()}</option>
            <option value="lowest">{m.calc_base_lowest()}</option>
            <option value="height">{m.calc_base_height()}</option>
          </Select>
        </Label>
        {base === "height" ? (
          <Label>
            {m.calc_base_height_value()}
            <Input
              inputMode="decimal"
              value={height}
              onChange={(e) => setHeight(e.target.value)}
              required
            />
          </Label>
        ) : null}
        <SurfaceFields value={surface} onChange={setSurface} />
      </div>
      <p className="m-0 max-w-prose text-[13px] text-muted">{m.calc_stockpile_hint()}</p>
      <RegionChecks
        regions={regions}
        value={regionIds}
        onChange={setRegionIds}
        emptyText={m.calc_need_region()}
      />
      {footer(
        ready.length < 1 ? m.calc_need_one() : regionIds.length === 0 ? m.calc_need_region() : null,
      )}
    </form>
  );
}

export function CompareForm({ ready, regions, onSubmit, footer }: FormProps) {
  const [capture, setCapture] = useState(latest(ready));
  const [type, setType] = useState<"height" | "slope" | "surface">("height");
  const [height, setHeight] = useState("");
  const [slope, setSlope] = useState({ x: "", y: "", z: "", percent: "", azimuth: "" });
  const [designCapture, setDesignCapture] = useState<number | undefined>();
  const [pickedRegion, setRegionId] = useState<number | null | undefined>(undefined);
  const regionId = pickedRegion === undefined ? (regions[0]?.id ?? null) : pickedRegion;
  const [tolerance, setTolerance] = useState("3");
  const [surface, setSurface] = useState(defaultSurface);
  const c = capture ?? latest(ready);
  const num = (v: string) => Number(v.replace(",", "."));
  const others = ready.filter((r) => r.id !== c);
  const design = () => {
    if (type === "height") return { type, height: num(height) };
    if (type === "slope")
      return {
        type,
        x: num(slope.x),
        y: num(slope.y),
        z: num(slope.z),
        percent: num(slope.percent),
        azimuth: num(slope.azimuth),
      };
    return { type, captureId: designCapture ?? others[0]?.id };
  };
  return (
    <form
      onSubmit={submitWith(onSubmit, () => ({
        captureId: c,
        design: design(),
        regionId,
        tolerance: num(tolerance) / 100,
        surface,
      }))}
      className="flex flex-col gap-4"
    >
      <div className={grid}>
        <CaptureSelect label={m.calc_actual()} captures={ready} value={c} onChange={setCapture} />
        <Label>
          {m.calc_design()}
          <Select value={type} onChange={(e) => setType(e.target.value as typeof type)}>
            <option value="height">{m.calc_design_height()}</option>
            <option value="slope">{m.calc_design_slope()}</option>
            <option value="surface">{m.calc_design_surface()}</option>
          </Select>
        </Label>
        {type === "height" ? (
          <Label>
            {m.calc_design_height_value()}
            <Input
              inputMode="decimal"
              value={height}
              onChange={(e) => setHeight(e.target.value)}
              required
            />
          </Label>
        ) : null}
        {type === "surface" ? (
          <CaptureSelect
            label={m.calc_design_surface()}
            captures={others}
            value={designCapture ?? others[0]?.id}
            onChange={setDesignCapture}
          />
        ) : null}
        <RegionSelect regions={regions} value={regionId} onChange={setRegionId} />
        <Label>
          {m.calc_tolerance()}
          <Input
            inputMode="decimal"
            value={tolerance}
            onChange={(e) => setTolerance(e.target.value)}
            required
          />
        </Label>
        <SurfaceFields value={surface} onChange={setSurface} />
      </div>
      {type === "slope" ? (
        <fieldset className="m-0 grid gap-4 border-0 p-0 md:grid-cols-3">
          <legend className="mb-2 p-0 text-[13px] text-muted">
            {m.calc_design_slope_legend()}
          </legend>
          {(
            [
              ["x", m.calc_slope_x()],
              ["y", m.calc_slope_y()],
              ["z", m.calc_slope_z()],
              ["percent", m.calc_slope_percent()],
              ["azimuth", m.calc_slope_azimuth()],
            ] as const
          ).map(([key, label]) => (
            <Label key={key}>
              {label}
              <Input
                inputMode="decimal"
                value={slope[key]}
                onChange={(e) => setSlope({ ...slope, [key]: e.target.value })}
                required
              />
            </Label>
          ))}
        </fieldset>
      ) : null}
      {footer(
        ready.length < 1
          ? m.calc_need_one()
          : type === "surface" && others.length === 0
            ? m.calc_need_two()
            : null,
      )}
    </form>
  );
}

export function SeriesForm({ ready, regions, onSubmit, footer }: FormProps) {
  const ordered = chronological(ready);
  const [pickedIds, setIds] = useState<number[] | null>(null);
  const ids = pickedIds ?? ordered.map((c) => c.id);
  const [pickedRegion, setRegionId] = useState<number | null | undefined>(undefined);
  const regionId = pickedRegion === undefined ? (regions[0]?.id ?? null) : pickedRegion;
  const [surface, setSurface] = useState(defaultSurface);
  const chosen = ordered.filter((c) => ids.includes(c.id)).map((c) => c.id);
  return (
    <form
      onSubmit={submitWith(onSubmit, () => ({ captureIds: chosen, regionId, surface }))}
      className="flex flex-col gap-4"
    >
      <fieldset className="m-0 flex flex-col gap-2 border-0 p-0">
        <legend className="mb-2 p-0 text-[13px] text-muted">{m.calc_series_captures()}</legend>
        {ordered.map((c) => (
          <label key={c.id} className="flex items-center gap-2 text-[14px]">
            <input
              type="checkbox"
              className="size-5 accent-fg"
              checked={ids.includes(c.id)}
              onChange={() =>
                setIds(ids.includes(c.id) ? ids.filter((x) => x !== c.id) : [...ids, c.id])
              }
            />
            {c.name}
          </label>
        ))}
      </fieldset>
      <div className={grid}>
        <RegionSelect regions={regions} value={regionId} onChange={setRegionId} />
        <SurfaceFields value={surface} onChange={setSurface} />
      </div>
      {footer(chosen.length < 2 ? m.calc_need_two() : null)}
    </form>
  );
}

export const EXPORT_FORMATS = ["tif", "dxf", "landxml", "csv", "obj", "ortho"] as const;

export function ExportForm({ ready, regions, onSubmit, footer }: FormProps) {
  const [capture, setCapture] = useState(latest(ready));
  const [formats, setFormats] = useState<string[]>(["tif", "dxf", "landxml"]);
  const [regionIds, setRegionIds] = useState<number[]>([]);
  const [interval, setContour] = useState("0.5");
  const [surface, setSurface] = useState({ ...defaultSurface, cell: 0.5 });
  const c = capture ?? latest(ready);
  const text: Record<(typeof EXPORT_FORMATS)[number], () => string> = {
    tif: m.export_tif,
    dxf: m.export_dxf,
    landxml: m.export_landxml,
    csv: m.export_csv,
    obj: m.export_obj,
    ortho: m.export_ortho,
  };
  const hasOrtho = Boolean(ready.find((r) => r.id === c)?.ortho);
  // ohne Orthofoto an der Aufnahme nicht wählbar (und nicht mitgeschickt)
  const chosen = hasOrtho ? formats : formats.filter((f) => f !== "ortho");
  return (
    <form
      onSubmit={submitWith(onSubmit, () => ({
        captureId: c,
        formats: chosen,
        regionIds,
        contourInterval: Number(interval),
        surface,
      }))}
      className="flex flex-col gap-4"
    >
      <div className={grid}>
        <CaptureSelect label={m.calc_capture()} captures={ready} value={c} onChange={setCapture} />
        <Label>
          {m.calc_contours()}
          <Select value={interval} onChange={(e) => setContour(e.target.value)}>
            {["0.1", "0.25", "0.5", "1", "2"].map((v) => (
              <option key={v} value={v}>
                {m.calc_cell_option({ cm: Math.round(Number(v) * 100) })}
              </option>
            ))}
          </Select>
        </Label>
        <SurfaceFields value={surface} onChange={setSurface} />
      </div>
      <fieldset className="m-0 flex flex-col gap-2 border-0 p-0">
        <legend className="mb-2 p-0 text-[13px] text-muted">{m.calc_formats()}</legend>
        {EXPORT_FORMATS.map((f) => (
          <label key={f} className="flex items-center gap-2 text-[14px]">
            <input
              type="checkbox"
              className="size-5 accent-fg"
              checked={chosen.includes(f)}
              disabled={f === "ortho" && !hasOrtho}
              onChange={() =>
                setFormats(formats.includes(f) ? formats.filter((x) => x !== f) : [...formats, f])
              }
            />
            {text[f]()}
          </label>
        ))}
      </fieldset>
      <RegionChecks
        regions={regions}
        value={regionIds}
        onChange={setRegionIds}
        emptyText={m.calc_no_regions_export()}
      />
      {footer(
        ready.length < 1 ? m.calc_need_one() : chosen.length === 0 ? m.calc_need_format() : null,
      )}
    </form>
  );
}
