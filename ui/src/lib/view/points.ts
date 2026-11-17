/**
 * Punktwolke für die 3D-Ansicht (Format aus crates/server/src/points.rs): Kopf mit Anzahl, Merkern (1 Farbe,
 * 2 Intensität) und Ursprung, dann Float32 x y z relativ zum Ursprung, RGB, Intensität als u16. Für three.js:
 * x = Ost, y = Höhe × Überhöhung, z = Süd.
 */
import { ApiError, errorOf } from "@/lib/api";
import { heightColor } from "./colors";

export type PointData = {
  count: number;
  colored: boolean;
  origin: [number, number, number];
  /** x y z je Punkt, relativ zum Ursprung */
  xyz: Float32Array;
  rgb: Uint8Array | null;
  /** 0…65535, auf den Bereich der Datei gestreckt */
  intensity: Uint16Array | null;
};

/** Wonach die Punkte gefärbt werden */
export type ColorBy = "file" | "height" | "intensity";

const HEADER = 29;

export const parsePoints = (buffer: ArrayBuffer): PointData => {
  const v = new DataView(buffer);
  if (buffer.byteLength < HEADER) throw new ApiError(0, "api.internal", "points header");
  const count = v.getUint32(0, true);
  const flags = v.getUint8(4);
  const colored = (flags & 1) !== 0;
  const hasIntensity = (flags & 2) !== 0;
  const origin: [number, number, number] = [
    v.getFloat64(5, true),
    v.getFloat64(13, true),
    v.getFloat64(21, true),
  ];
  const rgbAt = HEADER + count * 12;
  const intensityAt = rgbAt + (colored ? count * 3 : 0);
  const size = intensityAt + (hasIntensity ? count * 2 : 0);
  if (flags > 3 || buffer.byteLength !== size) throw new ApiError(0, "api.internal", "points size");
  // Float32Array braucht eine durch 4 teilbare Startposition: kopieren
  const xyz = new Float32Array(buffer.slice(HEADER, HEADER + count * 12));
  const rgb = colored ? new Uint8Array(buffer, rgbAt, count * 3) : null;
  // Uint16Array braucht eine gerade Startposition: kopieren
  const intensity = hasIntensity ? new Uint16Array(buffer.slice(intensityAt, size)) : null;
  return { count, colored, origin, xyz, rgb, intensity };
};

export const fetchPoints = async (url: string): Promise<PointData> => {
  const res = await fetch(url, { credentials: "same-origin" });
  if (!res.ok) throw await errorOf(res);
  return parsePoints(await res.arrayBuffer());
};

export type PointCloudScene = {
  positions: Float32Array;
  colors: Float32Array;
  /** Breite (Ost), Tiefe (Nord), Höhe (überhöht) in m */
  size: [number, number, number];
};

/** Was eine Punktwolke an Färbungen hergibt; die erste ist die Vorgabe */
export const colorModes = (p: Pick<PointData, "rgb" | "intensity">): ColorBy[] => [
  ...(p.rgb ? (["file"] as const) : []),
  ...(p.intensity ? (["intensity"] as const) : []),
  "height",
];

/** 2. und 98. Perzentil: einzelne Glanzlichter sollen nicht alles grau machen */
export const intensityRange = (v: Uint16Array): [number, number] => {
  const counts = new Uint32Array(65536);
  for (const x of v) counts[x]!++;
  const at = (q: number) => {
    const target = q * v.length;
    let sum = 0;
    for (let i = 0; i < counts.length; i++) {
      sum += counts[i]!;
      if (sum > target) return i;
    }
    return 65535;
  };
  const [lo, hi] = [at(0.02), at(0.98)];
  return hi > lo ? [lo, hi] : [0, 65535];
};

/** Positionen und Farben für three.js: Farbe aus der Datei, nach Intensität (grau) oder nach Höhe */
export const pointScene = (
  p: PointData,
  exaggeration = 1,
  colorBy: ColorBy = colorModes(p)[0]!,
): PointCloudScene => {
  const positions = new Float32Array(p.count * 3);
  const colors = new Float32Array(p.count * 3);
  let [minX, maxX, minY, maxY, maxZ] = [Infinity, -Infinity, Infinity, -Infinity, 0];
  for (let i = 0; i < p.count; i++) {
    const [x, y, z] = [p.xyz[i * 3]!, p.xyz[i * 3 + 1]!, p.xyz[i * 3 + 2]!];
    minX = Math.min(minX, x);
    maxX = Math.max(maxX, x);
    minY = Math.min(minY, y);
    maxY = Math.max(maxY, y);
    maxZ = Math.max(maxZ, z);
    positions.set([x, z * exaggeration, -y], i * 3);
  }
  const [lo, hi] = p.intensity && colorBy === "intensity" ? intensityRange(p.intensity) : [0, 1];
  for (let i = 0; i < p.count; i++) {
    let c: number[];
    if (colorBy === "file" && p.rgb) c = [p.rgb[i * 3]!, p.rgb[i * 3 + 1]!, p.rgb[i * 3 + 2]!];
    else if (colorBy === "intensity" && p.intensity) {
      const g = Math.round(Math.min(1, Math.max(0, (p.intensity[i]! - lo) / (hi - lo))) * 255);
      c = [g, g, g];
    } else c = heightColor(maxZ > 0 ? p.xyz[i * 3 + 2]! / maxZ : 0.5);
    colors.set(
      c.map((v) => v / 255),
      i * 3,
    );
  }
  return {
    positions,
    colors,
    size: p.count ? [maxX - minX, maxY - minY, maxZ * exaggeration] : [1, 1, 0],
  };
};
