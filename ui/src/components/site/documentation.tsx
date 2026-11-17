/**
 * Dokumentation der Baustelle: Zwangspunkte mit Sollhöhe (gegen die neueste Aufnahme geprüft), Fotos mit
 * Position aus GPS, Notizen. Fotos ohne GPS erscheinen in der Liste, aber nicht in der Ansicht.
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Camera, FileDown, MapPin, Route, StickyNote, Trash2, Upload } from "lucide-react";
import { type FormEvent, useId, useState } from "react";
import { EmptyState, ErrorText, Section } from "@/components/page";
import { cm } from "@/components/report/compare-report";
import { Button, buttonVariants } from "@/components/ui/button";
import { Input, Label, Select, Textarea } from "@/components/ui/input";
import { api, type Marker, type MarkerKind, uploadPhoto } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { formatDateTime, formatHeight, formatNumber } from "@/lib/format";
import { chronological } from "@/lib/jobs";
import { checkConstraint, compass, lineLength } from "@/lib/markers";
import { capturesQuery, markersQuery } from "@/lib/queries";
import { fetchGrid, type HeightGrid } from "@/lib/view/grid";
import { m } from "@/paraglide/messages";

const ICON = { constraint: MapPin, photo: Camera, note: StickyNote, line: Route } as const;
const KIND_TEXT: Record<MarkerKind, () => string> = {
  constraint: m.doc_kind_constraint,
  photo: m.doc_kind_photo,
  note: m.doc_kind_note,
  line: m.doc_kind_line,
};

const parse = (s: string) => (s.trim() ? Number(s.replace(",", ".")) : undefined);

export function Documentation({ siteId, canEdit }: { siteId: number; canEdit: boolean }) {
  const markers = useQuery(markersQuery(siteId));
  const captures = useQuery(capturesQuery(siteId));
  const latest = chronological((captures.data ?? []).filter((c) => c.status === "ready")).at(-1);
  // dasselbe Raster wie die Ansicht (gleicher Schlüssel, nur einmal geladen)
  const grid = useQuery({
    queryKey: ["grid", "capture", latest?.id],
    queryFn: () => fetchGrid(`/api/captures/${latest!.id}/grid`),
    enabled: latest !== undefined,
    staleTime: Number.POSITIVE_INFINITY,
  });
  const queryClient = useQueryClient();
  const inputId = useId();
  const [uploading, setUploading] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["markers", siteId] });
  const upload = async (files: FileList | null) => {
    setError(null);
    for (const f of Array.from(files ?? [])) {
      setUploading(f.name);
      try {
        await uploadPhoto(siteId, f, () => {});
      } catch (e) {
        setError(`${f.name}: ${errorText(e)}`);
      }
    }
    setUploading(null);
    await refresh();
  };
  const remove = useMutation({ mutationFn: api.deleteMarker, onSuccess: refresh });

  return (
    <Section
      title={m.doc_title()}
      description={m.doc_text()}
      actions={
        <>
          <a
            href={`/api/sites/${siteId}/documentation/report`}
            className={buttonVariants({ variant: "outline" })}
          >
            <FileDown size={16} aria-hidden="true" />
            {m.doc_report()}
          </a>
          <a
            href={`/api/sites/${siteId}/documentation.dxf`}
            className={buttonVariants({ variant: "outline" })}
          >
            <FileDown size={16} aria-hidden="true" />
            {m.doc_dxf()}
          </a>
          {canEdit ? (
            <>
              <input
                id={inputId}
                type="file"
                multiple
                accept="image/jpeg,image/png"
                className="sr-only"
                onChange={(e) => upload(e.target.files)}
              />
              <label htmlFor={inputId} className={buttonVariants({ variant: "primary" })}>
                <Upload size={16} aria-hidden="true" />
                {m.doc_upload()}
              </label>
            </>
          ) : null}
        </>
      }
    >
      {uploading ? (
        <p role="status" className="m-0 text-[13px] text-muted">
          {m.doc_uploading({ name: uploading })}
        </p>
      ) : null}
      <ErrorText>{error ?? (remove.error ? errorText(remove.error) : null)}</ErrorText>
      {canEdit ? <NewMarkerForm siteId={siteId} onDone={refresh} /> : null}
      {markers.data?.length ? (
        <ul className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0">
          {markers.data.map((mk) => (
            <MarkerRow
              key={mk.id}
              marker={mk}
              grid={grid.data}
              canEdit={canEdit}
              onDelete={() => remove.mutate(mk.id)}
            />
          ))}
        </ul>
      ) : markers.isPending ? null : (
        <EmptyState>{m.doc_empty()}</EmptyState>
      )}
    </Section>
  );
}

function MarkerRow({
  marker: mk,
  grid,
  canEdit,
  onDelete,
}: {
  marker: Marker;
  grid: HeightGrid | undefined;
  canEdit: boolean;
  onDelete: () => void;
}) {
  const Icon = ICON[mk.kind];
  const check = checkConstraint(mk, grid);
  return (
    <li className="flex gap-3 px-4 py-3">
      {mk.photo ? (
        <a
          href={`/api/markers/${mk.id}/photo`}
          target="_blank"
          rel="noreferrer"
          className="shrink-0"
        >
          <img
            src={`/api/markers/${mk.id}/thumb`}
            alt={mk.title}
            loading="lazy"
            className="size-16 rounded-md border border-border object-cover"
          />
        </a>
      ) : (
        <Icon size={20} aria-hidden="true" className="mt-0.5 shrink-0 text-muted" />
      )}
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="font-medium">{mk.title}</span>
        <span className="text-[12px] text-muted">
          {KIND_TEXT[mk.kind]()} · {formatDateTime(mk.takenAt ?? mk.createdAt)}
          {mk.x !== null && mk.y !== null
            ? ` · ${m.doc_position({ e: mk.x.toFixed(2).replace(".", ","), n: mk.y.toFixed(2).replace(".", ",") })}`
            : ` · ${m.doc_no_position()}`}
        </span>
        {mk.direction !== null && mk.direction !== undefined ? (
          <span className="tabular text-[13px]">
            {m.doc_direction({ direction: compass(mk.direction) })}
          </span>
        ) : null}
        {mk.note ? <span className="text-[13px]">{mk.note}</span> : null}
        {mk.kind === "line" && mk.points ? (
          <span className="tabular text-[13px]">
            {m.doc_line({
              length: formatNumber(lineLength(mk.points), 2),
              points: mk.points.length,
            })}
          </span>
        ) : null}
        {mk.kind === "constraint" && mk.z !== null ? (
          <span className="tabular text-[13px]">
            {m.doc_target({ h: formatHeight(mk.z) })}
            {check ? (
              <span className={check.ok ? "text-ok" : "text-warn"}>
                {" · "}
                {m.doc_actual({ h: formatHeight(check.actual), diff: cm(check.diff) })}
              </span>
            ) : null}
          </span>
        ) : null}
      </div>
      {canEdit ? (
        <Button
          size="iconSm"
          variant="ghost"
          aria-label={m.doc_delete({ name: mk.title })}
          onClick={onDelete}
        >
          <Trash2 size={14} aria-hidden="true" />
        </Button>
      ) : null}
    </li>
  );
}

function NewMarkerForm({ siteId, onDone }: { siteId: number; onDone: () => void }) {
  const [kind, setKind] = useState<"constraint" | "note">("constraint");
  const [title, setTitle] = useState("");
  const [note, setNote] = useState("");
  const [pos, setPos] = useState({ e: "", n: "", h: "" });
  const create = useMutation({
    mutationFn: () =>
      api.createMarker(siteId, {
        kind,
        title,
        note,
        x: parse(pos.e),
        y: parse(pos.n),
        z: parse(pos.h),
      }),
    onSuccess: () => {
      setTitle("");
      setNote("");
      setPos({ e: "", n: "", h: "" });
      onDone();
    },
  });
  const submit = (e: FormEvent) => {
    e.preventDefault();
    create.mutate();
  };
  return (
    <details className="text-[13px]">
      <summary className="cursor-pointer text-muted">{m.doc_add()}</summary>
      <form onSubmit={submit} className="mt-3 flex flex-col gap-3">
        <div className="grid gap-3 sm:grid-cols-2">
          <Label>
            {m.doc_kind()}
            <Select value={kind} onChange={(e) => setKind(e.target.value as typeof kind)}>
              <option value="constraint">{m.doc_kind_constraint()}</option>
              <option value="note">{m.doc_kind_note()}</option>
            </Select>
          </Label>
          <Label>
            {m.doc_title_field()}
            <Input value={title} onChange={(e) => setTitle(e.target.value)} required />
          </Label>
          <Label>
            {m.doc_east()}
            <Input
              inputMode="decimal"
              value={pos.e}
              required={kind === "constraint"}
              onChange={(e) => setPos({ ...pos, e: e.target.value })}
            />
          </Label>
          <Label>
            {m.doc_north()}
            <Input
              inputMode="decimal"
              value={pos.n}
              required={kind === "constraint"}
              onChange={(e) => setPos({ ...pos, n: e.target.value })}
            />
          </Label>
          <Label>
            {kind === "constraint" ? m.doc_target_height() : m.doc_height()}
            <Input
              inputMode="decimal"
              value={pos.h}
              required={kind === "constraint"}
              onChange={(e) => setPos({ ...pos, h: e.target.value })}
            />
          </Label>
          <Label>
            {m.doc_note()}
            <Textarea className="min-h-10" value={note} onChange={(e) => setNote(e.target.value)} />
          </Label>
        </div>
        <ErrorText>{create.error ? errorText(create.error) : null}</ErrorText>
        <div className="flex justify-end">
          <Button type="submit" variant="primary" disabled={create.isPending}>
            {m.doc_save()}
          </Button>
        </div>
      </form>
    </details>
  );
}
