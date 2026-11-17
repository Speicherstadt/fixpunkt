/** Abrechnung: Aufmaß, Schichten, Lieferscheine, Freigaben */
import { createFileRoute } from "@tanstack/react-router";
import { Billing } from "@/components/site/billing";
import { useSite } from "@/components/site/use-site";

export const Route = createFileRoute("/_app/sites/$siteId/abrechnung")({
  component: AbrechnungTab,
});

function AbrechnungTab() {
  const { siteId, canEdit } = useSite();
  return <Billing siteId={siteId} canEdit={canEdit} />;
}
