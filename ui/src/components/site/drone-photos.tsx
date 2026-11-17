/**
 * Drohnenfotos → NodeODM → neue Aufnahme. Ohne eingerichteten NodeODM nur ein Hinweis mit Link zur Doku.
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plane, Upload } from "lucide-react";
import { useId, useState } from "react";
import { ErrorText, Section } from "@/components/page";
import { Button, buttonVariants } from "@/components/ui/button";
import { Input, Label, Select } from "@/components/ui/input";
import { api, type OdmQuality, uploadImage } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { formatBytes, formatDate } from "@/lib/format";
import {
  dateInputToMs,
  IMAGE_ACCEPT,
  MIN_IMAGES,
  parseOffset,
  photosOnly,
  type UploadProgress,
  uploadFlight,
} from "@/lib/odm";
import { m } from "@/paraglide/messages";

const DOCS = "https://firsttris.github.io/fixpunkt/drohnenfotos.html";

const QUALITY: Record<OdmQuality, () => string> = {
  fast: m.odm_quality_fast,
  standard: m.odm_quality_standard,
  high: m.odm_quality_high,
};

const today = () => new Date().toISOString().slice(0, 10);

export function DronePhotos({ siteId }: { siteId: number }) {
  const status = useQuery({ queryKey: ["odm"], queryFn: api.odmStatus, staleTime: 60_000 });
  const s = status.data;
  return (
    <Section title={m.odm_title()} description={m.odm_text()}>
      {!s ? null : !s.configured ? (
        <p className="m-0 text-[13px] text-muted">
          {m.odm_off()}{" "}
          <a href={DOCS} className="text-fg underline">
            {m.odm_off_link()}
          </a>
        </p>
      ) : !s.reachable ? (
        <ErrorText>{m.odm_unreachable()}</ErrorText>
      ) : (
        <>
          <p className="m-0 text-[12px] text-muted">
            {m.odm_ready({ version: s.version ?? "?", queue: s.queue ?? 0 })}
          </p>
          <FlightForm siteId={siteId} />
        </>
      )}
    </Section>
  );
}

function FlightForm({ siteId }: { siteId: number }) {
  const queryClient = useQueryClient();
  const inputId = useId();
  const [files, setFiles] = useState<File[]>([]);
  const [date, setDate] = useState(today);
  const [name, setName] = useState("");
  const [quality, setQuality] = useState<OdmQuality>("standard");
  const [offset, setOffset] = useState("0");
  const [progress, setProgress] = useState<UploadProgress | null>(null);
  const capturedAt = dateInputToMs(date);
  const heightOffset = parseOffset(offset);
  const send = useMutation({
    mutationFn: () =>
      uploadFlight(
        {
          create: (p) => api.createOdm(siteId, p),
          upload: uploadImage,
          start: api.startOdm,
        },
        {
          name: name.trim() || m.odm_name_default({ date: formatDate(capturedAt) }),
          capturedAt,
          quality,
          heightOffset: heightOffset ?? 0,
        },
        files,
        setProgress,
      ),
    onSettled: () => queryClient.invalidateQueries({ queryKey: ["jobs", siteId] }),
    onSuccess: () => {
      setFiles([]);
      setName("");
      setProgress(null);
    },
  });
  const reason =
    files.length < MIN_IMAGES
      ? m.odm_too_few({ min: MIN_IMAGES })
      : heightOffset === null
        ? m.odm_offset_invalid()
        : null;
  const size = files.reduce((sum, f) => sum + f.size, 0);
  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        if (!reason) send.mutate();
      }}
    >
      <div className="flex flex-wrap items-center gap-3">
        <input
          id={inputId}
          type="file"
          multiple
          accept={IMAGE_ACCEPT}
          aria-label={m.odm_photos()}
          className="sr-only"
          onChange={(e) => setFiles(photosOnly(Array.from(e.target.files ?? [])))}
        />
        <label htmlFor={inputId} className={buttonVariants()}>
          <Upload size={16} aria-hidden="true" />
          {m.odm_choose()}
        </label>
        {files.length ? (
          <span className="tabular text-[13px]">
            {m.odm_selected({ count: files.length, size: formatBytes(size) })}
          </span>
        ) : null}
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <Label>
          {m.odm_name()}
          <Input
            value={name}
            placeholder={m.odm_name_default({ date: formatDate(capturedAt) })}
            onChange={(e) => setName(e.target.value)}
          />
        </Label>
        <Label>
          {m.odm_date()}
          <Input type="date" value={date} required onChange={(e) => setDate(e.target.value)} />
        </Label>
        <Label>
          {m.odm_quality()}
          <Select value={quality} onChange={(e) => setQuality(e.target.value as OdmQuality)}>
            {(Object.keys(QUALITY) as OdmQuality[]).map((q) => (
              <option key={q} value={q}>
                {QUALITY[q]()}
              </option>
            ))}
          </Select>
        </Label>
        <Label>
          {m.odm_offset()}
          <Input
            inputMode="decimal"
            value={offset}
            aria-describedby={`${inputId}-offset`}
            onChange={(e) => setOffset(e.target.value)}
          />
          <span id={`${inputId}-offset`} className="text-[12px]">
            {m.odm_offset_hint()}
          </span>
        </Label>
      </div>
      {progress ? (
        <div className="flex flex-col gap-1.5">
          <span className="text-[13px]" aria-live="polite">
            {m.odm_uploading({ done: progress.done, total: progress.total })}
          </span>
          <progress
            value={progress.fraction}
            max={1}
            aria-label={m.odm_photos()}
            className="h-1.5 w-full accent-fg"
          />
        </div>
      ) : null}
      {send.isSuccess ? (
        <p role="status" className="m-0 text-[13px] text-ok">
          {m.odm_started()}
        </p>
      ) : null}
      <ErrorText>{send.error ? errorText(send.error) : null}</ErrorText>
      <div className="flex items-center justify-end gap-3">
        {reason ? <span className="text-[13px] text-muted">{reason}</span> : null}
        <Button type="submit" variant="primary" disabled={send.isPending || reason !== null}>
          <Plane size={16} aria-hidden="true" />
          {m.odm_submit()}
        </Button>
      </div>
    </form>
  );
}
