import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowLeft, Download, FileSpreadsheet, Printer, TriangleAlert } from "lucide-react";
import { ErrorText, PageHeader, Section } from "@/components/page";
import { VolumeTable } from "@/components/report/volume-table";
import { nameOf } from "@/components/site/calculations";
import { Button, buttonVariants } from "@/components/ui/button";
import { PlaceholderRows } from "@/components/ui/placeholder";
import { StatusBadge } from "@/components/ui/status";
import { VolumeResult } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { formatDateTime, formatVolume } from "@/lib/format";
import { capturesQuery, jobQuery, siteQuery } from "@/lib/queries";
import { needsCoverageWarning, reportCsv, reportRows } from "@/lib/report";
import { m } from "@/paraglide/messages";

export const Route = createFileRoute("/_app/jobs/$jobId")({ component: JobPage });

function JobPage() {
  const jobId = Number(Route.useParams().jobId);
  const job = useQuery(jobQuery(jobId));
  const siteId = job.data?.siteId ?? 0;
  const site = useQuery({ ...siteQuery(siteId), enabled: siteId > 0 });
  const captures = useQuery({ ...capturesQuery(siteId), enabled: siteId > 0 });
  if (job.isPending) return <PlaceholderRows />;
  if (job.error) return <ErrorText>{errorText(job.error)}</ErrorText>;
  const j = job.data;
  const names = (id: unknown) => nameOf(captures.data, id);
  const result = VolumeResult.safeParse(j.result);
  const back = (
    <Link
      to="/sites/$siteId"
      params={{ siteId: String(j.siteId) }}
      className={buttonVariants({ variant: "ghost" })}
    >
      <ArrowLeft size={16} aria-hidden="true" />
      {site.data?.name ?? m.nav_sites()}
    </Link>
  );

  if (!result.success) {
    return (
      <>
        <PageHeader
          title={m.report_compare({ before: names(j.params.before), after: names(j.params.after) })}
          actions={back}
        />
        <Section title={m.report_status()}>
          <StatusBadge status={j.status} />
          {j.error ? <ErrorText>{j.error}</ErrorText> : null}
          {j.status === "queued" || j.status === "running" ? (
            <p className="m-0 text-[13px] text-muted">{m.report_wait()}</p>
          ) : null}
        </Section>
      </>
    );
  }

  const { rows, total } = reportRows(result.data.regions, result.data.total, m.report_total());
  const csv = reportCsv(rows, total, [
    m.report_region(),
    m.report_area(),
    m.report_cut(),
    m.report_fill(),
    m.report_net(),
  ]);
  const csvHref = `data:text/csv;charset=utf-8,${encodeURIComponent(`﻿${csv}`)}`;
  return (
    <>
      <PageHeader
        title={m.report_title()}
        subtitle={`${site.data?.name ?? ""} · ${m.report_compare({ before: names(j.params.before), after: names(j.params.after) })} · ${formatDateTime(j.finishedAt ?? j.createdAt)}`}
        actions={
          <div className="flex flex-wrap gap-2 print:hidden">
            {back}
            <a href={csvHref} download={`mengen-${j.id}.csv`} className={buttonVariants()}>
              <FileSpreadsheet size={16} aria-hidden="true" />
              {m.report_csv()}
            </a>
            {result.data.files?.includes("diff.tif") ? (
              <a href={`/api/jobs/${j.id}/files/diff.tif`} className={buttonVariants()}>
                <Download size={16} aria-hidden="true" />
                {m.report_diff()}
              </a>
            ) : null}
            <Button onClick={() => window.print()}>
              <Printer size={16} aria-hidden="true" />
              {m.report_print()}
            </Button>
          </div>
        }
      />
      <div className="flex flex-col gap-6">
        <dl className="m-0 grid grid-cols-3 gap-2 md:gap-4">
          {[
            { label: m.report_cut(), value: total.cut, className: "text-cut" },
            { label: m.report_fill(), value: total.fill, className: "text-fill" },
            { label: m.report_net(), value: total.net, className: "" },
          ].map((k) => (
            <div
              key={k.label}
              className="flex min-w-0 flex-col gap-1 rounded-xl border border-border p-3 md:p-4"
            >
              <dt className="text-[13px] text-muted">{k.label}</dt>
              <dd className={`tabular m-0 text-[16px] font-semibold md:text-[22px] ${k.className}`}>
                {formatVolume(k.value)}
              </dd>
            </div>
          ))}
        </dl>
        {needsCoverageWarning([...rows, total]) ? (
          <p
            role="note"
            className="m-0 flex items-start gap-2 rounded-lg border border-warn px-4 py-3 text-[13px]"
          >
            <TriangleAlert size={16} aria-hidden="true" className="mt-0.5 shrink-0 text-warn" />
            {m.report_coverage_warning()}
          </p>
        ) : null}
        <Section title={m.report_by_region()}>
          <VolumeTable rows={rows} total={total} />
          <p className="m-0 text-[12px] text-muted">
            {m.report_method({ cm: Math.round(result.data.cell * 100) })}
          </p>
        </Section>
      </div>
    </>
  );
}
