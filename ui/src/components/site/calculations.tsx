import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Calculator, ChevronRight } from "lucide-react";
import { type FormEvent, useState } from "react";
import { EmptyState, ErrorText, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Label, Select } from "@/components/ui/input";
import { StatusBadge } from "@/components/ui/status";
import { api, type Capture, VolumeResult } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { formatDateTime, formatVolume } from "@/lib/format";
import { capturesQuery, jobsQuery, regionsQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";

export const CELL_SIZES = [0.1, 0.25, 0.5, 1] as const;

/** Name einer Aufnahme für die Liste der Berechnungen; gelöschte zeigen einen Strich */
export const nameOf = (captures: Capture[] | undefined, id: unknown): string =>
  captures?.find((c) => c.id === id)?.name ?? "–";

/** Vorauswahl: älteste Aufnahme als Urgelände, jüngste als Stand nach dem Aushub */
export const defaultPair = (ready: Capture[]): [number | undefined, number | undefined] => {
  const sorted = [...ready].sort((a, b) => a.capturedAt - b.capturedAt);
  return [sorted[0]?.id, sorted.length > 1 ? sorted[sorted.length - 1]?.id : undefined];
};

export function Calculations({ siteId, canEdit }: { siteId: number; canEdit: boolean }) {
  const captures = useQuery(capturesQuery(siteId));
  const regions = useQuery(regionsQuery(siteId));
  const jobs = useQuery(jobsQuery(siteId));
  const ready = (captures.data ?? []).filter((c) => c.status === "ready");
  const [first, last] = defaultPair(ready);
  const [before, setBefore] = useState<number | undefined>();
  const [after, setAfter] = useState<number | undefined>();
  const [cell, setCell] = useState("0.25");
  const [filterGround, setFilterGround] = useState(true);
  const [regionIds, setRegionIds] = useState<number[] | null>(null);
  const chosenRegions = regionIds ?? (regions.data ?? []).map((r) => r.id);
  const queryClient = useQueryClient();
  const b = before ?? first;
  const a = after ?? last;

  const start = useMutation({
    mutationFn: () =>
      api.createJob(siteId, "volume", {
        before: b,
        after: a,
        regionIds: chosenRegions,
        surface: { cell: Number(cell), classifyGround: filterGround },
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["jobs", siteId] }),
  });

  const submit = (e: FormEvent) => {
    e.preventDefault();
    start.mutate();
  };
  const toggleRegion = (id: number) =>
    setRegionIds(
      chosenRegions.includes(id) ? chosenRegions.filter((r) => r !== id) : [...chosenRegions, id],
    );

  return (
    <Section title={m.calc_title()} description={m.calc_text()}>
      {canEdit ? (
        ready.length < 2 ? (
          <EmptyState>{m.calc_need_two()}</EmptyState>
        ) : (
          <form onSubmit={submit} className="flex flex-col gap-4">
            <div className="grid gap-4 md:grid-cols-2">
              <Label>
                {m.calc_before()}
                <Select value={b ?? ""} onChange={(e) => setBefore(Number(e.target.value))}>
                  {ready.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </Select>
              </Label>
              <Label>
                {m.calc_after()}
                <Select value={a ?? ""} onChange={(e) => setAfter(Number(e.target.value))}>
                  {ready.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </Select>
              </Label>
              <Label>
                {m.calc_cell()}
                <Select value={cell} onChange={(e) => setCell(e.target.value)}>
                  {CELL_SIZES.map((s) => (
                    <option key={s} value={String(s)}>
                      {m.calc_cell_option({ cm: Math.round(s * 100) })}
                    </option>
                  ))}
                </Select>
              </Label>
              <label className="flex items-center gap-2 self-end pb-2.5 text-[14px]">
                <input
                  type="checkbox"
                  className="size-5 accent-fg"
                  checked={filterGround}
                  onChange={(e) => setFilterGround(e.target.checked)}
                />
                {m.calc_filter_ground()}
              </label>
            </div>
            {regions.data?.length ? (
              <fieldset className="m-0 flex flex-col gap-2 border-0 p-0">
                <legend className="mb-2 p-0 text-[13px] text-muted">{m.calc_regions()}</legend>
                {regions.data.map((r) => (
                  <label key={r.id} className="flex items-center gap-2 text-[14px]">
                    <input
                      type="checkbox"
                      className="size-5 accent-fg"
                      checked={chosenRegions.includes(r.id)}
                      onChange={() => toggleRegion(r.id)}
                    />
                    {r.name}
                  </label>
                ))}
              </fieldset>
            ) : (
              <p className="m-0 text-[13px] text-muted">{m.calc_no_regions()}</p>
            )}
            <ErrorText>{start.error ? errorText(start.error) : null}</ErrorText>
            <div className="flex justify-end">
              <Button type="submit" variant="primary" disabled={start.isPending || b === a}>
                <Calculator size={16} aria-hidden="true" />
                {m.calc_start()}
              </Button>
            </div>
            {b === a ? (
              <p className="m-0 text-right text-[13px] text-muted">{m.calc_same()}</p>
            ) : null}
          </form>
        )
      ) : null}
      {jobs.data?.length ? (
        <ul className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0">
          {jobs.data.map((j) => {
            const result = VolumeResult.safeParse(j.result);
            return (
              <li key={j.id}>
                <Link
                  to="/jobs/$jobId"
                  params={{ jobId: String(j.id) }}
                  className="flex items-center gap-3 px-4 py-3 hover:bg-hover"
                >
                  <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                    <span className="font-medium">
                      {m.report_compare({
                        before: nameOf(captures.data, j.params.before),
                        after: nameOf(captures.data, j.params.after),
                      })}
                    </span>
                    <span className="text-[12px] text-muted">{formatDateTime(j.createdAt)}</span>
                  </div>
                  {result.success ? (
                    <span className="tabular text-[14px]">
                      {m.calc_cut_short({ value: formatVolume(result.data.total.cut_m3) })}
                    </span>
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
