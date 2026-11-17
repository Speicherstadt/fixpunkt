/**
 * Freigabe durch den Auftraggeber: Link auf das eingefrorene Aufmaß erzeugen, Stand der Freigaben sehen,
 * Aufmaßblatt mit Entscheidung laden, Link zurückziehen.
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronDown, ChevronRight, Copy, FileDown, Mail, Send, Undo2 } from "lucide-react";
import { type FormEvent, useState } from "react";
import { ErrorText } from "@/components/page";
import { Button, buttonVariants } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { Input, Label } from "@/components/ui/input";
import { api, type Share, shareLink, sharePdfHref } from "@/lib/api";
import { decisionText } from "@/lib/billing";
import { cn } from "@/lib/cn";
import { errorText } from "@/lib/errors";
import { formatDate, formatEuro } from "@/lib/format";
import { sharesQuery } from "@/lib/queries";
import { m } from "@/paraglide/messages";

/** Knopf „Zur Freigabe senden“ mit Dialog: Notiz, dann der Link zum Kopieren oder per E-Mail */
export function ShareButton({
  siteId,
  period,
}: {
  siteId: number;
  period: { from: number; to: number };
}) {
  const qc = useQueryClient();
  const [open, setOpen] = useState(false);
  const [note, setNote] = useState("");
  const create = useMutation({
    mutationFn: () => api.createShare(siteId, { ...period, note: note.trim() || undefined }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["billing", siteId] }),
  });
  const link = create.data?.token ? shareLink(create.data.token) : null;
  const close = (o: boolean) => {
    setOpen(o);
    if (!o) {
      create.reset();
      setNote("");
    }
  };
  const submit = (e: FormEvent) => {
    e.preventDefault();
    create.mutate();
  };
  return (
    <>
      <Button onClick={() => setOpen(true)}>
        <Send size={16} aria-hidden="true" />
        {m.share_send()}
      </Button>
      <Dialog
        open={open}
        onOpenChange={close}
        title={m.share_send()}
        description={m.share_send_text()}
      >
        {link ? (
          <div className="flex flex-col gap-3">
            <Label>
              {m.share_link()}
              <Input readOnly value={link} onFocus={(e) => e.target.select()} />
            </Label>
            <p className="m-0 text-[13px] text-muted">{m.share_link_once()}</p>
            <div className="flex flex-wrap justify-end gap-2">
              <a
                className={buttonVariants()}
                href={`mailto:?subject=${encodeURIComponent(m.share_mail_subject())}&body=${encodeURIComponent(
                  m.share_mail_body({ link }),
                )}`}
              >
                <Mail size={16} aria-hidden="true" />
                {m.share_mail()}
              </a>
              <Button variant="primary" onClick={() => navigator.clipboard?.writeText(link)}>
                <Copy size={16} aria-hidden="true" />
                {m.share_copy()}
              </Button>
            </div>
          </div>
        ) : (
          <form onSubmit={submit} className="flex flex-col gap-3">
            <Label>
              {m.share_note()}
              <Input value={note} onChange={(e) => setNote(e.target.value)} />
            </Label>
            <ErrorText>{create.error ? errorText(create.error) : null}</ErrorText>
            <div className="flex justify-end gap-2">
              <Button onClick={() => close(false)}>{m.common_cancel()}</Button>
              <Button type="submit" variant="primary" disabled={create.isPending}>
                {m.share_create()}
              </Button>
            </div>
          </form>
        )}
      </Dialog>
    </>
  );
}

const statusClass = (s: Share) =>
  s.decision?.decision === "approved"
    ? "text-ok"
    : s.decision?.decision === "objected"
      ? "text-warn"
      : "text-muted";

/** Liste der Freigaben einer Baustelle */
export function Shares({ siteId, canEdit }: { siteId: number; canEdit: boolean }) {
  const [open, setOpen] = useState(false);
  const shares = useQuery(sharesQuery(siteId));
  const qc = useQueryClient();
  const revoke = useMutation({
    mutationFn: api.revokeShare,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["billing", siteId] }),
  });
  const list = shares.data ?? [];
  return (
    <div className="flex flex-col gap-3">
      <Button
        variant="ghost"
        size="sm"
        className="self-start px-1"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        {open ? (
          <ChevronDown size={16} aria-hidden="true" />
        ) : (
          <ChevronRight size={16} aria-hidden="true" />
        )}
        {m.share_list({ count: list.length })}
      </Button>
      <ErrorText>{revoke.error ? errorText(revoke.error) : null}</ErrorText>
      {open ? (
        list.length ? (
          <ul
            aria-label={m.share_list_label()}
            className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0"
          >
            {list.map((s) => (
              <li key={s.id} className="flex flex-wrap items-center gap-3 px-3 py-2 text-[13px]">
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="font-medium">
                    {m.share_period({ from: s.from.name, to: s.to.name })}
                    {s.amount == null ? null : ` · ${formatEuro(s.amount)}`}
                  </span>
                  <span className="text-muted">
                    {m.share_sent({ date: formatDate(s.createdAt) })}
                    {s.note ? ` · ${s.note}` : null}
                  </span>
                  <span className={cn("font-medium", statusClass(s))}>
                    {s.revokedAt && !s.decision ? m.share_revoked() : decisionText(s.decision)}
                  </span>
                </span>
                <a
                  href={sharePdfHref(s.id)}
                  className={buttonVariants({ variant: "ghost", size: "iconSm" })}
                  aria-label={m.share_pdf_of({ to: s.to.name })}
                >
                  <FileDown size={16} aria-hidden="true" />
                </a>
                {canEdit && !s.revokedAt ? (
                  <Button
                    variant="ghost"
                    size="iconSm"
                    aria-label={m.share_revoke_of({ to: s.to.name })}
                    onClick={() => revoke.mutate(s.id)}
                  >
                    <Undo2 size={16} aria-hidden="true" />
                  </Button>
                ) : null}
              </li>
            ))}
          </ul>
        ) : (
          <p className="m-0 text-[13px] text-muted">{m.share_none()}</p>
        )
      ) : null}
    </div>
  );
}
