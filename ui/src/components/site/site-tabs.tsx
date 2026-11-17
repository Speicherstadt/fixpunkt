/** Reiter einer Baustelle: jede Seite hat eine eigene Adresse (Zurück-Taste, Lesezeichen, Links) */
import { Link } from "@tanstack/react-router";
import { m } from "@/paraglide/messages";

export const SITE_TABS = [
  { to: "/sites/$siteId", label: m.site_tab_overview, exact: true },
  { to: "/sites/$siteId/aufnahmen", label: m.site_tab_captures, exact: false },
  { to: "/sites/$siteId/ansicht", label: m.site_tab_view, exact: false },
  { to: "/sites/$siteId/massen", label: m.site_tab_volumes, exact: false },
  { to: "/sites/$siteId/abrechnung", label: m.site_tab_billing, exact: false },
  { to: "/sites/$siteId/dokumentation", label: m.site_tab_docs, exact: false },
] as const;

export function SiteTabs({ siteId }: { siteId: number }) {
  return (
    <nav aria-label={m.site_tabs()} className="-mx-1 mb-6 overflow-x-auto border-b border-border">
      <ul className="m-0 flex min-w-max list-none gap-1 p-0 px-1">
        {SITE_TABS.map((t) => (
          <li key={t.to}>
            <Link
              to={t.to}
              params={{ siteId: String(siteId) }}
              activeOptions={{ exact: t.exact }}
              className="-mb-px flex h-10 items-center border-b-2 border-transparent px-3 text-[14px] text-muted hover:text-fg [&.active]:border-fg [&.active]:font-medium [&.active]:text-fg"
            >
              {t.label()}
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  );
}
