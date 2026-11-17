import { Link } from "@tanstack/react-router";
import { ArrowRight } from "lucide-react";
import { buttonVariants } from "@/components/ui/button";
import type { OdmResult } from "@/lib/api";
import { formatCount, formatDuration } from "@/lib/format";
import { m } from "@/paraglide/messages";
import { Kpis } from "./kpis";

/** Ergebnis der Photogrammetrie: Fotos, Rechenzeit, Bezugssystem, Link zur neuen Aufnahme */
export function OdmReport({ result, siteId }: { result: OdmResult; siteId: number }) {
  return (
    <div className="flex flex-col gap-4">
      <Kpis
        items={[
          { label: m.odm_result_images(), value: formatCount(result.images) },
          { label: m.odm_result_time(), value: formatDuration(result.processingMs) },
          {
            label: m.odm_result_crs(),
            value: result.sourceEpsg ? `EPSG:${result.sourceEpsg}` : "–",
          },
        ]}
      />
      <p className="m-0 text-[13px] text-muted">
        {result.converted
          ? m.odm_result_converted({ offset: String(result.heightOffset).replace(".", ",") })
          : m.odm_result_unchanged()}
      </p>
      <div>
        <Link
          to="/sites/$siteId/aufnahmen"
          params={{ siteId: String(siteId) }}
          className={buttonVariants({ variant: "primary" })}
        >
          {m.odm_result_capture()}
          <ArrowRight size={16} aria-hidden="true" />
        </Link>
      </div>
    </div>
  );
}
