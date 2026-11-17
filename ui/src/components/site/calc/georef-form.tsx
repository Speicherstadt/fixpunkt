/**
 * Georeferenzieren über Passpunkte: Punkte eintippen oder als Liste einfügen, Vorschau der Transformation mit
 * Restklaffen (Server, ohne etwas zu ändern), dann als Auftrag anwenden – das Ergebnis wird eine neue Aufnahme.
 */
import { useQuery } from "@tanstack/react-query";
import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { ErrorText } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input, Label, Select, Textarea } from "@/components/ui/input";
import { api } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { type Gcp, MIN_POINTS, MODES, type Mode, num, parseGcps, toInput } from "@/lib/georef";
import { chronological } from "@/lib/jobs";
import { m } from "@/paraglide/messages";
import { getLocale } from "@/paraglide/runtime";
import { CaptureSelect } from "./fields";
import type { FormProps } from "./forms";
import { Residuals } from "./residuals";

type Row = { name: string; v: string[] };

const MODE_TEXT: Record<Mode, () => string> = {
  similarity: m.georef_mode_similarity,
  rigid: m.georef_mode_rigid,
  translation: m.georef_mode_translation,
};

const empty = (i: number): Row => ({ name: `P${i + 1}`, v: ["", "", "", "", "", ""] });
/** Dezimalzeichen der Oberflächensprache */
const decimal = (): "," | "." =>
  new Intl.NumberFormat(getLocale()).format(1.5).includes(",") ? "," : ".";

const fromGcp = (g: Gcp): Row => ({
  name: g.name,
  v: [...g.src, ...g.dst].map((x) => toInput(x, decimal())),
});

/** vollständige Zeilen als Passpunkte */
const toGcps = (rows: Row[]): Gcp[] =>
  rows.flatMap((r) => {
    const v = r.v.map(num);
    if (v.some((x) => x === null) || !r.name.trim()) return [];
    const [a, b, c, d, e, f] = v as number[];
    return [{ name: r.name.trim(), src: [a!, b!, c!], dst: [d!, e!, f!] }];
  });

const COLUMNS = () => [
  m.georef_src_x(),
  m.georef_src_y(),
  m.georef_src_z(),
  m.georef_dst_x(),
  m.georef_dst_y(),
  m.georef_dst_z(),
];

export function GeorefForm({ ready, onSubmit, footer }: FormProps) {
  const clouds = chronological(ready.filter((c) => c.info?.type === "pointcloud"));
  const [capture, setCapture] = useState<number | undefined>();
  const captureId = capture ?? clouds.at(-1)?.id;
  const [mode, setMode] = useState<Mode>("similarity");
  const [rows, setRows] = useState<Row[]>([0, 1, 2].map(empty));
  const [paste, setPaste] = useState("");
  const points = toGcps(rows);
  const enough = points.length >= MIN_POINTS[mode];
  const fit = useQuery({
    queryKey: ["georef-fit", mode, points],
    queryFn: () => api.georefFit(mode, points),
    enabled: enough,
    retry: false,
  });
  const set = (i: number, k: number | "name", value: string) =>
    setRows((rs) =>
      rs.map((r, j) =>
        j !== i ? r : k === "name" ? { ...r, name: value } : { ...r, v: r.v.with(k, value) },
      ),
    );
  const reason = !captureId
    ? m.georef_no_cloud()
    : !enough
      ? m.georef_too_few({ min: MIN_POINTS[mode] })
      : fit.error
        ? m.georef_not_determinable()
        : null;
  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit({ captureId, mode, points });
      }}
    >
      <div className="grid gap-4 md:grid-cols-2">
        <CaptureSelect
          label={m.georef_capture()}
          captures={clouds}
          value={captureId}
          onChange={setCapture}
        />
        <Label>
          {m.georef_mode()}
          <Select value={mode} onChange={(e) => setMode(e.target.value as Mode)}>
            {MODES.map((x) => (
              <option key={x} value={x}>
                {MODE_TEXT[x]()}
              </option>
            ))}
          </Select>
        </Label>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[13px]">
          <caption className="sr-only">{m.georef_points()}</caption>
          <thead>
            <tr className="text-left text-[12px] text-muted">
              <th className="py-1.5 pr-2 font-medium">{m.georef_name()}</th>
              {COLUMNS().map((c) => (
                <th key={c} className="py-1.5 pr-2 font-medium">
                  {c}
                </th>
              ))}
              <th>
                <span className="sr-only">{m.common_actions()}</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r, i) => (
              // biome-ignore lint/suspicious/noArrayIndexKey: Zeilen haben keine eigene ID, Reihenfolge ist fest
              <tr key={i}>
                <td className="py-1 pr-2">
                  <Input
                    className="h-9 w-20"
                    aria-label={`${m.georef_name()} ${i + 1}`}
                    value={r.name}
                    onChange={(e) => set(i, "name", e.target.value)}
                  />
                </td>
                {COLUMNS().map((c, k) => (
                  <td key={c} className="py-1 pr-2">
                    <Input
                      className="h-9 w-32"
                      inputMode="decimal"
                      aria-label={`${c} ${r.name}`}
                      value={r.v[k]}
                      onChange={(e) => set(i, k, e.target.value)}
                    />
                  </td>
                ))}
                <td className="py-1">
                  <Button
                    size="iconSm"
                    variant="ghost"
                    aria-label={m.georef_remove({ name: r.name })}
                    onClick={() => setRows((rs) => rs.filter((_, j) => j !== i))}
                  >
                    <Trash2 size={14} aria-hidden="true" />
                  </Button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button size="sm" onClick={() => setRows((rs) => [...rs, empty(rs.length)])}>
          <Plus size={14} aria-hidden="true" />
          {m.georef_add()}
        </Button>
      </div>
      <details className="text-[13px]">
        <summary className="cursor-pointer text-muted">{m.georef_paste()}</summary>
        <div className="mt-2 flex flex-col gap-2">
          <Label>
            {m.georef_paste_label()}
            <Textarea
              value={paste}
              placeholder="P1;612000,10;5352999,90;302,10;512000,10;5403000,10;300,00"
              onChange={(e) => setPaste(e.target.value)}
            />
          </Label>
          <div>
            <Button
              size="sm"
              disabled={!parseGcps(paste).length}
              onClick={() => {
                setRows(parseGcps(paste).map(fromGcp));
                setPaste("");
              }}
            >
              {m.georef_paste_apply({ count: parseGcps(paste).length })}
            </Button>
          </div>
        </div>
      </details>
      {fit.data ? <Residuals report={fit.data} /> : null}
      <ErrorText>{fit.error ? errorText(fit.error) : null}</ErrorText>
      {footer(reason)}
    </form>
  );
}
