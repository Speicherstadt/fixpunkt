import { Download } from "lucide-react";
import { Section } from "@/components/page";
import { buttonVariants } from "@/components/ui/button";
import { m } from "@/paraglide/messages";
import { fileHref } from "./kpis";

const FILE_TEXT: Record<string, () => string> = {
  "dgm.tif": m.export_tif,
  "dem.tif": m.export_tif,
  "dgm.dxf": m.export_dxf,
  "dgm.xml": m.export_landxml,
  "dgm.csv": m.export_csv,
  "dgm.obj": m.export_obj,
};

/** Ergebnisdateien zum Herunterladen (Export, Höhenmodell) */
export function FilesReport({ jobId, files }: { jobId: number; files: string[] }) {
  return (
    <Section title={m.files_title()} description={m.files_text()}>
      <ul className="m-0 flex list-none flex-col divide-y divide-border p-0">
        {files.map((f) => (
          <li key={f} className="flex items-center justify-between gap-3 py-2.5">
            <span className="flex flex-col">
              <span className="font-medium">{(FILE_TEXT[f] ?? (() => f))()}</span>
              <span className="text-[12px] text-muted">{f}</span>
            </span>
            <a href={fileHref(jobId, f)} download className={buttonVariants({ size: "sm" })}>
              <Download size={14} aria-hidden="true" />
              {m.files_download()}
            </a>
          </li>
        ))}
      </ul>
    </Section>
  );
}
