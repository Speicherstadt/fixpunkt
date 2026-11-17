/**
 * Zeichenfläche der 2D-Ansicht: Höhenbild, Überlagerung, Bereiche, aktuelle Messung. Maus: ziehen verschiebt,
 * Rad zoomt, Klick setzt einen Punkt (außer im Werkzeug „Verschieben“). Tastatur: Pfeile verschieben, + und −
 * zoomen, Enter setzt einen Punkt am Fadenkreuz in der Mitte.
 */
import { type KeyboardEvent, type PointerEvent, useEffect, useMemo, useRef, useState } from "react";
import type { PlanGeometry } from "@/lib/api";
import { renderDeviation, renderHeights } from "@/lib/view/colors";
import type { Pt } from "@/lib/view/geometry";
import { bboxOf, type HeightGrid } from "@/lib/view/grid";
import { type Corners, orthoTransform } from "@/lib/view/ortho";
import {
  fitTo,
  pan,
  scaleBar,
  toScreen,
  toWorld,
  type Viewport,
  zoomAt,
} from "@/lib/view/viewport";
import { m } from "@/paraglide/messages";

export type Shape = { points: Pt[]; closed: boolean };
export type Outline = { name: string; points: Pt[] };
/** Markierung der Dokumentation: Raute für Zwangspunkte, Kreis für Fotos und Notizen */
export type Pin = { label: string; at: Pt; diamond: boolean };

type Props = {
  grid: HeightGrid;
  range: [number, number];
  overlay: { grid: HeightGrid; limit: number; removedIsNegative: boolean } | null;
  outlines: Outline[];
  /** gezeichnete Linien der Dokumentation (Leitungsverlauf) */
  lines: Outline[];
  pins: Pin[];
  /** DXF-Plan als Strichzeichnung über dem Höhenbild */
  plan: PlanGeometry | null;
  /** Orthofoto statt des Höhenbilds */
  ortho: { image: HTMLImageElement; width: number; height: number; corners: Corners } | null;
  shape: Shape;
  /** Klick oder Enter setzt Punkte; im Werkzeug „Verschieben“ nicht */
  picking: boolean;
  onPick: (p: Pt) => void;
  onHover: (p: Pt | null) => void;
  label: string;
};

/** Bild in Rastergröße als Canvas, zum Zeichnen in beliebiger Vergrößerung */
const toCanvas = (cols: number, rows: number, pixels: Uint8ClampedArray) => {
  if (typeof document === "undefined") return null;
  const canvas = document.createElement("canvas");
  canvas.width = cols;
  canvas.height = rows;
  const ctx = canvas.getContext("2d");
  if (!ctx) return null;
  ctx.putImageData(new ImageData(new Uint8ClampedArray(pixels), cols, rows), 0, 0);
  return canvas;
};

/** Tastatur: Pfeile verschieben, + und − zoomen, 0 passt ein; null für andere Tasten */
const keyMove = (key: string, view: Viewport, grid: HeightGrid): Viewport | null => {
  const step = 40;
  switch (key) {
    case "ArrowLeft":
      return pan(view, step, 0);
    case "ArrowRight":
      return pan(view, -step, 0);
    case "ArrowUp":
      return pan(view, 0, step);
    case "ArrowDown":
      return pan(view, 0, -step);
    case "+":
      return zoomAt(view, view.width / 2, view.height / 2, 1.25);
    case "-":
      return zoomAt(view, view.width / 2, view.height / 2, 0.8);
    case "0":
      return fitTo(bboxOf(grid), view.width, view.height);
    default:
      return null;
  }
};

const cssVar = (name: string) =>
  getComputedStyle(document.documentElement).getPropertyValue(name).trim();

