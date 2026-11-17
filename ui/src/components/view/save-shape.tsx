/**
 * Gezeichnete Form speichern: als Abrechnungsbereich (Werkzeug „Bereich zeichnen“) oder als Linie der
 * Dokumentation (Werkzeug „Leitung zeichnen“, Höhen vom Gelände darunter).
 */
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { ErrorText } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input, Label } from "@/components/ui/input";
import { api } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { tracePoints } from "@/lib/markers";
import { type Pt, polygonGeojson } from "@/lib/view/geometry";
import type { HeightGrid } from "@/lib/view/grid";
import { m } from "@/paraglide/messages";

export function SaveShape({
  kind,
  siteId,
  epsg,
  grid,
  points,
  onSaved,
}: {
  kind: "region" | "line";
  siteId: number;
  epsg: number;
  grid: HeightGrid;
  points: Pt[];
  onSaved: () => void;
}) {
  const qc = useQueryClient();
  const [name, setName] = useState("");
  const [note, setNote] = useState("");
  const enough = points.length >= (kind === "region" ? 3 : 2);
  const save = useMutation({
    mutationFn: async (): Promise<void> => {
      if (kind === "region")
        await api.addRegions(siteId, polygonGeojson(points, epsg), name.trim());
      else
        await api.createMarker(siteId, {
          kind: "line",
          title: name.trim(),
          note: note.trim(),
          points: tracePoints(points, grid),
        });
    },
    onSuccess: async () => {
      setName("");
      setNote("");
      onSaved();
      await qc.invalidateQueries({ queryKey: [kind === "region" ? "regions" : "markers", siteId] });
    },
  });
  return (
    <>
      <form
        className="ml-auto flex flex-wrap items-end gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          save.mutate();
        }}
      >
        <Label className="w-56">
          {kind === "region" ? m.view_region_name() : m.view_line_name()}
          <Input value={name} onChange={(e) => setName(e.target.value)} required />
        </Label>
        {kind === "line" ? (
          <Label className="w-56">
            {m.view_line_note()}
            <Input value={note} onChange={(e) => setNote(e.target.value)} />
          </Label>
        ) : null}
        <Button type="submit" variant="primary" disabled={!enough || save.isPending}>
          {kind === "region" ? m.view_region_save() : m.view_line_save()}
        </Button>
      </form>
      {!enough ? (
        <p className="m-0 w-full text-[13px] text-muted">
          {kind === "region" ? m.view_draw_hint() : m.view_trace_hint()}
        </p>
      ) : null}
      <ErrorText>{save.error ? errorText(save.error) : null}</ErrorText>
    </>
  );
}
