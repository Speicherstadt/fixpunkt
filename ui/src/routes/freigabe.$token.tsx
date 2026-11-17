/**
 * Freigabe eines Aufmaßes für den Auftraggeber, ohne Anmeldung: das eingefrorene Aufmaß ansehen, als PDF laden,
 * anerkennen oder Einwand erheben. Nur die erste Entscheidung zählt.
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { Check, FileDown, HardHat, MessageSquareWarning } from "lucide-react";
import { type FormEvent, useState } from "react";
import { ErrorText, PageHeader, Section } from "@/components/page";
import { Kpis } from "@/components/report/kpis";
import { ChecksTable, PositionsTable } from "@/components/site/billing-tables";
import { Button, buttonVariants } from "@/components/ui/button";
import { Input, Label, Textarea } from "@/components/ui/input";
import { PlaceholderRows } from "@/components/ui/placeholder";
import { api, type DecisionInput, type PublicShare, publicSharePdfHref } from "@/lib/api";
import { decisionText } from "@/lib/billing";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { formatDate, formatEuro, formatVolume } from "@/lib/format";
import { m } from "@/paraglide/messages";

export const Route = createFileRoute("/freigabe/$token")({ component: SharePage });

function SharePage() {
  const { token } = Route.useParams();
  const share = useQuery({
    queryKey: ["public-share", token],
    queryFn: () => api.publicShare(token),
    retry: false,
  });
  return (
    <div className="mx-auto flex min-h-full max-w-5xl flex-col px-4 py-6 md:px-8">
      <div className="mb-4 flex items-center gap-2 font-semibold">
        <HardHat size={20} aria-hidden="true" />
        Fixpunkt
      </div>
      <main id="inhalt" className="flex flex-col gap-6">
        {share.isPending ? (
          <PlaceholderRows />
        ) : share.error ? (
          <>
            <PageHeader title={m.public_share_title()} />
            <ErrorText>{m.public_share_gone()}</ErrorText>
          </>
        ) : (
          <ShareView token={token} share={share.data} />
        )}
      </main>
    </div>
  );
}

function ShareView({ token, share }: { token: string; share: PublicShare }) {
  const s = share.statement;
  const bank = s.positions.reduce((n, p) => n + (p.period?.bankM3 ?? 0), 0);
  return (
    <>
      <PageHeader
        title={m.public_share_title()}
        subtitle={`${share.site} · ${m.share_period({
          from: `${s.from.name} (${formatDate(s.from.at)})`,
          to: `${s.to.name} (${formatDate(s.to.at)})`,
        })}`}
        actions={
          <a href={publicSharePdfHref(token)} className={buttonVariants({ variant: "primary" })}>
            <FileDown size={16} aria-hidden="true" />
            {m.billing_pdf()}
          </a>
        }
      />
      <Section title={m.billing_positions()} description={share.note ?? undefined}>
        <Kpis
          items={[
            { label: m.billing_kpi_bank(), value: formatVolume(bank), className: "text-cut" },
            { label: m.billing_kpi_positions(), value: String(s.positions.length) },
            {
              label: m.billing_kpi_amount(),
              value: s.amount == null ? "–" : formatEuro(s.amount),
            },
          ]}
        />
        <PositionsTable statement={s} />
        {s.checks.length ? <ChecksTable checks={s.checks} /> : null}
        <p className="m-0 text-[12px] text-muted">
          {m.public_share_frozen({ date: formatDate(share.createdAt) })}
        </p>
      </Section>
      <Decide token={token} share={share} />
    </>
  );
}

function Decide({ token, share }: { token: string; share: PublicShare }) {
  const qc = useQueryClient();
  const [name, setName] = useState("");
  const [organization, setOrganization] = useState("");
  const [comment, setComment] = useState("");
  const decide = useMutation({
    mutationFn: (decision: DecisionInput["decision"]) =>
      api.decideShare(token, {
        decision,
        name,
        organization: organization.trim() || undefined,
        comment: comment.trim() || undefined,
      }),
    onSuccess: (data) => qc.setQueryData(["public-share", token], data),
  });
  const submit = (e: FormEvent) => e.preventDefault();
  if (share.decision)
    return (
      <Section title={m.public_share_decision()}>
        <p
          role="status"
          className={cn(
            "m-0 text-[14px] font-medium",
            share.decision.decision === "approved" ? "text-ok" : "text-warn",
          )}
        >
          {decisionText(share.decision)}
        </p>
      </Section>
    );
  return (
    <Section title={m.public_share_decision()} description={m.public_share_decision_text()}>
      <form
        onSubmit={submit}
        aria-label={m.public_share_decision()}
        className="flex flex-col gap-3"
      >
        <div className="grid gap-3 sm:grid-cols-2">
          <Label>
            {m.public_share_name()}
            <Input
              required
              autoComplete="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </Label>
          <Label>
            {m.public_share_organization()}
            <Input
              autoComplete="organization"
              value={organization}
              onChange={(e) => setOrganization(e.target.value)}
            />
          </Label>
        </div>
        <Label>
          {m.public_share_comment()}
          <Textarea rows={3} value={comment} onChange={(e) => setComment(e.target.value)} />
        </Label>
        <ErrorText>{decide.error ? errorText(decide.error) : null}</ErrorText>
        <div className="flex flex-wrap justify-end gap-2">
          <Button
            disabled={decide.isPending || !name.trim() || !comment.trim()}
            onClick={() => decide.mutate("objected")}
          >
            <MessageSquareWarning size={16} aria-hidden="true" />
            {m.public_share_object()}
          </Button>
          <Button
            variant="primary"
            disabled={decide.isPending || !name.trim()}
            onClick={() => decide.mutate("approved")}
          >
            <Check size={16} aria-hidden="true" />
            {m.public_share_approve()}
          </Button>
        </div>
      </form>
    </Section>
  );
}
