import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Calculator, ChevronRight } from "lucide-react";
import { useState } from "react";
import { ErrorText, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { StatusBadge } from "@/components/ui/status";
import { api, type Capture } from "@/lib/api";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { formatDateTime } from "@/lib/format";
import { JOB_KINDS, type JobKind, jobSummary, jobTitle, KIND_TEXT } from "@/lib/jobs";
import { capturesQuery, jobsQuery, regionsQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";
import {
  CompareForm,
  ExportForm,
  type FormProps,
  SeriesForm,
  StockpileForm,
  VolumeForm,
} from "./calc/forms";

export { defaultPair } from "./calc/forms";

/** Name einer Aufnahme für die Liste der Berechnungen; gelöschte zeigen einen Strich */
export const nameOf = (captures: Capture[] | undefined, id: unknown): string =>
  captures?.find((c) => c.id === id)?.name ?? "–";

const FORMS: Record<JobKind, (p: FormProps) => React.JSX.Element> = {
  volume: VolumeForm,
  stockpile: StockpileForm,
  compare: CompareForm,
  series: SeriesForm,
  export: ExportForm,
};

const HINTS: Record<JobKind, () => string> = {
  volume: m.calc_text,
  stockpile: m.calc_text_stockpile,
  compare: m.calc_text_compare,
  series: m.calc_text_series,
  export: m.calc_text_export,
};

const START: Record<JobKind, () => string> = {
  volume: m.calc_start,
  stockpile: m.calc_start_stockpile,
  compare: m.calc_start_compare,
  series: m.calc_start_series,
  export: m.calc_start_export,
};

export function Calculations({ siteId, canEdit }: { siteId: number; canEdit: boolean }) {
  const captures = useQuery(capturesQuery(siteId));
  const regions = useQuery(regionsQuery(siteId));
  const jobs = useQuery(jobsQuery(siteId));
  const ready = (captures.data ?? []).filter((c) => c.status === "ready");
  const [kind, setKind] = useState<JobKind>("volume");
  const queryClient = useQueryClient();
  const start = useMutation({
    mutationFn: (params: Record<string, unknown>) => api.createJob(siteId, kind, params),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["jobs", siteId] }),
  });
  const Form = FORMS[kind];
  const names = (id: unknown) => nameOf(captures.data, id);

  return (
    <Section title={m.calc_title()} description={HINTS[kind]()}>
      {canEdit ? (
        <>
          <fieldset className="m-0 flex flex-wrap gap-1 rounded-lg bg-subtle p-1 border-0">
            <legend className="sr-only">{m.calc_kind()}</legend>
            {JOB_KINDS.map((k) => (
              <label
                key={k}
                className={cn(
                  "flex h-8 cursor-pointer items-center rounded-md px-3 text-[13px] text-muted has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-fg",
                  k === kind && "bg-bg font-medium text-fg shadow-sm",
                )}
              >
                <input
                  type="radio"
                  name="kind"
                  value={k}
                  checked={k === kind}
                  onChange={() => {
                    setKind(k);
                    start.reset();
                  }}
                  className="sr-only"
                />
                {KIND_TEXT[k]()}
              </label>
            ))}
          </fieldset>
          <Form
            key={kind}
            ready={ready}
            regions={regions.data ?? []}
            onSubmit={(params) => start.mutate(params)}
            footer={(reason) => (
              <div className="flex flex-col gap-2">
                <ErrorText>{start.error ? errorText(start.error) : null}</ErrorText>
                <div className="flex items-center justify-end gap-3">
                  {reason ? <span className="text-[13px] text-muted">{reason}</span> : null}
                  <Button
                    type="submit"
                    variant="primary"
                    disabled={start.isPending || reason !== null}
                  >
                    <Calculator size={16} aria-hidden="true" />
                    {START[kind]()}
                  </Button>
                </div>
              </div>
            )}
          />
        </>
      ) : null}
      {jobs.data?.length ? (
        <ul className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0">
          {jobs.data.map((j) => {
            const summary = jobSummary(j);
            return (
              <li key={j.id}>
                <Link
                  to="/jobs/$jobId"
                  params={{ jobId: String(j.id) }}
                  className="flex items-center gap-3 px-4 py-3 hover:bg-hover"
                >
                  <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                    <span className="truncate font-medium">{jobTitle(j, names)}</span>
                    <span className="text-[12px] text-muted">{formatDateTime(j.createdAt)}</span>
                  </div>
                  {summary ? (
                    <span className="tabular hidden text-[14px] sm:inline">{summary}</span>
                  ) : null}
                  <StatusBadge status={j.status} />
                  <ChevronRight size={16} aria-hidden="true" className="text-muted" />
                </Link>
              </li>
            );
          })}
        </ul>
      ) : null}
    </Section>
  );
}
