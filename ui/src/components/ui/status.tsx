import { CircleAlert, CircleCheck, Clock, LoaderCircle } from "lucide-react";
import { cn } from "@/lib/cn";
import { m } from "@/paraglide/messages";

export type Status = "uploaded" | "queued" | "running" | "ready" | "done" | "failed";

const LOOK: Record<
  Status,
  { icon: typeof Clock; className: string; text: () => string; spin?: boolean }
> = {
  uploaded: { icon: LoaderCircle, className: "text-muted", text: m.status_checking, spin: true },
  queued: { icon: Clock, className: "text-muted", text: m.status_queued },
  running: { icon: LoaderCircle, className: "text-accent", text: m.status_running, spin: true },
  ready: { icon: CircleCheck, className: "text-ok", text: m.status_ready },
  done: { icon: CircleCheck, className: "text-ok", text: m.status_done },
  failed: { icon: CircleAlert, className: "text-danger", text: m.status_failed },
};

/** Status mit Symbol und Wort – Farbe allein trägt nie die Bedeutung */
export const StatusBadge = ({ status }: { status: string }) => {
  const look = LOOK[status as Status] ?? LOOK.queued;
  const Icon = look.icon;
  return (
    <span className={cn("inline-flex items-center gap-1.5 text-[13px]", look.className)}>
      <Icon
        size={14}
        aria-hidden="true"
        className={look.spin ? "motion-safe:animate-spin" : undefined}
      />
      {look.text()}
    </span>
  );
};
