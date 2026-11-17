/**
 * Schichten (Homogenbereiche): von oben nach unten mit Unterkante unter dem Urgelände, die unterste reicht bis
 * zur Sohle; je Schicht LV-Position und Preis. Gelten für Aushub-Berechnungen, die danach starten.
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronDown, ChevronRight, Plus, Trash2 } from "lucide-react";
import { type FormEvent, useState } from "react";
import { ErrorText } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input, Label, Select } from "@/components/ui/input";
import { api, type BillingSettings, type PriceUnit } from "@/lib/api";
import { priceUnitLabel } from "@/lib/billing";
import { errorText } from "@/lib/errors";
import { formatNumber, parseDecimal } from "@/lib/format";
import { billingSettingsQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";

type Row = {
  key: number;
  name: string;
  bottom: string;
  position: string;
  price: string;
  unit: PriceUnit;
};

/** stabile Schlüssel für Zeilen, die noch keine Kennung haben */
let nextKey = 0;
const newKey = () => ++nextKey;

const text = (v: number | null | undefined) => (v == null ? "" : formatNumber(v, 2));

export function BillingLayers({ siteId, canEdit }: { siteId: number; canEdit: boolean }) {
  const [open, setOpen] = useState(false);
  const settings = useQuery(billingSettingsQuery(siteId));
  const count = settings.data?.layers?.length ?? 0;
  return (
    <div className="flex flex-col gap-3">
      <Button
        variant="ghost"
        size="sm"
        className="self-start px-1"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        {open ? (
          <ChevronDown size={16} aria-hidden="true" />
        ) : (
          <ChevronRight size={16} aria-hidden="true" />
        )}
        {m.billing_layers({ count })}
      </Button>
      {open && settings.data ? (
        <LayersEditor siteId={siteId} canEdit={canEdit} settings={settings.data} />
      ) : null}
    </div>
  );
}

function LayersEditor({
  siteId,
  canEdit,
  settings,
}: {
  siteId: number;
  canEdit: boolean;
  settings: BillingSettings;
}) {
  const qc = useQueryClient();
  const [rows, setRows] = useState<Row[]>(
    (settings.layers ?? []).map((l) => ({
      key: newKey(),
      name: l.name,
      bottom: text(l.bottomM),
      position: l.lvPosition ?? "",
      price: text(l.unitPrice),
      unit: l.priceUnit ?? "m3",
    })),
  );
  const save = useMutation({
    mutationFn: () =>
      api.saveBillingSettings(siteId, {
        ...settings,
        layers: rows.map((r, i) => ({
          name: r.name.trim(),
          // die unterste reicht bis zur Sohle
          bottomM: i === rows.length - 1 ? null : (parseDecimal(r.bottom) ?? null),
          lvPosition: r.position.trim() || null,
          unitPrice: parseDecimal(r.price) ?? null,
          priceUnit: r.unit,
        })),
      }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["billing", siteId] }),
  });
  const set = (i: number, patch: Partial<Row>) =>
    setRows(rows.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  const submit = (e: FormEvent) => {
    e.preventDefault();
    save.mutate();
  };
  return (
    <form
      onSubmit={submit}
      aria-label={m.billing_layers_title()}
      className="flex flex-col gap-3 rounded-lg bg-subtle p-3"
    >
      <p className="m-0 text-[13px] text-muted">{m.billing_layers_text()}</p>
      {rows.length ? (
        <ol className="m-0 flex list-none flex-col gap-3 p-0">
          {rows.map((r, i) => {
            const last = i === rows.length - 1;
            const name = r.name || String(i + 1);
            return (
              <li
                key={r.key}
                className="grid items-end gap-3 sm:grid-cols-[2fr_1fr_1.5fr_1fr_1fr_auto]"
              >
                <Label>
                  {m.billing_layer_name()}
                  <Input
                    required
                    value={r.name}
                    disabled={!canEdit}
                    onChange={(e) => set(i, { name: e.target.value })}
                  />
                </Label>
                <Label>
                  {m.billing_layer_bottom()}
                  <Input
                    inputMode="decimal"
                    required={!last}
                    disabled={!canEdit || last}
                    placeholder={last ? m.billing_layer_to_bottom() : undefined}
                    value={last ? "" : r.bottom}
                    aria-label={m.billing_layer_bottom_of({ layer: name })}
                    onChange={(e) => set(i, { bottom: e.target.value })}
                  />
                </Label>
                <Label>
                  {m.billing_position()}
                  <Input
                    value={r.position}
                    disabled={!canEdit}
                    aria-label={m.billing_layer_position_of({ layer: name })}
                    onChange={(e) => set(i, { position: e.target.value })}
                  />
                </Label>
                <Label>
                  {m.billing_unit_price()}
                  <Input
                    inputMode="decimal"
                    value={r.price}
                    disabled={!canEdit}
                    aria-label={m.billing_layer_price_of({ layer: name })}
                    onChange={(e) => set(i, { price: e.target.value })}
                  />
                </Label>
                <Label>
                  {m.billing_price_unit()}
                  <Select
                    value={r.unit}
                    disabled={!canEdit}
                    aria-label={m.billing_layer_unit_of({ layer: name })}
                    onChange={(e) => set(i, { unit: e.target.value as PriceUnit })}
                  >
                    {(["m3", "m3_loose", "t"] as const).map((u) => (
                      <option key={u} value={u}>
                        {m.billing_per({ unit: priceUnitLabel(u) })}
                      </option>
                    ))}
                  </Select>
                </Label>
                {canEdit ? (
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={m.billing_layer_delete({ layer: name })}
                    onClick={() => setRows(rows.filter((_, j) => j !== i))}
                  >
                    <Trash2 size={16} aria-hidden="true" />
                  </Button>
                ) : null}
              </li>
            );
          })}
        </ol>
      ) : (
        <p className="m-0 text-[13px] text-muted">{m.billing_layers_none()}</p>
      )}
      <ErrorText>{save.error ? errorText(save.error) : null}</ErrorText>
      {save.isSuccess ? (
        <p role="status" className="m-0 text-[13px] text-ok">
          {m.billing_layers_saved()}
        </p>
      ) : null}
      {canEdit ? (
        <div className="flex flex-wrap justify-between gap-2">
          <Button
            size="sm"
            onClick={() =>
              setRows([
                ...rows,
                { key: newKey(), name: "", bottom: "", position: "", price: "", unit: "m3" },
              ])
            }
          >
            <Plus size={16} aria-hidden="true" />
            {m.billing_layer_add()}
          </Button>
          <Button size="sm" type="submit" variant="primary" disabled={save.isPending}>
            {m.billing_layers_save()}
          </Button>
        </div>
      ) : null}
    </form>
  );
}
