import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { MapPinned, Target, Trash2 } from "lucide-react";
import { useId, useState } from "react";
import { EmptyState, ErrorText, Section } from "@/components/page";
import { Button, buttonVariants } from "@/components/ui/button";
import { api } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { formatArea } from "@/lib/format";
import { regionsQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";
import { TargetForm, targetText } from "./region-target";

export function Regions({ siteId, canEdit }: { siteId: number; canEdit: boolean }) {
  const regions = useQuery(regionsQuery(siteId));
  const queryClient = useQueryClient();
  const inputId = useId();
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["regions", siteId] });
  const add = useMutation({
    mutationFn: async (file: File) => api.addRegions(siteId, await file.text()),
    onSuccess: refresh,
  });
  const remove = useMutation({ mutationFn: api.deleteRegion, onSuccess: refresh });
  const [editing, setEditing] = useState<number | null>(null);
  return (
    <Section
      title={m.regions_title()}
      description={m.regions_text()}
      actions={
        canEdit ? (
          <>
            <input
              id={inputId}
              type="file"
              accept=".geojson,.json"
              className="sr-only"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) add.mutate(f);
                e.target.value = "";
              }}
            />
            <label htmlFor={inputId} className={buttonVariants()}>
              <MapPinned size={16} aria-hidden="true" />
              {m.regions_upload()}
            </label>
          </>
        ) : null
      }
    >
      <ErrorText>
        {add.error ? errorText(add.error) : remove.error ? errorText(remove.error) : null}
      </ErrorText>
      {regions.data?.length ? (
        <ul className="m-0 flex list-none flex-col divide-y divide-border p-0">
          {regions.data.map((r) => (
            <li key={r.id} className="flex flex-col gap-2 py-2">
              <div className="flex items-center justify-between gap-3">
                <span className="flex min-w-0 flex-col">
                  <span>{r.name}</span>
                  {targetText(r) ? (
                    <span className="text-[12px] text-muted">{targetText(r)}</span>
                  ) : null}
                </span>
                <span className="flex items-center gap-2">
                  <span className="tabular text-[13px] whitespace-nowrap text-muted">
                    {formatArea(r.areaM2)}
                  </span>
                  {canEdit ? (
                    <Button
                      variant="ghost"
                      size="iconSm"
                      onClick={() => setEditing(editing === r.id ? null : r.id)}
                      aria-label={m.target_edit({ name: r.name })}
                      aria-expanded={editing === r.id}
                    >
                      <Target size={16} aria-hidden="true" />
                    </Button>
                  ) : null}
                  {canEdit ? (
                    <Button
                      variant="ghost"
                      size="iconSm"
                      onClick={() => remove.mutate(r.id)}
                      aria-label={m.regions_delete({ name: r.name })}
                    >
                      <Trash2 size={16} aria-hidden="true" />
                    </Button>
                  ) : null}
                </span>
              </div>
              {editing === r.id ? <TargetForm region={r} onDone={() => setEditing(null)} /> : null}
            </li>
          ))}
        </ul>
      ) : regions.isPending ? null : (
        <EmptyState>{m.regions_empty()}</EmptyState>
      )}
    </Section>
  );
}
