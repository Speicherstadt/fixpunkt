/** Felder, die mehrere Berechnungsarten teilen */
import { Label, Select } from "@/components/ui/input";
import type { Capture, Region } from "@/lib/api";
import { m } from "@/paraglide/messages";

export const CELL_SIZES = [0.1, 0.25, 0.5, 1] as const;

export type SurfaceInput = { cell: number; classifyGround: boolean };

export const CaptureSelect = ({
  label,
  captures,
  value,
  onChange,
}: {
  label: string;
  captures: Capture[];
  value: number | undefined;
  onChange: (id: number) => void;
}) => (
  <Label>
    {label}
    <Select value={value ?? ""} onChange={(e) => onChange(Number(e.target.value))}>
      {captures.map((c) => (
        <option key={c.id} value={c.id}>
          {c.name}
        </option>
      ))}
    </Select>
  </Label>
);

/** Rasterweite und Bodenfilter */
export const SurfaceFields = ({
  value,
  onChange,
}: {
  value: SurfaceInput;
  onChange: (v: SurfaceInput) => void;
}) => (
  <>
    <Label>
      {m.calc_cell()}
      <Select value={String(value.cell)} onChange={(e) => onChange({ ...value, cell: Number(e.target.value) })}>
        {CELL_SIZES.map((s) => (
          <option key={s} value={String(s)}>
            {m.calc_cell_option({ cm: Math.round(s * 100) })}
          </option>
        ))}
      </Select>
    </Label>
    <label className="flex items-center gap-2 self-end pb-2.5 text-[14px]">
      <input
        type="checkbox"
        className="size-5 accent-fg"
        checked={value.classifyGround}
        onChange={(e) => onChange({ ...value, classifyGround: e.target.checked })}
      />
      {m.calc_filter_ground()}
    </label>
  </>
);

/** Mehrere Bereiche (Häkchen) */
export const RegionChecks = ({
  regions,
  value,
  onChange,
  emptyText,
}: {
  regions: Region[];
  value: number[];
  onChange: (ids: number[]) => void;
  emptyText: string;
}) =>
  regions.length ? (
    <fieldset className="m-0 flex flex-col gap-2 border-0 p-0">
      <legend className="mb-2 p-0 text-[13px] text-muted">{m.calc_regions()}</legend>
      {regions.map((r) => (
        <label key={r.id} className="flex items-center gap-2 text-[14px]">
          <input
            type="checkbox"
            className="size-5 accent-fg"
            checked={value.includes(r.id)}
            onChange={() => onChange(value.includes(r.id) ? value.filter((x) => x !== r.id) : [...value, r.id])}
          />
          {r.name}
        </label>
      ))}
    </fieldset>
  ) : (
    <p className="m-0 text-[13px] text-muted">{emptyText}</p>
  );

/** Ein Bereich oder keiner */
export const RegionSelect = ({
  regions,
  value,
  onChange,
}: {
  regions: Region[];
  value: number | null;
  onChange: (id: number | null) => void;
}) => (
  <Label>
    {m.calc_region_one()}
    <Select value={value ?? ""} onChange={(e) => onChange(e.target.value ? Number(e.target.value) : null)}>
      <option value="">{m.calc_region_none()}</option>
      {regions.map((r) => (
        <option key={r.id} value={r.id}>
          {r.name}
        </option>
      ))}
    </Select>
  </Label>
);
