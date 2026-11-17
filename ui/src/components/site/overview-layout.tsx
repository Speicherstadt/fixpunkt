/** Überblick anpassen: Abschnitte ein- und ausblenden und verschieben; wird sofort gespeichert */
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ArrowDown, ArrowUp, RotateCcw } from "lucide-react";
import { ErrorText } from "@/components/page";
import { Button } from "@/components/ui/button";
import { api, type Layout } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { type LayoutItem, move, toggle } from "@/lib/layout";
import { m } from "@/paraglide/messages";
import { SECTION_TEXT } from "./overview-sections";

export function LayoutEditor({
  siteId,
  layout,
  onDone,
}: {
  siteId: number;
  layout: Layout;
  onDone: () => void;
}) {
  const qc = useQueryClient();
  const key = ["layout", siteId];
  const save = useMutation({
    mutationFn: (sections: LayoutItem[]) => api.saveLayout(siteId, sections),
    // sofort zeigen, nicht auf den Server warten
    onMutate: (sections) => qc.setQueryData(key, { sections, custom: true }),
    onSettled: () => qc.invalidateQueries({ queryKey: key }),
  });
  const reset = useMutation({
    mutationFn: () => api.resetLayout(siteId),
    onSettled: () => qc.invalidateQueries({ queryKey: key }),
  });
  const items = layout.sections;
  return (
    <div className="flex flex-col gap-3 rounded-lg bg-subtle p-3">
      <ul className="m-0 flex list-none flex-col gap-1 p-0" aria-label={m.overview_customize()}>
        {items.map((it, i) => {
          const name = (SECTION_TEXT[it.id] ?? (() => it.id))();
          return (
            <li key={it.id} className="flex items-center justify-between gap-2">
              <label className="flex items-center gap-2 text-[14px]">
                <input
                  type="checkbox"
                  className="size-5 accent-fg"
                  checked={it.visible}
                  onChange={() => save.mutate(toggle(items, it.id))}
                />
                {name}
              </label>
              <span className="flex gap-1">
                <Button
                  size="iconSm"
                  variant="ghost"
                  aria-label={m.overview_move_up({ name })}
                  disabled={i === 0}
                  onClick={() => save.mutate(move(items, i, -1))}
                >
                  <ArrowUp size={16} aria-hidden="true" />
                </Button>
                <Button
                  size="iconSm"
                  variant="ghost"
                  aria-label={m.overview_move_down({ name })}
                  disabled={i === items.length - 1}
                  onClick={() => save.mutate(move(items, i, 1))}
                >
                  <ArrowDown size={16} aria-hidden="true" />
                </Button>
              </span>
            </li>
          );
        })}
      </ul>
      <ErrorText>{save.error ? errorText(save.error) : null}</ErrorText>
      <div className="flex justify-between gap-2">
        <Button size="sm" variant="ghost" onClick={() => reset.mutate()} disabled={!layout.custom}>
          <RotateCcw size={14} aria-hidden="true" />
          {m.overview_reset()}
        </Button>
        <Button size="sm" variant="primary" onClick={onDone}>
          {m.overview_done()}
        </Button>
      </div>
    </div>
  );
}
