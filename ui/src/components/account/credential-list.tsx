/** Liste von Passkeys oder Tokens: Name, angelegt, zuletzt benutzt, Löschen */
import { Trash2 } from "lucide-react";
import { EmptyState } from "@/components/page";
import { Button } from "@/components/ui/button";
import type { Credential } from "@/lib/api";
import { formatDate } from "@/lib/format";
import { m } from "@/paraglide/messages";

export function CredentialList({
  items,
  empty,
  onDelete,
}: {
  items: Credential[];
  empty: string;
  onDelete: (c: Credential) => void;
}) {
  if (!items.length) return <EmptyState>{empty}</EmptyState>;
  return (
    <ul className="m-0 flex list-none flex-col divide-y divide-border p-0">
      {items.map((c) => (
        <li key={c.id} className="flex items-center justify-between gap-3 py-2.5">
          <div className="flex min-w-0 flex-col">
            <span className="truncate text-[14px]">{c.name}</span>
            <span className="text-[12px] text-muted">
              {m.credential_created({ date: formatDate(c.createdAt) })}
              {" · "}
              {c.lastUsedAt
                ? m.credential_used({ date: formatDate(c.lastUsedAt) })
                : m.credential_unused()}
            </span>
          </div>
          <Button
            variant="ghost"
            size="iconSm"
            aria-label={m.credential_delete({ name: c.name })}
            onClick={() => onDelete(c)}
          >
            <Trash2 size={16} aria-hidden="true" />
          </Button>
        </li>
      ))}
    </ul>
  );
}
