/** Massenziel eines Bereichs bearbeiten: Material, Soll-Menge in m³, Abtrag oder Auftrag */
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { type FormEvent, useState } from "react";
import { ErrorText } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input, Label, Select } from "@/components/ui/input";
import { api, type Region, type TargetKind } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { formatNumber } from "@/lib/format";
import { m } from "@/paraglide/messages";

/** Vorschläge fürs Material (frei änderbar) */
export const MATERIALS = [
  "Oberboden",
  "Boden Klasse 3",
  "Boden Klasse 4",
  "Boden Klasse 5",
  "Fels",
  "Schotter",
  "Recycling",
];

/** „Boden Klasse 4 · Ziel 1.000 m³ Abtrag“ oder „Oberboden · Lager“ */
export const targetText = (r: Region) =>
  [
    r.material ?? null,
    r.targetKind === "stock" ? m.target_stock() : null,
    r.targetM3 && r.targetKind !== "stock"
      ? m.target_summary({
          m3: formatNumber(r.targetM3, 0),
          kind: r.targetKind === "fill" ? m.target_fill() : m.target_cut(),
        })
      : null,
  ]
    .filter(Boolean)
    .join(" · ");

const parse = (s: string) => {
  const v = Number(s.replace(/\./g, "").replace(",", "."));
  return s.trim() && Number.isFinite(v) ? v : undefined;
};

export function TargetForm({ region, onDone }: { region: Region; onDone: () => void }) {
  const qc = useQueryClient();
  const [material, setMaterial] = useState(region.material ?? "");
  const [target, setTarget] = useState(region.targetM3 ? String(region.targetM3) : "");
  const [kind, setKind] = useState<TargetKind>(region.targetKind ?? "cut");
  const save = useMutation({
    mutationFn: () =>
      api.updateRegion(region.id, {
        material: material.trim() || undefined,
        targetM3: kind === "stock" ? undefined : parse(target),
        targetKind: kind,
      }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["regions", region.siteId] });
      await qc.invalidateQueries({ queryKey: ["dashboard", region.siteId] });
      onDone();
    },
  });
  const submit = (e: FormEvent) => {
    e.preventDefault();
    save.mutate();
  };
  const listId = `materials-${region.id}`;
  return (
    <form
      onSubmit={submit}
      aria-label={m.target_form({ name: region.name })}
      className="flex flex-col gap-3 rounded-lg bg-subtle p-3"
    >
      <div className="grid gap-3 sm:grid-cols-3">
        <Label>
          {m.target_material()}
          <Input list={listId} value={material} onChange={(e) => setMaterial(e.target.value)} />
          <datalist id={listId}>
            {MATERIALS.map((x) => (
              <option key={x} value={x} />
            ))}
          </datalist>
        </Label>
        <Label>
          {m.target_kind()}
          <Select value={kind} onChange={(e) => setKind(e.target.value as TargetKind)}>
            <option value="cut">{m.target_cut()}</option>
            <option value="fill">{m.target_fill()}</option>
            <option value="stock">{m.target_stock_option()}</option>
          </Select>
        </Label>
        {kind === "stock" ? (
          <p className="m-0 self-end pb-2.5 text-[13px] text-muted">{m.target_stock_hint()}</p>
        ) : (
          <Label>
            {m.target_m3()}
            <Input inputMode="decimal" value={target} onChange={(e) => setTarget(e.target.value)} />
          </Label>
        )}
      </div>
      <ErrorText>{save.error ? errorText(save.error) : null}</ErrorText>
      <div className="flex justify-end gap-2">
        <Button size="sm" onClick={onDone}>
          {m.common_cancel()}
        </Button>
        <Button size="sm" type="submit" variant="primary" disabled={save.isPending}>
          {m.target_save()}
        </Button>
      </div>
    </form>
  );
}
