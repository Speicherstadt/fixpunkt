/** Aufnahmen hochladen, Drohnenfotos auswerten, Flug planen */
import { createFileRoute } from "@tanstack/react-router";
import { Captures } from "@/components/site/captures";
import { DronePhotos } from "@/components/site/drone-photos";
import { FlightPlanning } from "@/components/site/flight-plan";
import { useSite } from "@/components/site/use-site";

export const Route = createFileRoute("/_app/sites/$siteId/aufnahmen")({ component: AufnahmenTab });

function AufnahmenTab() {
  const { siteId, canEdit } = useSite();
  return (
    <>
      <Captures siteId={siteId} canEdit={canEdit} />
      {canEdit ? <DronePhotos siteId={siteId} /> : null}
      <FlightPlanning siteId={siteId} />
    </>
  );
}
