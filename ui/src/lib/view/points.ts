/**
 * Punktwolke für die 3D-Ansicht (Format aus crates/server/src/points.rs): Kopf mit Anzahl, Farbe und Ursprung,
 * dann Float32 x y z relativ zum Ursprung, dann RGB. Für three.js: x = Ost, y = Höhe × Überhöhung, z = Süd.
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
};

const HEADER = 29;

export const parsePoints = (buffer: ArrayBuffer): PointData => {
  const v = new DataView(buffer);
  if (buffer.byteLength < HEADER) throw new ApiError(0, "api.internal", "points header");
  const count = v.getUint32(0, true);
  const colored = v.getUint8(4) === 1;
  const origin: [number, number, number] = [
    v.getFloat64(5, true),
    v.getFloat64(13, true),
    v.getFloat64(21, true),
  ];
  const size = HEADER + count * 12 + (colored ? count * 3 : 0);
  if (buffer.byteLength !== size) throw new ApiError(0, "api.internal", "points size");
  // Float32Array braucht eine durch 4 teilbare Startposition: kopieren
  const xyz = new Float32Array(buffer.slice(HEADER, HEADER + count * 12));
  const rgb = colored ? new Uint8Array(buffer, HEADER + count * 12, count * 3) : null;
  return { count, colored, origin, xyz, rgb };
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

/** Positionen und Farben für three.js; ohne Farben aus der Datei nach Höhe eingefärbt */
export const pointScene = (p: PointData, exaggeration = 1): PointCloudScene => {
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
  for (let i = 0; i < p.count; i++) {
    const c = p.rgb
      ? [p.rgb[i * 3]!, p.rgb[i * 3 + 1]!, p.rgb[i * 3 + 2]!]
      : heightColor(maxZ > 0 ? p.xyz[i * 3 + 2]! / maxZ : 0.5);
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
