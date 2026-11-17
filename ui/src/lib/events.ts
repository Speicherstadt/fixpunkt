import type { QueryClient } from "@tanstack/react-query";
import { z } from "zod";

const ServerEvent = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("changed"),
    topic: z.enum(["sites", "captures", "regions", "jobs", "users", "markers", "plans"]),
    siteId: z.number().nullable().optional(),
  }),
  z.object({ type: z.literal("job"), id: z.number(), siteId: z.number(), status: z.string() }),
]);
type ServerEvent = z.infer<typeof ServerEvent>;

/** Welche Abfragen ein Ereignis veraltet macht */
export const staleKeys = (e: ServerEvent): unknown[][] => {
  if (e.type === "job")
    return [["job", e.id], ["jobs", e.siteId], ["dashboard", e.siteId], ["sites"]];
  const site = e.siteId ?? undefined;
  switch (e.topic) {
    case "sites":
      return [["sites"], ["site"]];
    case "users":
      return [["users"]];
    default:
      // alles, was zu einer Baustelle gehört, steht auch im Überblick
      return [
        [e.topic, ...(site === undefined ? [] : [site])],
        ["dashboard", ...(site === undefined ? [] : [site])],
        ["sites"],
      ];
  }
};

/** Live-Updates vom Server: Listen neu laden, sobald sich etwas ändert (auch von anderen Nutzern) */
export const connectEvents = (queryClient: QueryClient): (() => void) => {
  const source = new EventSource("/api/events");
  source.onmessage = (msg) => {
    const parsed = ServerEvent.safeParse(JSON.parse(msg.data));
    if (!parsed.success) return;
    for (const queryKey of staleKeys(parsed.data)) queryClient.invalidateQueries({ queryKey });
  };
  // Zu langsam gelesen: alles neu laden
  source.addEventListener("lagged", () => queryClient.invalidateQueries());
  return () => source.close();
};
