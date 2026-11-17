import { ChevronDown, ChevronRight } from "lucide-react";
import { type ReactNode, useId, useState } from "react";
import { cn } from "@/lib/cn";

/** Seitentitel (genau ein h1 je Seite) mit Aktionen rechts */
export const PageHeader = ({
  title,
  subtitle,
  actions,
}: {
  title: string;
  subtitle?: ReactNode;
  actions?: ReactNode;
}) => (
  <header className="flex flex-wrap items-end justify-between gap-3 pb-6">
    <div className="flex min-w-0 flex-col gap-1">
      <h1 className="m-0 text-[22px] font-semibold">{title}</h1>
      {subtitle ? <div className="text-[13px] text-muted">{subtitle}</div> : null}
    </div>
    {actions ? <div className="flex flex-wrap gap-2">{actions}</div> : null}
  </header>
);

/**
 * Karte mit Überschrift. `collapsible`: die Überschrift klappt den Inhalt auf und zu; zugeklappt bleibt er
 * eingehängt (ein laufender Upload geht nicht verloren), nur unsichtbar.
 */
export const Section = ({
  title,
  description,
  actions,
  children,
  className,
  collapsible = false,
  defaultOpen = false,
}: {
  title: string;
  description?: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  collapsible?: boolean;
  defaultOpen?: boolean;
}) => {
  const id = useId();
  const [open, setOpen] = useState(defaultOpen);
  const shown = !collapsible || open;
  return (
    <section
      aria-labelledby={id}
      className={cn("flex flex-col gap-4 rounded-xl border border-border p-5", className)}
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex flex-col gap-1">
          <h2 id={id} className="m-0 text-[16px] font-semibold">
            {collapsible ? (
              <button
                type="button"
                aria-expanded={open}
                onClick={() => setOpen(!open)}
                className="-mx-1 inline-flex items-center gap-1.5 rounded-md px-1 text-left hover:bg-hover"
              >
                {open ? (
                  <ChevronDown size={16} aria-hidden="true" />
                ) : (
                  <ChevronRight size={16} aria-hidden="true" />
                )}
                {title}
              </button>
            ) : (
              title
            )}
          </h2>
          {description ? (
            <p className="m-0 max-w-prose text-[13px] text-muted">{description}</p>
          ) : null}
        </div>
        {actions && shown ? <div className="flex flex-wrap gap-2">{actions}</div> : null}
      </div>
      {collapsible ? (
        <div hidden={!open} className="flex flex-col gap-4">
          {children}
        </div>
      ) : (
        children
      )}
    </section>
  );
};

export const EmptyState = ({ children }: { children: ReactNode }) => (
  <p className="m-0 rounded-lg bg-subtle px-4 py-6 text-center text-[13px] text-muted">
    {children}
  </p>
);

/** Fehler mit Text; immer als Alert vorgelesen */
export const ErrorText = ({ children }: { children: ReactNode }) =>
  children ? (
    <p role="alert" className="m-0 text-[13px] text-danger">
      {children}
    </p>
  ) : null;
