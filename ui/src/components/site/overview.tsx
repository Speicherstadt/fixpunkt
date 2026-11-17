/**
 * Überblick einer Baustelle: Kennzahlen, Aushub je Befliegung, Massenziele, Lagerbestand und Verlauf, was
 * Aufmerksamkeit braucht. Welche Abschnitte in welcher Reihenfolge, stellt jeder Benutzer selbst ein (Anpassen).
 */
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { FileText, SlidersHorizontal } from "lucide-react";
import { useState } from "react";
import { EmptyState, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { dashboardQuery, layoutQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";
import { LayoutEditor } from "./overview-layout";
import { sectionContent } from "./overview-sections";
import { ReportBuilder } from "./report-builder";

export function Overview({ siteId }: { siteId: number }) {
  const { data: d } = useQuery(dashboardQuery(siteId));
  const { data: layout } = useQuery(layoutQuery(siteId));
  const [editing, setEditing] = useState(false);
  const [report, setReport] = useState(false);
  if (!d) return null;
  if (d.captures.count === 0)
    return (
      <Section title={m.overview_title()}>
        <EmptyState>
          {m.overview_empty()}{" "}
          <Link
            to="/sites/$siteId/aufnahmen"
            params={{ siteId: String(siteId) }}
            className="text-fg underline"
          >
            {m.overview_empty_link()}
          </Link>
        </EmptyState>
      </Section>
    );
  const sections = layout?.sections ?? [];
  return (
    <Section
      title={m.overview_title()}
      actions={
        <>
          <Button size="sm" onClick={() => setReport(true)}>
            <FileText size={14} aria-hidden="true" />
            {m.compose_open()}
          </Button>
          <Button size="sm" aria-expanded={editing} onClick={() => setEditing(!editing)}>
            <SlidersHorizontal size={14} aria-hidden="true" />
            {m.overview_customize()}
          </Button>
        </>
      }
    >
      <ReportBuilder siteId={siteId} open={report} onOpenChange={setReport} />
      {editing && layout ? (
        <LayoutEditor siteId={siteId} layout={layout} onDone={() => setEditing(false)} />
      ) : null}
      {sections
        .filter((s) => s.visible)
        .map((s) => {
          const content = sectionContent(s.id, d);
          return content ? (
            <div key={s.id} className="contents">
              {content}
            </div>
          ) : null;
        })}
    </Section>
  );
}
