/** Dokumentation: Pläne, Zwangspunkte, Leitungen, Fotos, Notizen */
import { createFileRoute } from "@tanstack/react-router";
import { Documentation } from "@/components/site/documentation";
import { Plans } from "@/components/site/plans";
import { useSite } from "@/components/site/use-site";

export const Route = createFileRoute("/_app/sites/$siteId/dokumentation")({
  component: DokumentationTab,
});

function DokumentationTab() {
  const { siteId, canEdit } = useSite();
  return (
    <>
      <Plans siteId={siteId} canEdit={canEdit} />
      <Documentation siteId={siteId} canEdit={canEdit} />
    </>
  );
}
