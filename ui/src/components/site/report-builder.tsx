/** Eigenen Bericht zusammenstellen: Titel, Firma, Bausteine wählen und ordnen, als PDF herunterladen */
import { ArrowDown, ArrowUp, FileDown } from "lucide-react";
import { useState } from "react";
import { Button, buttonVariants } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { Input, Label } from "@/components/ui/input";
import { move, toggle } from "@/lib/layout";
import {
  defaultReport,
  loadReport,
  type ReportSettings,
  reportHref,
  saveReport,
} from "@/lib/report-parts";
import { m } from "@/paraglide/messages";
import { SECTION_TEXT } from "./overview-sections";

const PART_TEXT: Record<string, () => string> = {
  ...SECTION_TEXT,
  compare: m.compose_part_compare,
  constraints: m.compose_part_constraints,
  lines: m.compose_part_lines,
  notes: m.compose_part_notes,
  photos: m.compose_part_photos,
};

export function ReportBuilder({
  siteId,
  open,
  onOpenChange,
}: {
  siteId: number;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [s, set] = useState<ReportSettings>(loadReport);
  const update = (next: ReportSettings) => {
    set(next);
    saveReport(next);
  };
  const href = reportHref(siteId, s);
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={m.compose_title()}
      description={m.compose_description()}
    >
      <div className="grid gap-3 sm:grid-cols-2">
        <Label>
          {m.compose_name()}
          <Input
            value={s.title}
            placeholder={m.compose_name_placeholder()}
            onChange={(e) => update({ ...s, title: e.target.value })}
          />
        </Label>
        <Label>
          {m.compose_company()}
          <Input value={s.company} onChange={(e) => update({ ...s, company: e.target.value })} />
        </Label>
      </div>
      <ul
        className="m-0 flex max-h-80 list-none flex-col gap-1 overflow-y-auto p-0"
        aria-label={m.compose_parts()}
      >
        {s.parts.map((p, i) => {
          const name = (PART_TEXT[p.id] ?? (() => p.id))();
          return (
            <li key={p.id} className="flex items-center justify-between gap-2">
              <label className="flex items-center gap-2 text-[14px]">
                <input
                  type="checkbox"
                  className="size-5 accent-fg"
                  checked={p.visible}
                  onChange={() => update({ ...s, parts: toggle(s.parts, p.id) })}
                />
                {name}
              </label>
              <span className="flex gap-1">
                <Button
                  size="iconSm"
                  variant="ghost"
                  aria-label={m.overview_move_up({ name })}
                  disabled={i === 0}
                  onClick={() => update({ ...s, parts: move(s.parts, i, -1) })}
                >
                  <ArrowUp size={16} aria-hidden="true" />
                </Button>
                <Button
                  size="iconSm"
                  variant="ghost"
                  aria-label={m.overview_move_down({ name })}
                  disabled={i === s.parts.length - 1}
                  onClick={() => update({ ...s, parts: move(s.parts, i, 1) })}
                >
                  <ArrowDown size={16} aria-hidden="true" />
                </Button>
              </span>
            </li>
          );
        })}
      </ul>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <Button size="sm" variant="ghost" onClick={() => update(defaultReport())}>
          {m.overview_reset()}
        </Button>
        {href ? (
          <a href={href} className={buttonVariants({ variant: "primary" })}>
            <FileDown size={16} aria-hidden="true" />
            {m.compose_download()}
          </a>
        ) : (
          <span className="text-[13px] text-muted">{m.compose_none()}</span>
        )}
      </div>
    </Dialog>
  );
}
