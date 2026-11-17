import { queryOptions } from "@tanstack/react-query";
import { api, setCsrf } from "@/lib/api";

export const authQuery = queryOptions({
  queryKey: ["auth"],
  queryFn: async () => {
    const state = await api.authState();
    setCsrf(state.csrf);
    return state;
  },
  staleTime: 60_000,
});

export const sitesQuery = queryOptions({ queryKey: ["sites"], queryFn: api.sites });
export const siteQuery = (id: number) =>
  queryOptions({ queryKey: ["site", id], queryFn: () => api.site(id) });
export const capturesQuery = (siteId: number) =>
  queryOptions({ queryKey: ["captures", siteId], queryFn: () => api.captures(siteId) });
export const regionsQuery = (siteId: number) =>
  queryOptions({ queryKey: ["regions", siteId], queryFn: () => api.regions(siteId) });
export const jobsQuery = (siteId: number) =>
  queryOptions({ queryKey: ["jobs", siteId], queryFn: () => api.jobs(siteId) });
export const jobQuery = (id: number) =>
  queryOptions({ queryKey: ["job", id], queryFn: () => api.job(id) });
export const plansQuery = (siteId: number) =>
  queryOptions({ queryKey: ["plans", siteId], queryFn: () => api.plans(siteId) });
/** Geometrie eines Plans ändert sich nie (neue Fassung = neuer Plan) */
export const planGeometryQuery = (id: number) =>
  queryOptions({
    queryKey: ["plan-geometry", id],
    queryFn: () => api.planGeometry(id),
    staleTime: Number.POSITIVE_INFINITY,
  });
export const flightPlanQuery = (regionId: number, query: string) =>
  queryOptions({
    queryKey: ["flightplan", regionId, query],
    queryFn: () => api.flightPlan(regionId, query),
    placeholderData: (prev) => prev,
  });
export const layoutQuery = (siteId: number) =>
  queryOptions({ queryKey: ["layout", siteId], queryFn: () => api.layout(siteId) });
export const dashboardQuery = (siteId: number) =>
  queryOptions({ queryKey: ["dashboard", siteId], queryFn: () => api.dashboard(siteId) });
export const markersQuery = (siteId: number) =>
  queryOptions({ queryKey: ["markers", siteId], queryFn: () => api.markers(siteId) });
/** Einstellungen ändern sich nur mit einem Neustart des Servers */
export const settingsQuery = queryOptions({
  queryKey: ["settings"],
  queryFn: api.settings,
  staleTime: Number.POSITIVE_INFINITY,
});
export const tokensQuery = queryOptions({ queryKey: ["tokens"], queryFn: api.tokens });
export const passkeysQuery = queryOptions({ queryKey: ["passkeys"], queryFn: api.passkeys });
export const usersQuery = queryOptions({ queryKey: ["users"], queryFn: api.users });
