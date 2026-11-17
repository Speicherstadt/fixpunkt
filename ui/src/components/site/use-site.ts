/** Baustelle der aktuellen Seite und ob der Benutzer ändern darf (für die Reiter unter /sites/$siteId) */
import { useQuery } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";
import { authQuery } from "@/lib/queries";

export const useSite = () => {
  const siteId = Number(useParams({ from: "/_app/sites/$siteId" }).siteId);
  const { data: auth } = useQuery(authQuery);
  return { siteId, canEdit: auth?.user?.role !== "viewer" };
};
