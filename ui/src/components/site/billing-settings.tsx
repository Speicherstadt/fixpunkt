/** Kennwerte für die Umrechnung: Dichte und Auflockerung je Material, Nutzlast eines LKW */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronDown, ChevronRight } from "lucide-react";
import { type FormEvent, useState } from "react";
import { ErrorText } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input, Label } from "@/components/ui/input";
import { api, type BillingSettings } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { formatNumber, parseDecimal } from "@/lib/format";
import { billingSettingsQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";

const text = (v: number | null) => (v == null ? "" : formatNumber(v, 2));
const value = (s: string) => parseDecimal(s) ?? null;

export function BillingSettingsForm({ siteId, canEdit }: { siteId: number; canEdit: boolean }) {
  const [open, setOpen] = useState(false);
  const settings = useQuery(billingSettingsQuery(siteId));
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
        {m.billing_settings()}
      </Button>
      {open && settings.data ? (
        <SettingsEditor
          // neu, wenn Materialien dazukommen (z. B. eine neue Schicht)
          key={settings.data.materials.map((x) => x.name).join("|")}
          siteId={siteId}
          canEdit={canEdit}
          initial={settings.data}
        />
      ) : null}
    </div>
  );
}

function SettingsEditor({
  siteId,
  canEdit,
  initial,
}: {
  siteId: number;
  canEdit: boolean;
  initial: BillingSettings;
}) {
  const qc = useQueryClient();
  const [truck, setTruck] = useState(text(initial.truckT));
  const [rows, setRows] = useState(
    initial.materials.map((x) => ({
      name: x.name,
      density: text(x.density),
      bulking: text(x.bulking),
    })),
  );
  const save = useMutation({
    mutationFn: () =>
      api.saveBillingSettings(siteId, {
        truckT: value(truck),
        // Schichten bleiben, wie sie sind
        layers: undefined,
        materials: rows.map((r) => ({
          name: r.name,
          density: value(r.density),
          bulking: value(r.bulking),
        })),
      }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["billing", siteId] }),
  });
  const set = (i: number, k: "density" | "bulking", v: string) =>
    setRows(rows.map((r, j) => (j === i ? { ...r, [k]: v } : r)));
  const submit = (e: FormEvent) => {
    e.preventDefault();
    save.mutate();
  };
  return (
    <form
      onSubmit={submit}
      aria-label={m.billing_settings()}
      className="flex flex-col gap-3 rounded-lg bg-subtle p-3"
    >
      <p className="m-0 text-[13px] text-muted">{m.billing_settings_text()}</p>
      <Label className="max-w-xs">
        {m.billing_truck()}
        <Input
          inputMode="decimal"
          value={truck}
          disabled={!canEdit}
          onChange={(e) => setTruck(e.target.value)}
        />
      </Label>
      {rows.length ? (
        <ul className="m-0 flex list-none flex-col gap-3 p-0">
          {rows.map((r, i) => (
            <li key={r.name} className="grid items-end gap-3 sm:grid-cols-3">
              <span className="pb-2.5 text-[14px] font-medium">{r.name}</span>
              <Label>
                {m.billing_density()}
                <Input
                  inputMode="decimal"
                  value={r.density}
                  disabled={!canEdit}
                  aria-label={m.billing_density_of({ material: r.name })}
                  onChange={(e) => set(i, "density", e.target.value)}
                />
              </Label>
              <Label>
                {m.billing_bulking()}
                <Input
                  inputMode="decimal"
                  value={r.bulking}
                  disabled={!canEdit}
                  aria-label={m.billing_bulking_of({ material: r.name })}
                  onChange={(e) => set(i, "bulking", e.target.value)}
                />
              </Label>
            </li>
          ))}
        </ul>
      ) : (
        <p className="m-0 text-[13px] text-muted">{m.billing_settings_none()}</p>
      )}
      <ErrorText>{save.error ? errorText(save.error) : null}</ErrorText>
      {save.isSuccess ? (
        <p role="status" className="m-0 text-[13px] text-ok">
          {m.billing_settings_saved()}
        </p>
      ) : null}
      {canEdit ? (
        <div className="flex justify-end">
          <Button size="sm" type="submit" variant="primary" disabled={save.isPending}>
            {m.billing_settings_save()}
          </Button>
        </div>
      ) : null}
    </form>
  );
}
