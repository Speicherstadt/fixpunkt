/**
 * Pläne (DXF) der Baustelle: hochladen, Ebenen und Inhalt sehen, Plan/Ist der Höhenpunkte gegen die neueste
 * Aufnahme. In der Ansicht lässt sich ein Plan über das Höhenbild legen.
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FileDown, FileText, Trash2, Upload } from "lucide-react";
import { useId, useState } from "react";
import { EmptyState, ErrorText, Section } from "@/components/page";
import { cm } from "@/components/report/compare-report";
import { Kpis } from "@/components/report/kpis";
import { Button, buttonVariants } from "@/components/ui/button";
import { Label, Select } from "@/components/ui/input";
import { api, type Plan, uploadPlan } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { formatCount, formatHeight } from "@/lib/format";
import { chronological } from "@/lib/jobs";
import { planIst, planIstSummary } from "@/lib/plans";
import { capturesQuery, planGeometryQuery, plansQuery } from "@/lib/queries";
import { fetchGrid } from "@/lib/view/grid";
import { m } from "@/paraglide/messages";

export function Plans({ siteId, canEdit }: { siteId: number; canEdit: boolean }) {
  const plans = useQuery(plansQuery(siteId));
  const queryClient = useQueryClient();
  const inputId = useId();
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["plans", siteId] });
  const upload = useMutation({
    mutationFn: async (files: File[]) => {
      for (const f of files) await uploadPlan(siteId, f);
    },
    onSettled: refresh,
  });
  const remove = useMutation({ mutationFn: api.deletePlan, onSuccess: refresh });
  const [picked, setPicked] = useState<number | null>(null);
  const withSpots = (plans.data ?? []).filter((p) => p.info.spots > 0);
  const planId = picked ?? withSpots[0]?.id ?? null;

  return (
    <Section
      title={m.plans_title()}
      description={m.plans_text()}
      actions={
        canEdit ? (
          <>
            <input
              id={inputId}
              type="file"
              multiple
              accept=".dxf"
              className="sr-only"
              onChange={(e) => {
                upload.mutate(Array.from(e.target.files ?? []));
                e.target.value = "";
              }}
            />
            <label htmlFor={inputId} className={buttonVariants({ variant: "primary" })}>
              <Upload size={16} aria-hidden="true" />
              {m.plans_upload()}
            </label>
          </>
        ) : null
      }
    >
      {upload.isPending ? (
        <p role="status" className="m-0 text-[13px] text-muted">
          {m.plans_uploading()}
        </p>
      ) : null}
      <ErrorText>
        {upload.error ? errorText(upload.error) : remove.error ? errorText(remove.error) : null}
      </ErrorText>
      {plans.data?.length ? (
        <ul className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0">
          {plans.data.map((p) => (
            <PlanRow key={p.id} plan={p} canEdit={canEdit} onDelete={() => remove.mutate(p.id)} />
          ))}
        </ul>
      ) : plans.isPending ? null : (
        <EmptyState>{m.plans_empty()}</EmptyState>
      )}
      {withSpots.length && planId !== null ? (
        <div className="flex flex-col gap-3">
          <h3 className="m-0 text-[14px] font-semibold">{m.plans_ist_title()}</h3>
          {withSpots.length > 1 ? (
            <Label className="max-w-sm">
              {m.plans_pick()}
              <Select value={planId} onChange={(e) => setPicked(Number(e.target.value))}>
                {withSpots.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </Select>
            </Label>
          ) : null}
          <PlanIst siteId={siteId} planId={planId} />
        </div>
      ) : null}
    </Section>
  );
}

function PlanRow({
  plan: p,
  canEdit,
  onDelete,
}: {
  plan: Plan;
  canEdit: boolean;
  onDelete: () => void;
}) {
  return (
    <li className="flex items-center gap-3 px-4 py-3">
      <FileText size={20} aria-hidden="true" className="shrink-0 text-muted" />
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="truncate font-medium">{p.name}</span>
        <span className="truncate text-[12px] text-muted">
          {m.plans_counts({
            layers: p.info.layers.length,
            paths: formatCount(p.info.paths),
            spots: formatCount(p.info.spots),
          })}
          {p.info.zoneOffset ? ` · ${m.plans_zone({ zone: p.info.zoneOffset / 1e6 })}` : ""}
        </span>
      </div>
      {p.info.spots ? (
        <a
          href={`/api/plans/${p.id}/report`}
          className={buttonVariants({ variant: "ghost", size: "iconSm" })}
          aria-label={m.plans_report({ name: p.name })}
          title={m.plans_report({ name: p.name })}
        >
          <FileDown size={16} aria-hidden="true" />
        </a>
      ) : null}
      {canEdit ? (
        <Button
          size="iconSm"
          variant="ghost"
          aria-label={m.plans_delete({ name: p.name })}
          onClick={onDelete}
        >
          <Trash2 size={14} aria-hidden="true" />
        </Button>
      ) : null}
    </li>
  );
}

/** Höhenpunkte des Plans gegen die neueste geprüfte Aufnahme */
function PlanIst({ siteId, planId }: { siteId: number; planId: number }) {
  const geometry = useQuery(planGeometryQuery(planId));
  const captures = useQuery(capturesQuery(siteId));
  const latest = chronological((captures.data ?? []).filter((c) => c.status === "ready")).at(-1);
  const grid = useQuery({
    queryKey: ["grid", "capture", latest?.id],
    queryFn: () => fetchGrid(`/api/captures/${latest!.id}/grid`),
    enabled: latest !== undefined,
    staleTime: Number.POSITIVE_INFINITY,
  });
  if (!geometry.data) return null;
  const rows = planIst(geometry.data, grid.data);
  const s = planIstSummary(rows);
  return (
    <>
      <p className="m-0 text-[12px] text-muted">
        {latest ? m.plans_against({ name: latest.name }) : m.plans_no_capture()}
      </p>
      <Kpis
        items={[
          { label: m.plans_measured(), value: `${s.measured} / ${rows.length}` },
          { label: m.plans_mean(), value: s.mean === null ? "–" : cm(s.mean) },
          { label: m.plans_max(), value: s.maxAbs === null ? "–" : cm(s.maxAbs) },
        ]}
      />
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[13px]">
          <caption className="sr-only">{m.plans_ist_title()}</caption>
          <thead>
            <tr className="text-left text-[12px] text-muted">
              <th className="py-1.5 pr-3 font-medium">{m.plans_layer()}</th>
              <th className="py-1.5 pr-3 font-medium">{m.plans_position()}</th>
              <th className="py-1.5 pr-3 text-right font-medium">{m.plans_soll()}</th>
              <th className="py-1.5 pr-3 text-right font-medium">{m.plans_ist()}</th>
              <th className="py-1.5 text-right font-medium">{m.plans_diff()}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r) => (
              <tr key={`${r.at[0]}-${r.at[1]}-${r.soll}`} className="border-t border-border">
                <td className="py-1.5 pr-3">{r.layer}</td>
                <td className="tabular py-1.5 pr-3">
                  {r.at.map((v) => v.toFixed(2).replace(".", ",")).join(" / ")}
                </td>
                <td className="tabular py-1.5 pr-3 text-right">{formatHeight(r.soll)}</td>
                <td className="tabular py-1.5 pr-3 text-right">
                  {r.ist === null ? "–" : formatHeight(r.ist)}
                </td>
                <td className="tabular py-1.5 text-right">{r.diff === null ? "–" : cm(r.diff)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
