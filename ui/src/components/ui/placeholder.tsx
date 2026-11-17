import { cn } from "@/lib/cn";
import { m } from "@/paraglide/messages";

/** Graue Balken an der Stelle der Zeilen, die noch laden. Vorbild: Kontext placeholder.tsx */
export const Bar = ({ className }: { className?: string }) => (
  <span className={cn("block h-2.5 rounded-full bg-subtle motion-safe:animate-pulse", className)} />
);

const WIDTHS = ["w-2/5", "w-1/3", "w-1/2", "w-3/5"] as const;

export const PlaceholderRows = ({ count = 4 }: { count?: number }) => (
  <div role="status" className="flex flex-col gap-4 py-2">
    <span className="sr-only">{m.common_loading()}</span>
    {Array.from({ length: count }, (_, index) => (
      // biome-ignore lint/suspicious/noArrayIndexKey: feste Zeilen ohne Identität
      <div key={index} aria-hidden="true" className="flex items-center justify-between gap-3">
        <Bar className={WIDTHS[index % WIDTHS.length]} />
        <Bar className="w-16" />
      </div>
    ))}
  </div>
);
