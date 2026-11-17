/**
 * Fortsetzbarer Upload (tus) für große Dateien: Stücke à 8 MB, bei Abbruch automatisch neu versuchen, nach einem
 * Neuladen der Seite dort weitermachen, wo es aufgehört hat (gleiche Datei, gleiches Ziel).
 * Server: crates/server/src/uploads.rs
 */
import * as tus from "tus-js-client";
import { ApiError, requestHeaders } from "@/lib/api";

export const CHUNK_SIZE = 8 * 1024 * 1024;
export const RETRY_DELAYS = [0, 1000, 3000, 5000, 10000, 20000];

/** Wohin die Datei geht – wie `Target` in uploads.rs */
export type UploadTarget =
  | { target: "capture"; siteId: number; name: string; capturedAt?: number }
  | { target: "image"; jobId: number }
  | { target: "ortho"; captureId: number };

/** Metadaten für den Server, alles als Text */
export const metadataOf = (fileName: string, t: UploadTarget): Record<string, string> => {
  const m: Record<string, string> = { filename: fileName };
  for (const [k, v] of Object.entries(t)) if (v !== undefined) m[k] = String(v);
  return m;
};

/**
 * Schlüssel zum Fortsetzen: Datei (Name, Größe, Änderungszeit) und Ziel. Ohne das Ziel würde dieselbe Datei für
 * eine zweite Baustelle den alten Upload fortsetzen.
 */
export const fingerprintOf = (
  file: { name: string; size: number; lastModified: number },
  t: UploadTarget,
) => ["fixpunkt", file.name, file.size, file.lastModified, JSON.stringify(t)].join("|");

/** Fehler von tus als ApiError, damit die UI ihn übersetzt */
const toApiError = (e: Error): ApiError => {
  const res = (e as tus.DetailedError).originalResponse;
  if (!res) return new ApiError(0, "http.0", e.message);
  try {
    const body = JSON.parse(res.getBody() ?? "");
    if (typeof body.error === "string")
      return new ApiError(res.getStatus(), body.error, body.detail);
  } catch {
    // kein JSON
  }
  return new ApiError(res.getStatus(), `http.${res.getStatus()}`);
};

export const resumableUpload = (
  file: File,
  target: UploadTarget,
  onProgress: (fraction: number) => void,
): Promise<void> =>
  new Promise((resolve, reject) => {
    const upload = new tus.Upload(file, {
      endpoint: "/api/uploads",
      chunkSize: CHUNK_SIZE,
      retryDelays: RETRY_DELAYS,
      metadata: metadataOf(file.name, target),
      fingerprint: (f) => Promise.resolve(fingerprintOf(f as File, target)),
      removeFingerprintOnSuccess: true,
      // CSRF-Kopf bei jeder Anfrage frisch, falls sich die Sitzung erneuert hat
      onBeforeRequest: (req) => {
        for (const [k, v] of Object.entries(requestHeaders(req.getMethod(), false)))
          req.setHeader(k, v);
      },
      // Eingaben des Nutzers (falsches Ziel, falsches Format) nicht wiederholen
      onShouldRetry: (err) => {
        const status = (err as tus.DetailedError).originalResponse?.getStatus() ?? 0;
        return status === 0 || status >= 500 || status === 409 || status === 423;
      },
      onProgress: (sent, total) => onProgress(total ? sent / total : 0),
      onSuccess: () => resolve(),
      onError: (e) => reject(toApiError(e)),
    });
    upload
      .findPreviousUploads()
      .then((previous) => {
        if (previous[0]) upload.resumeFromPreviousUpload(previous[0]);
        upload.start();
      })
      .catch(() => upload.start());
  });
