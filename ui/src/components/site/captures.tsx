import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Download, Trash2, Upload } from "lucide-react";
import { useId, useRef, useState } from "react";
import { EmptyState, ErrorText, Section } from "@/components/page";
import { Button, buttonVariants } from "@/components/ui/button";
import { PlaceholderRows } from "@/components/ui/placeholder";
import { StatusBadge } from "@/components/ui/status";
import { api, type Capture, uploadCapture } from "@/lib/api";
import { errorText } from "@/lib/errors";
import { formatBytes, formatCount, formatDate } from "@/lib/format";
import { capturesQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";

/** Dateiname ohne Endung als Vorschlag für den Namen der Aufnahme */
export const captureName = (fileName: string): string => fileName.replace(/\.[^.]+$/, "");

const ACCEPT = ".las,.laz,.xyz,.txt,.csv,.pts,.tif,.tiff,.asc";

type PendingUpload = { name: string; progress: number };

export function Captures({ siteId, canEdit }: { siteId: number; canEdit: boolean }) {
  const captures = useQuery(capturesQuery(siteId));
  const queryClient = useQueryClient();
  const input = useRef<HTMLInputElement>(null);
  const inputId = useId();
  const [uploads, setUploads] = useState<PendingUpload[]>([]);
  const [error, setError] = useState<string | null>(null);

  const upload = async (files: FileList | null) => {
    setError(null);
    for (const file of Array.from(files ?? [])) {
      setUploads((u) => [...u, { name: file.name, progress: 0 }]);
      try {
        await uploadCapture(siteId, file, captureName(file.name), (p) =>
          setUploads((u) => u.map((x) => (x.name === file.name ? { ...x, progress: p } : x))),
        );
      } catch (err) {
        setError(`${file.name}: ${errorText(err)}`);
      } finally {
        setUploads((u) => u.filter((x) => x.name !== file.name));
        await queryClient.invalidateQueries({ queryKey: ["captures", siteId] });
      }
    }
    if (input.current) input.current.value = "";
  };

  const remove = useMutation({
    mutationFn: (id: number) => api.deleteCapture(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["captures", siteId] }),
  });

  return (
    <Section
      title={m.captures_title()}
      description={m.captures_text()}
      actions={
        canEdit ? (
          <>
            <input
              ref={input}
              id={inputId}
              type="file"
              multiple
              accept={ACCEPT}
              className="sr-only"
              onChange={(e) => upload(e.target.files)}
            />
            <label htmlFor={inputId} className={buttonVariants({ variant: "primary" })}>
              <Upload size={16} aria-hidden="true" />
              {m.captures_upload()}
            </label>
          </>
        ) : null
      }
    >
      {uploads.map((u) => (
        <div key={u.name} className="flex flex-col gap-1.5">
          <span className="text-[13px]">{m.captures_uploading({ name: u.name })}</span>
          <progress
            value={u.progress}
            max={1}
            aria-label={u.name}
            className="h-1.5 w-full accent-fg"
          />
        </div>
      ))}
      <ErrorText>{error ?? (remove.error ? errorText(remove.error) : null)}</ErrorText>
      {captures.isPending ? (
        <PlaceholderRows count={2} />
      ) : captures.data?.length ? (
        <div className="overflow-x-auto">
          <table className="w-full border-collapse text-[14px]">
            <thead>
              <tr className="text-left text-[12px] text-muted">
                <th className="py-2 pr-3 font-medium">{m.captures_name()}</th>
                <th className="py-2 pr-3 font-medium">{m.captures_date()}</th>
                <th className="py-2 pr-3 text-right font-medium">{m.captures_points()}</th>
                <th className="py-2 pr-3 text-right font-medium">{m.captures_size()}</th>
                <th className="py-2 pr-3 font-medium">{m.captures_status()}</th>
                <th className="py-2">
                  <span className="sr-only">{m.common_actions()}</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {captures.data.map((c) => (
                <CaptureRow
                  key={c.id}
                  c={c}
                  canEdit={canEdit}
                  onDelete={() => remove.mutate(c.id)}
                />
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <EmptyState>{m.captures_empty()}</EmptyState>
      )}
    </Section>
  );
}

function CaptureRow({
  c,
  canEdit,
  onDelete,
}: {
  c: Capture;
  canEdit: boolean;
  onDelete: () => void;
}) {
  const points = c.info?.type === "pointcloud" && c.info.points ? formatCount(c.info.points) : "–";
  return (
    <tr className="border-t border-border align-top">
      <td className="py-2.5 pr-3">
        <div className="font-medium">{c.name}</div>
        <div className="text-[12px] text-muted">{c.fileName}</div>
        {c.error ? <div className="text-[12px] text-danger">{c.error}</div> : null}
      </td>
      <td className="py-2.5 pr-3 whitespace-nowrap">{formatDate(c.capturedAt)}</td>
      <td className="tabular py-2.5 pr-3 text-right">{points}</td>
      <td className="tabular py-2.5 pr-3 text-right whitespace-nowrap">
        {formatBytes(c.fileSize)}
      </td>
      <td className="py-2.5 pr-3">
        <StatusBadge status={c.status} />
      </td>
      <td className="py-1.5">
        <div className="flex justify-end gap-1">
          <a
            href={`/api/captures/${c.id}/file`}
            className={buttonVariants({ variant: "ghost", size: "iconSm" })}
            aria-label={m.captures_download({ name: c.name })}
          >
            <Download size={16} aria-hidden="true" />
          </a>
          {canEdit ? (
            <Button
              variant="ghost"
              size="iconSm"
              onClick={onDelete}
              aria-label={m.captures_delete({ name: c.name })}
            >
              <Trash2 size={16} aria-hidden="true" />
            </Button>
          ) : null}
        </div>
      </td>
    </tr>
  );
}
