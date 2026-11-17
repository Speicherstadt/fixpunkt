import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowLeft, Download, FileSpreadsheet, FileText, Printer } from "lucide-react";
import type { ReactNode } from "react";
import { ErrorText, PageHeader, Section } from "@/components/page";
import { CompareReport } from "@/components/report/compare-report";
import { FilesReport } from "@/components/report/files-report";
import { fileHref } from "@/components/report/kpis";
import { OdmReport } from "@/components/report/odm-report";
import { SeriesReport } from "@/components/report/series-report";
import { VolumeReport } from "@/components/report/volume-report";
import { Residuals } from "@/components/site/calc/residuals";
import { nameOf } from "@/components/site/calculations";
import { Button, buttonVariants } from "@/components/ui/button";
import { PlaceholderRows } from "@/components/ui/placeholder";
import { StatusBadge } from "@/components/ui/status";
import {
  CompareResult,
  FilesResult,
  type Job,
  OdmResult,
  SeriesResult,
  VolumeResult,
} from "@/lib/api";
import { errorText } from "@/lib/errors";
import { formatDateTime } from "@/lib/format";
import { GeorefResult } from "@/lib/georef";
import { jobTitle, reportHref } from "@/lib/jobs";
import { capturesQuery, jobQuery, siteQuery } from "@/lib/queries";
import { reportCsv, reportRows } from "@/lib/report";
import { m } from "@/paraglide/messages";

export const Route = createFileRoute("/_app/jobs/$jobId")({ component: JobPage });

/** Ergebnis je Art; null, solange der Auftrag kein passendes Ergebnis hat */
function resultView(job: Job): ReactNode {
  switch (job.kind) {
    case "volume":
    case "stockpile": {
      const r = VolumeResult.safeParse(job.result);
      return r.success ? (
        <VolumeReport result={r.data} stockpile={job.kind === "stockpile"} />
      ) : null;
    }
    case "compare": {
      const r = CompareResult.safeParse(job.result);
      return r.success ? <CompareReport result={r.data} /> : null;
    }
    case "series": {
      const r = SeriesResult.safeParse(job.result);
      return r.success ? <SeriesReport result={r.data} /> : null;
    }
    case "georef": {
      const r = GeorefResult.safeParse(job.result);
      return r.success ? (
        <div className="flex flex-col gap-4">
          <Residuals report={r.data} />
          <div>
            <Link
              to="/sites/$siteId/aufnahmen"
              params={{ siteId: String(job.siteId) }}
              className={buttonVariants({ variant: "primary" })}
            >
              {m.odm_result_capture()}
            </Link>
          </div>
        </div>
      ) : null;
    }
    case "odm": {
      const r = OdmResult.safeParse(job.result);
      return r.success ? <OdmReport result={r.data} siteId={job.siteId} /> : null;
    }
    default: {
      const r = FilesResult.safeParse(job.result);
      return r.success ? <FilesReport jobId={job.id} files={r.data.files} /> : null;
    }
  }
}

/** CSV-Download für Massenberichte */
function csvLink(job: Job) {
  const r = VolumeResult.safeParse(job.result);
  if (!r.success) return null;
  const { rows, total } = reportRows(r.data.regions, r.data.total, m.report_total());
  const csv = reportCsv(rows, total, [
    m.report_region(),
    m.report_area(),
    m.report_cut(),
    m.report_fill(),
    m.report_net(),
  ]);
  return (
    <a
      href={`data:text/csv;charset=utf-8,${encodeURIComponent(`﻿${csv}`)}`}
      download={`mengen-${job.id}.csv`}
      className={buttonVariants()}
    >
      <FileSpreadsheet size={16} aria-hidden="true" />
      {m.report_csv()}
    </a>
  );
}

/** Raster-Download (Differenz, Abweichung) für Berichte, die eines schreiben */
function pdfLink(job: Job) {
  const href = reportHref(job);
  if (!href) return null;
  return (
    <a href={href} download className={buttonVariants({ variant: "primary" })}>
      <FileText size={16} aria-hidden="true" />
      {m.report_pdf()}
    </a>
  );
}

function rasterLink(job: Job) {
  const files = FilesResult.safeParse(job.result);
  const raster = files.success
    ? files.data.files.find((f) => f === "diff.tif" || f === "abweichung.tif")
    : undefined;
  if (!raster || job.kind === "export" || job.kind === "dem") return null;
  return (
    <a href={fileHref(job.id, raster)} className={buttonVariants()}>
      <Download size={16} aria-hidden="true" />
      {raster === "diff.tif" ? m.report_diff() : m.report_deviation()}
    </a>
  );
}

function JobPage() {
  const jobId = Number(Route.useParams().jobId);
  const job = useQuery(jobQuery(jobId));
  const siteId = job.data?.siteId ?? 0;
  const site = useQuery({ ...siteQuery(siteId), enabled: siteId > 0 });
  const captures = useQuery({ ...capturesQuery(siteId), enabled: siteId > 0 });
  if (job.isPending) return <PlaceholderRows />;
  if (job.error) return <ErrorText>{errorText(job.error)}</ErrorText>;
  const j = job.data;
  const title = jobTitle(j, (id) => nameOf(captures.data, id));
  const view = resultView(j);
  const back = (
    <Link
      // Aufträge stehen unter „Massen“
      to="/sites/$siteId/massen"
      params={{ siteId: String(j.siteId) }}
      className={buttonVariants({ variant: "ghost" })}
    >
      <ArrowLeft size={16} aria-hidden="true" />
      {site.data?.name ?? m.nav_sites()}
    </Link>
  );

  if (!view) {
    return (
      <>
        <PageHeader title={title} actions={back} />
        <Section title={m.report_status()}>
          <StatusBadge status={j.status} />
          {j.error ? <ErrorText>{j.error}</ErrorText> : null}
          {j.status === "queued" || j.status === "running" ? (
            <p className="m-0 text-[13px] text-muted">{m.report_wait()}</p>
          ) : null}
          {j.status === "running" && j.progress != null ? (
            <progress
              value={j.progress}
              max={100}
              aria-label={m.job_progress({ percent: Math.round(j.progress) })}
              className="h-1.5 w-full accent-fg"
            />
          ) : null}
        </Section>
      </>
    );
  }

  return (
    <>
      <PageHeader
        title={j.kind === "volume" ? m.report_title() : title}
        subtitle={`${site.data?.name ?? ""} · ${j.kind === "volume" ? `${title} · ` : ""}${formatDateTime(j.finishedAt ?? j.createdAt)}`}
        actions={
          <div className="flex flex-wrap gap-2 print:hidden">
            {back}
            {pdfLink(j)}
            {csvLink(j)}
            {rasterLink(j)}
            <Button onClick={() => window.print()}>
              <Printer size={16} aria-hidden="true" />
              {m.report_print()}
            </Button>
          </div>
        }
      />
      {view}
    </>
  );
}
