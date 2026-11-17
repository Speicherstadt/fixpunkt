/** Ansicht: 2D, 3D, Messen, Bereiche zeichnen */
import { createFileRoute } from "@tanstack/react-router";
import { useSite } from "@/components/site/use-site";
import { SiteView } from "@/components/view/site-view";

export const Route = createFileRoute("/_app/sites/$siteId/ansicht")({ component: AnsichtTab });

function AnsichtTab() {
  const { siteId, canEdit } = useSite();
  return <SiteView siteId={siteId} canEdit={canEdit} />;
}
