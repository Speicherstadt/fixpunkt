/** Lieferscheine und Wiegescheine: einzeln erfassen, als CSV einlesen, löschen */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronDown, ChevronRight, Trash2, Upload } from "lucide-react";
import { type FormEvent, useId, useState } from "react";
import { ErrorText } from "@/components/page";
import { Button, buttonVariants } from "@/components/ui/button";
import { Input, Label, Select } from "@/components/ui/input";
import { api, type DeliveryUnit, importDeliveries } from "@/lib/api";
import { deliveryUnitLabel, today } from "@/lib/billing";
import { errorText } from "@/lib/errors";
import { formatDay, formatNumber, parseDecimal } from "@/lib/format";
import { billingSettingsQuery, deliveriesQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";

export function Deliveries({ siteId, canEdit }: { siteId: number; canEdit: boolean }) {
  const [open, setOpen] = useState(false);
  const list = useQuery(deliveriesQuery(siteId));
  const qc = useQueryClient();
  const inputId = useId();
  const refresh = () => qc.invalidateQueries({ queryKey: ["billing", siteId] });
  const upload = useMutation({
    mutationFn: (f: File) => importDeliveries(siteId, f),
    onSuccess: refresh,
  });
  const remove = useMutation({ mutationFn: api.deleteDelivery, onSuccess: refresh });
  const count = list.data?.length ?? 0;
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <Button
          variant="ghost"
          size="sm"
          className="px-1"
          aria-expanded={open}
          onClick={() => setOpen(!open)}
        >
          {open ? (
            <ChevronDown size={16} aria-hidden="true" />
          ) : (
            <ChevronRight size={16} aria-hidden="true" />
          )}
          {m.billing_deliveries({ count })}
        </Button>
        {canEdit ? (
          <>
            <input
              id={inputId}
              type="file"
              accept=".csv,.txt,text/csv"
              className="sr-only"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) upload.mutate(f);
                e.target.value = "";
              }}
            />
            <label htmlFor={inputId} className={buttonVariants({ size: "sm" })}>
              <Upload size={16} aria-hidden="true" />
              {m.billing_import()}
            </label>
          </>
        ) : null}
      </div>
      <ErrorText>
        {upload.error ? errorText(upload.error) : remove.error ? errorText(remove.error) : null}
      </ErrorText>
      {upload.data ? (
        <p role="status" className="m-0 text-[13px] text-ok">
          {m.billing_imported({ count: upload.data.imported })}
        </p>
      ) : null}
      {open ? (
        <>
          <p className="m-0 text-[13px] text-muted">{m.billing_deliveries_text()}</p>
          {count ? (
            <ul className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0">
              {list.data?.map((d) => (
                <li key={d.id} className="flex items-center gap-3 px-3 py-2 text-[13px]">
                  <span className="tabular w-24 shrink-0 text-muted">
                    {formatDay(d.deliveredAt)}
                  </span>
                  <span className="min-w-0 flex-1 truncate">
                    {d.material ?? m.billing_no_material()}
                    {d.reference ? <span className="text-muted"> · {d.reference}</span> : null}
                  </span>
                  <span className="tabular shrink-0">
                    {formatNumber(d.quantity, d.unit === "loads" ? 0 : 1)}{" "}
                    {deliveryUnitLabel(d.unit)}
                  </span>
                  {canEdit ? (
                    <Button
                      variant="ghost"
                      size="iconSm"
                      onClick={() => remove.mutate(d.id)}
                      aria-label={m.billing_delivery_delete({
                        reference: d.reference ?? formatDay(d.deliveredAt),
                      })}
                    >
                      <Trash2 size={16} aria-hidden="true" />
                    </Button>
                  ) : null}
                </li>
              ))}
            </ul>
          ) : null}
          {canEdit ? <DeliveryForm siteId={siteId} /> : null}
        </>
      ) : null}
    </div>
  );
}

function DeliveryForm({ siteId }: { siteId: number }) {
  const qc = useQueryClient();
  const settings = useQuery(billingSettingsQuery(siteId));
  const [date, setDate] = useState(today());
  const [material, setMaterial] = useState("");
  const [quantity, setQuantity] = useState("");
  const [unit, setUnit] = useState<DeliveryUnit>("t");
  const [reference, setReference] = useState("");
  const add = useMutation({
    mutationFn: () =>
      api.addDelivery(siteId, {
        date,
        material: material.trim() || undefined,
        quantity: parseDecimal(quantity) ?? 0,
        unit,
        reference: reference.trim() || undefined,
      }),
    onSuccess: async () => {
      setQuantity("");
      setReference("");
      await qc.invalidateQueries({ queryKey: ["billing", siteId] });
    },
  });
  const submit = (e: FormEvent) => {
    e.preventDefault();
    add.mutate();
  };
  const listId = `delivery-materials-${siteId}`;
  return (
    <form
      onSubmit={submit}
      aria-label={m.billing_delivery_add()}
      className="flex flex-col gap-3 rounded-lg bg-subtle p-3"
    >
      <div className="grid gap-3 sm:grid-cols-5">
        <Label>
          {m.billing_date()}
          <Input type="date" required value={date} onChange={(e) => setDate(e.target.value)} />
        </Label>
        <Label>
          {m.target_material()}
          <Input list={listId} value={material} onChange={(e) => setMaterial(e.target.value)} />
          <datalist id={listId}>
            {settings.data?.materials.map((x) => (
              <option key={x.name} value={x.name} />
            ))}
          </datalist>
        </Label>
        <Label>
          {m.billing_quantity()}
          <Input
            inputMode="decimal"
            required
            value={quantity}
            onChange={(e) => setQuantity(e.target.value)}
          />
        </Label>
        <Label>
          {m.billing_unit()}
          <Select value={unit} onChange={(e) => setUnit(e.target.value as DeliveryUnit)}>
            {(["t", "m3", "loads"] as const).map((u) => (
              <option key={u} value={u}>
                {deliveryUnitLabel(u)}
              </option>
            ))}
          </Select>
        </Label>
        <Label>
          {m.billing_reference()}
          <Input value={reference} onChange={(e) => setReference(e.target.value)} />
        </Label>
      </div>
      <ErrorText>{add.error ? errorText(add.error) : null}</ErrorText>
      <div className="flex justify-end">
        <Button size="sm" type="submit" variant="primary" disabled={add.isPending}>
          {m.billing_delivery_add()}
        </Button>
      </div>
    </form>
  );
}
