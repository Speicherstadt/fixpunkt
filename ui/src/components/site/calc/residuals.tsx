/** Güte einer Passpunkt-Transformation: Kennzahlen und Restklaffen je Punkt (Vorschau und Ergebnis) */
import { TriangleAlert } from "lucide-react";
import { cm } from "@/components/report/compare-report";
import { Kpis } from "@/components/report/kpis";
import { formatNumber as num } from "@/lib/format";
import { type FitReport, isOutlier } from "@/lib/georef";
import { m } from "@/paraglide/messages";

export function Residuals({ report }: { report: FitReport }) {
  return (
    <div className="flex flex-col gap-3">
      <Kpis
        items={[
          { label: m.georef_rms_xy(), value: cm(report.rms_xy) },
          { label: m.georef_rms_z(), value: cm(report.rms_z) },
          { label: m.georef_max(), value: cm(report.max_3d) },
          { label: m.georef_scale(), value: `${num(report.scale_ppm, 0)} ppm` },
        ]}
      />
      <p className="m-0 text-[12px] text-muted">
        {m.georef_shift({
          x: num(report.translation[0], 3),
          y: num(report.translation[1], 3),
          z: num(report.translation[2], 3),
        })}
      </p>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[13px]">
          <caption className="sr-only">{m.georef_residuals()}</caption>
          <thead>
            <tr className="text-left text-[12px] text-muted">
              <th className="py-1.5 pr-3 font-medium">{m.georef_name()}</th>
              <th className="py-1.5 pr-3 text-right font-medium">Δx</th>
              <th className="py-1.5 pr-3 text-right font-medium">Δy</th>
              <th className="py-1.5 pr-3 text-right font-medium">Δz</th>
              <th className="py-1.5 text-right font-medium">{m.georef_d3()}</th>
            </tr>
          </thead>
          <tbody>
            {report.residuals.map((r) => {
              const bad = isOutlier(r.d3, report.rms_3d);
              return (
                <tr key={r.name} className="border-t border-border">
                  <td className="py-1.5 pr-3">
                    <span className="inline-flex items-center gap-1.5">
                      {r.name}
                      {bad ? (
                        <TriangleAlert
                          size={14}
                          className="text-warn"
                          aria-label={m.georef_outlier()}
                        />
                      ) : null}
                    </span>
                  </td>
                  <td className="tabular py-1.5 pr-3 text-right">{cm(r.dx)}</td>
                  <td className="tabular py-1.5 pr-3 text-right">{cm(r.dy)}</td>
                  <td className="tabular py-1.5 pr-3 text-right">{cm(r.dz)}</td>
                  <td className={`tabular py-1.5 text-right ${bad ? "text-warn" : ""}`}>
                    {cm(r.d3)}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
