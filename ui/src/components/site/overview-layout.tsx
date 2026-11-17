/** Überblick anpassen: Abschnitte ein- und ausblenden und verschieben; wird sofort gespeichert */
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ArrowDown, ArrowUp, RotateCcw } from "lucide-react";
import { useState } from "react";
import { ErrorText } from "@/components/page";
import { Button } from "@/components/ui/button";
import { api, type Layout } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { type LayoutItem, move, toggle } from "@/lib/layout";
import { layoutQuery } from "@/lib/queries";
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
  const key = layoutQuery(siteId).queryKey;
  // Auswahl hier halten: Häkchen und Pfeile wirken sofort, gespeichert wird im Hintergrund
  const [items, setItems] = useState<LayoutItem[]>(layout.sections);
  const save = useMutation({
    mutationFn: (sections: LayoutItem[]) => api.saveLayout(siteId, sections),
    onSuccess: (saved) => qc.setQueryData(key, saved),
  });
  const apply = (next: LayoutItem[]) => {
    setItems(next);
    // der Überblick zeigt es gleich; eine noch laufende Abfrage darf das nicht überschreiben
    void qc.cancelQueries({ queryKey: key });
    qc.setQueryData(key, { sections: next, custom: true });
    save.mutate(next);
  };
  const reset = useMutation({
    mutationFn: () => api.resetLayout(siteId),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: key });
      setItems((await qc.fetchQuery(layoutQuery(siteId))).sections);
    },
  });
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
                  onChange={() => apply(toggle(items, it.id))}
                />
                {name}
              </label>
              <span className="flex gap-1">
                <Button
                  size="iconSm"
                  variant="ghost"
                  aria-label={m.overview_move_up({ name })}
                  disabled={i === 0}
                  onClick={() => apply(move(items, i, -1))}
                >
                  <ArrowUp size={16} aria-hidden="true" />
                </Button>
                <Button
                  size="iconSm"
                  variant="ghost"
                  aria-label={m.overview_move_down({ name })}
                  disabled={i === items.length - 1}
                  onClick={() => apply(move(items, i, 1))}
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
        <Button
          size="sm"
          variant="ghost"
          onClick={() => reset.mutate()}
          disabled={!layout.custom && !save.isSuccess}
        >
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
