import { cn } from "@/lib/cn";

export type Kpi = { label: string; value: string; className?: string };

/** Kennzahlen nebeneinander, am Handy kompakt */
export const Kpis = ({ items }: { items: Kpi[] }) => (
  <dl
    className={cn(
      "m-0 grid gap-2 md:gap-4",
      items.length === 3 ? "grid-cols-3" : "grid-cols-2 md:grid-cols-4",
    )}
  >
    {items.map((k) => (
      <div
        key={k.label}
        className="flex min-w-0 flex-col gap-1 rounded-xl border border-border p-3 md:p-4"
      >
        <dt className="text-[13px] text-muted">{k.label}</dt>
        <dd className={cn("tabular m-0 text-[16px] font-semibold md:text-[22px]", k.className)}>
          {k.value}
        </dd>
      </div>
    ))}
  </dl>
);

/** Downloads eines Auftrags */
export const fileHref = (jobId: number, name: string) => `/api/jobs/${jobId}/files/${name}`;
