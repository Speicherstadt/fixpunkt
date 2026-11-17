/** Abrechnung: Zeitraum vorschlagen, Einheiten benennen, Abweichung bewerten (reine Funktionen) */
import type { Capture, DeliveryUnit, PriceUnit, ShareDecision, Statement } from "@/lib/api";
import { formatDate, formatNumber } from "@/lib/format";
import { m } from "@/paraglide/messages";

/** Geprüfte Aufnahmen in zeitlicher Reihenfolge */
export const readyCaptures = (cs: Capture[]) =>
  cs.filter((c) => c.status === "ready").sort((a, b) => a.capturedAt - b.capturedAt || a.id - b.id);

/** Vorschlag: der letzte Zeitraum, von der vorletzten bis zur letzten Befliegung */
export const defaultPeriod = (cs: Capture[]): { from: number; to: number } | null => {
  const [from, to] = readyCaptures(cs).slice(-2);
  return from && to ? { from: from.id, to: to.id } : null;
};

export const priceUnitLabel = (u: PriceUnit | DeliveryUnit): string =>
  ({
    m3: m.billing_unit_m3(),
    m3_loose: m.billing_unit_m3_loose(),
    t: m.billing_unit_t(),
    loads: m.billing_unit_loads(),
  })[u];

/** Einheit eines Lieferscheins: geladen wird lose */
export const deliveryUnitLabel = (u: DeliveryUnit): string =>
  u === "m3" ? m.billing_unit_m3_loose() : priceUnitLabel(u);

/** „600,0 t + 10 Ladungen“ */
export const deliveredText = (d: Statement["checks"][number]["delivered"]): string =>
  d
    .map(([u, q]) => `${formatNumber(q, u === "loads" ? 0 : 1)} ${deliveryUnitLabel(u)}`)
    .join(" + ");

/** Abweichung über 10 % lohnt einen zweiten Blick */
export const DEVIATION_WARN = 10;

export const signedPercent = (p: number) =>
  `${p > 0 ? "+" : p < 0 ? "−" : "±"}${formatNumber(Math.abs(p), 1)} %`;

/** Heute als `2026-11-17` (Vorgabe für einen neuen Lieferschein) */
export const today = (now = new Date()) => now.toISOString().slice(0, 10);

/** Stand einer Freigabe in Worten */
export const decisionText = (d: ShareDecision | null): string => {
  if (!d) return m.share_open();
  const name = d.organization ? `${d.name} (${d.organization})` : d.name;
  const date = formatDate(d.at);
  return d.decision === "approved"
    ? m.share_approved({ name, date })
    : m.share_objected({ name, date, comment: d.comment ?? "" });
};
