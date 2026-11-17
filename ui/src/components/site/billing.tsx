/**
 * Abrechnung einer Baustelle: Aufmaß für einen Zeitraum zwischen zwei Befliegungen mit Leistungsposition,
 * Einheitspreis und Betrag, Mengen in m³ fest und lose, Tonnen und LKW-Ladungen; Abgleich mit Lieferscheinen.
 */
import { useQuery } from "@tanstack/react-query";
import { FileDown } from "lucide-react";
import { useState } from "react";
import { EmptyState, ErrorText, Section } from "@/components/page";
import { Kpis } from "@/components/report/kpis";
import { buttonVariants } from "@/components/ui/button";
import { Label, Select } from "@/components/ui/input";
import { statementPdfHref } from "@/lib/api";
import { defaultPeriod, readyCaptures } from "@/lib/billing";
import { errorText } from "@/lib/errors";
import { formatDate, formatEuro, formatNumber, formatVolume } from "@/lib/format";
import { capturesQuery, statementQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";
import { Deliveries } from "./billing-deliveries";
import { BillingLayers } from "./billing-layers";
import { BillingSettingsForm } from "./billing-settings";
import { ShareButton, Shares } from "./billing-shares";
import { ChecksTable, PositionsTable } from "./billing-tables";

export function Billing({ siteId, canEdit }: { siteId: number; canEdit: boolean }) {
  const captures = useQuery(capturesQuery(siteId));
  const ready = readyCaptures(captures.data ?? []);
  const suggested = defaultPeriod(captures.data ?? []);
  const [picked, setPicked] = useState<{ from: number; to: number } | null>(null);
  const known = (id: number) => ready.some((c) => c.id === id);
  const period = picked && known(picked.from) && known(picked.to) ? picked : suggested;
  const valid = period !== null && period.from !== period.to;
  const statement = useQuery({
    ...statementQuery(siteId, period?.from ?? 0, period?.to ?? 0),
    enabled: valid,
  });
  const s = statement.data;
  const bank = s?.positions.reduce((n, p) => n + (p.period?.bankM3 ?? 0), 0) ?? 0;

  return (
    <Section
      title={m.billing_title()}
      description={m.billing_text()}
      actions={
        s && period ? (
          <>
            {canEdit ? <ShareButton siteId={siteId} period={period} /> : null}
            <a
              href={statementPdfHref(siteId, period.from, period.to)}
              className={buttonVariants({ variant: "primary" })}
            >
              <FileDown size={16} aria-hidden="true" />
              {m.billing_pdf()}
            </a>
          </>
        ) : null
      }
    >
      {period === null ? (
        captures.isPending ? null : (
          <EmptyState>{m.billing_empty()}</EmptyState>
        )
      ) : (
        <>
          <div className="grid gap-3 sm:grid-cols-2">
            {(["from", "to"] as const).map((k) => (
              <Label key={k}>
                {k === "from" ? m.billing_from() : m.billing_to()}
                <Select
                  value={period[k]}
                  onChange={(e) => setPicked({ ...period, [k]: Number(e.target.value) })}
                >
                  {ready.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name} ({formatDate(c.capturedAt)})
                    </option>
                  ))}
                </Select>
              </Label>
            ))}
          </div>
          <ErrorText>{statement.error ? errorText(statement.error) : null}</ErrorText>
          {s?.missing.length ? (
            <p role="status" className="m-0 text-[13px] text-warn">
              {m.billing_missing({ captures: s.missing.join(", ") })}
            </p>
          ) : null}
          {s ? (
            <>
              <Kpis
                items={[
                  { label: m.billing_kpi_bank(), value: formatVolume(bank), className: "text-cut" },
                  {
                    label: m.billing_kpi_positions(),
                    value: formatNumber(s.positions.length, 0),
                  },
                  {
                    label: m.billing_kpi_amount(),
                    value: s.amount == null ? "–" : formatEuro(s.amount),
                  },
                ]}
              />
              {s.positions.length ? (
                <PositionsTable statement={s} />
              ) : (
                <EmptyState>{m.billing_no_regions()}</EmptyState>
              )}
              {s.unpriced && s.positions.length ? (
                <p className="m-0 text-[12px] text-muted">
                  {m.billing_unpriced({ count: s.unpriced })}
                </p>
              ) : null}
              {s.layersOutdated ? (
                <p role="status" className="m-0 text-[13px] text-warn">
                  {m.billing_layers_outdated()}
                </p>
              ) : null}
              {s.unconverted ? (
                <p role="status" className="m-0 text-[13px] text-warn">
                  {m.billing_unconverted({ count: s.unconverted })}
                </p>
              ) : null}
              {s.checks.length ? <ChecksTable checks={s.checks} /> : null}
            </>
          ) : null}
        </>
      )}
      <BillingLayers siteId={siteId} canEdit={canEdit} />
      <BillingSettingsForm siteId={siteId} canEdit={canEdit} />
      <Deliveries siteId={siteId} canEdit={canEdit} />
      <Shares siteId={siteId} canEdit={canEdit} />
    </Section>
  );
}
