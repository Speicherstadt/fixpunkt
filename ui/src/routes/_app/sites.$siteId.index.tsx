/** Überblick: Kennzahlen, Fortschritt, Massenziele, Lagerbestand */
import { createFileRoute } from "@tanstack/react-router";
import { Overview } from "@/components/site/overview";
import { useSite } from "@/components/site/use-site";

export const Route = createFileRoute("/_app/sites/$siteId/")({ component: IndexTab });

function IndexTab() {
  const { siteId } = useSite();
  return <Overview siteId={siteId} />;
}
