/**
 * Übersichtskarte aller Baustellen (Leaflet). Punkte führen zur Baustelle; die Liste darunter ist der
 * zugängliche Weg dorthin, die Karte zeigt nur, wo was liegt. Ohne Kachel-URL ein schlichter Hintergrund.
 */
import { useNavigate } from "@tanstack/react-router";
import L from "leaflet";
import "leaflet/dist/leaflet.css";
import { useEffect, useRef } from "react";
import type { Settings } from "@/lib/api";
import { bounds, type MapSite } from "@/lib/sites-map";
import { m } from "@/paraglide/messages";

export function SitesMap({ sites, tiles }: { sites: MapSite[]; tiles: Settings["mapTiles"] }) {
  const el = useRef<HTMLElement>(null);
  const navigate = useNavigate();

  useEffect(() => {
    const box = bounds(sites);
    if (!el.current || !box) return;
    const map = L.map(el.current, { attributionControl: false, scrollWheelZoom: false });
    if (tiles) {
      L.tileLayer(tiles.url, { maxZoom: 19 }).addTo(map);
      L.control.attribution({ prefix: false }).addAttribution(tiles.attribution).addTo(map);
    }
    for (const s of sites) {
      L.circleMarker([s.lat, s.lon], { radius: 9, className: "site-marker" })
        .bindTooltip(s.name, { direction: "top", offset: [0, -8] })
        .on("click", () => navigate({ to: "/sites/$siteId", params: { siteId: String(s.id) } }))
        .addTo(map);
    }
    map.fitBounds(box, { padding: [40, 40], maxZoom: 15 });
    return () => {
      map.remove();
    };
  }, [sites, tiles, navigate]);

  return (
    <section
      ref={el}
      aria-label={m.map_label()}
      className="sites-map h-72 w-full overflow-hidden rounded-xl border border-border bg-subtle md:h-96"
    />
  );
}
