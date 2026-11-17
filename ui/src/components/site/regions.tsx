import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { MapPinned, Trash2 } from "lucide-react";
import { useId } from "react";
import { EmptyState, ErrorText, Section } from "@/components/page";
import { Button, buttonVariants } from "@/components/ui/button";
import { api } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { formatArea } from "@/lib/format";
import { regionsQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";

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
            <li key={r.id} className="flex items-center justify-between gap-3 py-2">
              <span>{r.name}</span>
              <span className="flex items-center gap-2">
                <span className="tabular text-[13px] text-muted">{formatArea(r.areaM2)}</span>
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
            </li>
          ))}
        </ul>
      ) : regions.isPending ? null : (
        <EmptyState>{m.regions_empty()}</EmptyState>
      )}
    </Section>
  );
}
