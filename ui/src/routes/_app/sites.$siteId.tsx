import { useMutation, useQuery } from "@tanstack/react-query";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { Trash2 } from "lucide-react";
import { useState } from "react";
import { ErrorText, PageHeader } from "@/components/page";
import { Billing } from "@/components/site/billing";
import { Calculations } from "@/components/site/calculations";
import { Captures } from "@/components/site/captures";
import { Documentation } from "@/components/site/documentation";
import { DronePhotos } from "@/components/site/drone-photos";
import { FlightPlanning } from "@/components/site/flight-plan";
import { Overview } from "@/components/site/overview";
import { Plans } from "@/components/site/plans";
import { Regions } from "@/components/site/regions";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { PlaceholderRows } from "@/components/ui/placeholder";
import { SiteView } from "@/components/view/site-view";
import { api } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { authQuery, siteQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";

export const Route = createFileRoute("/_app/sites/$siteId")({ component: SitePage });

function SitePage() {
  const siteId = Number(Route.useParams().siteId);
  const site = useQuery(siteQuery(siteId));
  const { data: auth } = useQuery(authQuery);
  const canEdit = auth?.user?.role !== "viewer";
  const [confirm, setConfirm] = useState(false);
  const navigate = useNavigate();
  const remove = useMutation({
    mutationFn: () => api.deleteSite(siteId),
    onSuccess: () => navigate({ to: "/" }),
  });
  if (site.isPending) return <PlaceholderRows />;
  if (site.error) return <ErrorText>{errorText(site.error)}</ErrorText>;
  return (
    <>
      <PageHeader
        title={site.data.name}
        subtitle={site.data.description || m.site_epsg({ epsg: site.data.epsg })}
        actions={
          canEdit ? (
            <Button variant="danger" onClick={() => setConfirm(true)}>
              <Trash2 size={16} aria-hidden="true" />
              {m.site_delete()}
            </Button>
          ) : null
        }
      />
      <div className="flex flex-col gap-6">
        <Overview siteId={siteId} />
        <Captures siteId={siteId} canEdit={canEdit} />
        {canEdit ? <DronePhotos siteId={siteId} /> : null}
        <SiteView siteId={siteId} canEdit={canEdit} />
        <div className="grid gap-6 lg:grid-cols-[1fr_2fr]">
          <Regions siteId={siteId} canEdit={canEdit} />
          <Calculations siteId={siteId} canEdit={canEdit} />
        </div>
        <Billing siteId={siteId} canEdit={canEdit} />
        <FlightPlanning siteId={siteId} />
        <Plans siteId={siteId} canEdit={canEdit} />
        <Documentation siteId={siteId} canEdit={canEdit} />
      </div>
      <Dialog
        open={confirm}
        onOpenChange={setConfirm}
        title={m.site_delete_question({ name: site.data.name })}
        description={m.site_delete_text()}
      >
        <ErrorText>{remove.error ? errorText(remove.error) : null}</ErrorText>
        <div className="flex justify-end gap-2">
          <Button onClick={() => setConfirm(false)} autoFocus>
            {m.common_cancel()}
          </Button>
          <Button variant="danger" onClick={() => remove.mutate()} disabled={remove.isPending}>
            {m.site_delete_confirm()}
          </Button>
        </div>
      </Dialog>
    </>
  );
}