export function MapCanvas({
  grid,
  range,
  overlay,
  outlines,
  lines,
  pins,
  plan,
  ortho,
  shape,
  picking,
  onPick,
  onHover,
  label,
}: Props) {
  const wrap = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [view, setView] = useState<Viewport | null>(null);
  const drag = useRef<{ x: number; y: number; moved: boolean } | null>(null);
  const base = useMemo(
    () => toCanvas(grid.cols, grid.rows, renderHeights(grid, range)),
    [grid, range],
  );
  const over = useMemo(
    () =>
      overlay
        ? toCanvas(
            overlay.grid.cols,
            overlay.grid.rows,
            renderDeviation(overlay.grid, overlay.limit, overlay.removedIsNegative),
          )
        : null,
    [overlay],
  );

  // Größe folgt dem Container; bei neuem Raster wieder einpassen
  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const resize = () => {
      const { width } = el.getBoundingClientRect();
      const height = Math.max(280, Math.min(560, width * 0.62));
      setView((v) => (v && v.width === width ? v : fitTo(bboxOf(grid), width, height)));
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(el);
    return () => ro.disconnect();
  }, [grid]);

  useEffect(() => setView((v) => (v ? fitTo(bboxOf(grid), v.width, v.height) : v)), [grid]);

  useEffect(() => {
    const c = canvas.current;
    if (!c || !view) return;
    const dpr = window.devicePixelRatio || 1;
    c.width = view.width * dpr;
    c.height = view.height * dpr;
    const ctx = c.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, view.width, view.height);
    const drawGrid = (img: HTMLCanvasElement | null, g: HeightGrid, alpha: number) => {
      if (!img) return;
      const b = bboxOf(g);
      const [x0, y0] = toScreen(view, b.minX, b.maxY);
      ctx.globalAlpha = alpha;
      ctx.imageSmoothingEnabled = view.scale < 1 / g.cell;
      ctx.drawImage(img, x0, y0, (b.maxX - b.minX) * view.scale, (b.maxY - b.minY) * view.scale);
      ctx.globalAlpha = 1;
    };
    if (ortho) {
      const [a, b, c2, d, e, f] = orthoTransform(ortho.corners, ortho.width, ortho.height, view);
      ctx.setTransform(dpr * a, dpr * b, dpr * c2, dpr * d, dpr * e, dpr * f);
      ctx.imageSmoothingEnabled = true;
      ctx.drawImage(ortho.image, 0, 0);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    } else drawGrid(base, grid, 1);
    if (overlay) drawGrid(over, overlay.grid, 0.85);
    const fg = cssVar("--fg") || "black";
    const bg = cssVar("--bg") || "white";
    const path = (pts: Pt[], closed: boolean) => {
      ctx.beginPath();
      pts.forEach(([x, y], i) => {
        const [sx, sy] = toScreen(view, x, y);
        if (i === 0) ctx.moveTo(sx, sy);
        else ctx.lineTo(sx, sy);
      });
      if (closed) ctx.closePath();
    };
    ctx.font = "12px Geist Variable, system-ui, sans-serif";
    if (plan) {
      ctx.lineWidth = 1;
      ctx.strokeStyle = fg;
      ctx.globalAlpha = 0.75;
      for (const pth of plan.paths) {
        path(pth.points, pth.closed);
        ctx.stroke();
      }
      ctx.globalAlpha = 1;
      // Texte erst, wenn sie groß genug sind (Plan-Texthöhe in m × Maßstab)
      ctx.fillStyle = fg;
      for (const l of plan.labels) {
        if (l.height * view.scale < 9) continue;
        const [sx, sy] = toScreen(view, l.at[0], l.at[1]);
        ctx.fillText(l.text, sx, sy);
      }
    }
    for (const o of outlines) {
      if (o.points.length < 3) continue;
      path(o.points, true);
      ctx.lineWidth = 3;
      ctx.strokeStyle = bg;
      ctx.stroke();
      ctx.lineWidth = 1.5;
      ctx.strokeStyle = fg;
      ctx.stroke();
      const [sx, sy] = toScreen(view, o.points[0]![0], o.points[0]![1]);
      // heller Rand um die Schrift, damit sie auf jeder Farbe lesbar bleibt
      ctx.lineWidth = 3;
      ctx.lineJoin = "round";
      ctx.strokeStyle = bg;
      ctx.strokeText(o.name, sx + 4, sy - 4);
      ctx.fillStyle = fg;
      ctx.fillText(o.name, sx + 4, sy - 4);
    }
    const accent = cssVar("--accent") || "blue";
    for (const l of lines) {
      if (l.points.length < 2) continue;
      path(l.points, false);
      ctx.lineJoin = "round";
      ctx.lineWidth = 5;
      ctx.strokeStyle = bg;
      ctx.stroke();
      ctx.lineWidth = 2.5;
      ctx.strokeStyle = accent;
      ctx.stroke();
      const [sx, sy] = toScreen(view, l.points[0]![0], l.points[0]![1]);
      ctx.lineWidth = 3;
      ctx.strokeStyle = bg;
      ctx.strokeText(l.name, sx + 6, sy - 6);
      ctx.fillStyle = fg;
      ctx.fillText(l.name, sx + 6, sy - 6);
    }
    for (const p of pins) {
      const [sx, sy] = toScreen(view, p.at[0], p.at[1]);
      ctx.beginPath();
      if (p.diamond) {
        ctx.moveTo(sx, sy - 7);
        ctx.lineTo(sx + 7, sy);
        ctx.lineTo(sx, sy + 7);
        ctx.lineTo(sx - 7, sy);
        ctx.closePath();
      } else ctx.arc(sx, sy, 5.5, 0, Math.PI * 2);
      ctx.fillStyle = accent;
      ctx.fill();
      ctx.lineWidth = 2;
      ctx.strokeStyle = bg;
      ctx.stroke();
      ctx.lineWidth = 3;
      ctx.strokeText(p.label, sx + 9, sy + 4);
      ctx.fillStyle = fg;
      ctx.fillText(p.label, sx + 9, sy + 4);
    }
    if (shape.points.length) {
      path(shape.points, shape.closed && shape.points.length > 2);
      if (shape.closed && shape.points.length > 2) {
        ctx.fillStyle = `color-mix(in srgb, ${accent} 18%, transparent)`;
        ctx.fill();
      }
      ctx.lineWidth = 2;
      ctx.strokeStyle = accent;
      ctx.stroke();
      for (const [x, y] of shape.points) {
        const [sx, sy] = toScreen(view, x, y);
        ctx.beginPath();
        ctx.arc(sx, sy, 4, 0, Math.PI * 2);
        ctx.fillStyle = bg;
        ctx.fill();
        ctx.stroke();
      }
    }
    if (picking) {
      // Fadenkreuz in der Mitte für die Tastatur
      const [cx, cy] = [view.width / 2, view.height / 2];
      ctx.strokeStyle = fg;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(cx - 8, cy);
      ctx.lineTo(cx + 8, cy);
      ctx.moveTo(cx, cy - 8);
      ctx.lineTo(cx, cy + 8);
      ctx.stroke();
    }
  }, [view, base, over, grid, overlay, outlines, lines, pins, plan, ortho, shape, picking]);

  if (!view) return <div ref={wrap} className="h-72 w-full" />;
  const bar = scaleBar(view);
  const local = (e: PointerEvent) => {
    const r = canvas.current!.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top] as const;
  };
  const onKey = (e: KeyboardEvent) => {
    const next = keyMove(e.key, view, grid);
    if (next) {
      e.preventDefault();
      setView(next);
    } else if (e.key === "Enter" && picking) {
      e.preventDefault();
      onPick(toWorld(view, view.width / 2, view.height / 2));
    }
  };

  return (
    <div
      ref={wrap}
      // biome-ignore lint/a11y/noNoninteractiveTabindex: die Karte ist mit der Tastatur bedienbar (onKeyDown)
      tabIndex={0}
      role="application"
      aria-label={label}
      aria-roledescription={m.view_canvas_role()}
      onKeyDown={onKey}
      className="relative w-full overflow-hidden rounded-lg border border-border bg-subtle"
    >
      <canvas
        ref={canvas}
        style={{ width: view.width, height: view.height, touchAction: "none" }}
        className={picking ? "block cursor-crosshair" : "block cursor-grab"}
        onWheel={(e) => {
          const r = canvas.current!.getBoundingClientRect();
          setView(
            zoomAt(view, e.clientX - r.left, e.clientY - r.top, e.deltaY < 0 ? 1.2 : 1 / 1.2),
          );
        }}
        onPointerDown={(e) => {
          const [x, y] = local(e);
          drag.current = { x, y, moved: false };
          canvas.current?.setPointerCapture(e.pointerId);
        }}
        onPointerMove={(e) => {
          const [x, y] = local(e);
          onHover(toWorld(view, x, y));
          const d = drag.current;
          if (!d) return;
          if (Math.hypot(x - d.x, y - d.y) > 3) d.moved = true;
          if (d.moved) {
            setView(pan(view, x - d.x, y - d.y));
            drag.current = { x, y, moved: true };
          }
        }}
        onPointerUp={(e) => {
          const d = drag.current;
          drag.current = null;
          if (d && !d.moved && picking) {
            const [x, y] = local(e);
            onPick(toWorld(view, x, y));
          }
        }}
        onPointerLeave={() => onHover(null)}
      />
      <div className="pointer-events-none absolute bottom-2 left-2 flex flex-col gap-1 rounded bg-bg/85 px-2 py-1 text-[12px]">
        <span className="block h-1 bg-fg" style={{ width: bar.px }} />
        <span className="tabular">{m.view_scale({ meters: bar.meters })}</span>
      </div>
    </div>
  );
}
