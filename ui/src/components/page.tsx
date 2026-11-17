import { type ReactNode, useId } from "react";
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

/** Karte mit Überschrift */
export const Section = ({
  title,
  description,
  actions,
  children,
  className,
}: {
  title: string;
  description?: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}) => {
  const id = useId();
  return (
    <section
      aria-labelledby={id}
      className={cn("flex flex-col gap-4 rounded-xl border border-border p-5", className)}
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex flex-col gap-1">
          <h2 id={id} className="m-0 text-[16px] font-semibold">
            {title}
          </h2>
          {description ? (
            <p className="m-0 max-w-prose text-[13px] text-muted">{description}</p>
          ) : null}
        </div>
        {actions ? <div className="flex flex-wrap gap-2">{actions}</div> : null}
      </div>
      {children}
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
