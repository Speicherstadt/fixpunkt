/** Massen: Abrechnungsbereiche und Berechnungen */
import { createFileRoute } from "@tanstack/react-router";
import { Calculations } from "@/components/site/calculations";
import { Regions } from "@/components/site/regions";
import { useSite } from "@/components/site/use-site";

export const Route = createFileRoute("/_app/sites/$siteId/massen")({ component: MassenTab });

function MassenTab() {
  const { siteId, canEdit } = useSite();
  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_2fr]">
      <Regions siteId={siteId} canEdit={canEdit} />
      <Calculations siteId={siteId} canEdit={canEdit} />
    </div>
  );
}
