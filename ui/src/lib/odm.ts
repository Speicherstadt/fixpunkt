/**
 * Drohnenfotos zu NodeODM: Entwurf anlegen, Fotos nacheinander hochladen (Fortschritt nach Bytes), starten.
 * Die Aufrufe kommen als Abhängigkeiten herein, damit der Ablauf ohne Server testbar ist.
 */
import type { Job, OdmParams } from "@/lib/api";

export const IMAGE_ACCEPT = ".jpg,.jpeg,.tif,.tiff,.png";
export const MIN_IMAGES = 3;

export type OdmDeps = {
  create: (params: OdmParams) => Promise<Job>;
  upload: (jobId: number, file: File, onProgress: (fraction: number) => void) => Promise<unknown>;
  start: (jobId: number) => Promise<Job>;
};

export type UploadProgress = { done: number; total: number; fraction: number };

/** Nur Fotos, die NodeODM versteht */
export const photosOnly = (files: File[]): File[] =>
  files.filter((f) => /\.(jpe?g|tiff?|png)$/i.test(f.name));

/** Höhenversatz aus dem Eingabefeld: Komma oder Punkt, leer heißt 0; null, wenn ungültig */
export const parseOffset = (text: string): number | null => {
  const t = text.trim().replace(",", ".");
  if (t === "") return 0;
  const v = Number(t);
  return Number.isFinite(v) && Math.abs(v) <= 200 ? v : null;
};

/** Ablauf; gibt den gestarteten Auftrag. Bricht ein Upload ab, bleibt der Entwurf zum Löschen stehen. */
export const uploadFlight = async (
  deps: OdmDeps,
  params: OdmParams,
  files: File[],
  onProgress: (p: UploadProgress) => void,
): Promise<Job> => {
  const job = await deps.create(params);
  const total = files.reduce((s, f) => s + f.size, 0) || 1;
  let sent = 0;
  for (const [i, file] of files.entries()) {
    await deps.upload(job.id, file, (f) =>
      onProgress({ done: i, total: files.length, fraction: (sent + f * file.size) / total }),
    );
    sent += file.size;
    onProgress({ done: i + 1, total: files.length, fraction: sent / total });
  }
  return deps.start(job.id);
};
