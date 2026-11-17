/**
 * Zugriff auf die API des Rust-Servers. Ändernde Anfragen tragen das CSRF-Token der Sitzung im
 * Header; Fehler kommen als Schlüssel (`{"error":"auth.wrongCredentials"}`) und werden in der UI
 * übersetzt (src/lib/errors.ts). Antworten werden mit Zod geprüft.
 */
import { z } from "zod";

export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    readonly detail?: string,
  ) {
    super(code);
  }
}

let csrf: string | null = null;
export const setCsrf = (token: string | null) => {
  csrf = token;
};

const SAFE = new Set(["GET", "HEAD", "OPTIONS"]);

/** Header einer Anfrage: JSON, bei ändernden Methoden das CSRF-Token */
export const requestHeaders = (method: string, json: boolean): Record<string, string> => ({
  ...(json ? { "content-type": "application/json" } : {}),
  ...(!SAFE.has(method) && csrf ? { "x-csrf-token": csrf } : {}),
});

/** Fehler aus einer Antwort lesen; ohne JSON-Körper der HTTP-Status */
export const errorOf = async (res: Response): Promise<ApiError> => {
  const body = await res.json().catch(() => null);
  const parsed = z.object({ error: z.string(), detail: z.string().optional() }).safeParse(body);
  return parsed.success
    ? new ApiError(res.status, parsed.data.error, parsed.data.detail)
    : new ApiError(res.status, `http.${res.status}`);
};

export const request = async <T>(
  method: string,
  path: string,
  schema: z.ZodType<T>,
  body?: unknown,
): Promise<T> => {
  const res = await fetch(`/api${path}`, {
    method,
    headers: requestHeaders(method, body !== undefined),
    body: body === undefined ? undefined : JSON.stringify(body),
    credentials: "same-origin",
  });
  if (!res.ok) throw await errorOf(res);
  if (res.status === 204) return schema.parse(undefined);
  return schema.parse(await res.json());
};

// ---- Schemas ---------------------------------------------------------------------------

export const Role = z.enum(["viewer", "editor", "admin"]);
export type Role = z.infer<typeof Role>;

export const CurrentUser = z.object({ id: z.number(), name: z.string(), role: Role });
export type CurrentUser = z.infer<typeof CurrentUser>;

export const AuthState = z.object({
  setupRequired: z.boolean(),
  user: CurrentUser.nullable(),
  csrf: z.string().nullable(),
});
export type AuthState = z.infer<typeof AuthState>;

export const User = z.object({
  id: z.number(),
  name: z.string(),
  role: Role,
  createdAt: z.number(),
});
export type User = z.infer<typeof User>;

export const Site = z.object({
  id: z.number(),
  name: z.string(),
  description: z.string(),
  epsg: z.number(),
  createdAt: z.number(),
});
export type Site = z.infer<typeof Site>;

export const SiteItem = Site.extend({
  captures: z.number(),
  jobs: z.number(),
  lastCaptureAt: z.number().nullable(),
});
export type SiteItem = z.infer<typeof SiteItem>;

export const CaptureInfo = z
  .object({
    type: z.enum(["pointcloud", "raster"]),
    points: z.number().optional(),
    cell: z.number().optional(),
    zMin: z.number().nullable().optional(),
    zMax: z.number().nullable().optional(),
    densityPerM2: z.number().optional(),
  })
  .loose();

export const Capture = z.object({
  id: z.number(),
  siteId: z.number(),
  name: z.string(),
  fileName: z.string(),
  fileSize: z.number(),
  capturedAt: z.number(),
  status: z.string(),
  info: CaptureInfo.nullable(),
  error: z.string().nullable(),
  createdAt: z.number(),
});
export type Capture = z.infer<typeof Capture>;

export const Region = z.object({
  id: z.number(),
  siteId: z.number(),
  name: z.string(),
  /** GeoJSON-Feature im metrischen System der Baustelle (crates/server/src/compute/mod.rs) */
  geojson: z.unknown(),
  areaM2: z.number(),
  createdAt: z.number(),
});
export type Region = z.infer<typeof Region>;

export const VolumeReport = z.object({
  area_m2: z.number(),
  cut_m3: z.number(),
  fill_m3: z.number(),
  net_m3: z.number(),
  cells_used: z.number(),
  cells_missing: z.number(),
  cell_size_m: z.number(),
});
export type VolumeReport = z.infer<typeof VolumeReport>;

export const RegionReport = VolumeReport.extend({ name: z.string(), boundary_area_m2: z.number() });
export type RegionReport = z.infer<typeof RegionReport>;

export const VolumeResult = z.object({
  regions: z.array(RegionReport),
  total: VolumeReport,
  cell: z.number(),
  files: z.array(z.string()).optional(),
});
export type VolumeResult = z.infer<typeof VolumeResult>;

export const HistogramBin = z.object({
  from: z.number().nullable(),
  to: z.number().nullable(),
  cells: z.number(),
  percent: z.number(),
});

export const Deviation = z.object({
  min: z.number(),
  max: z.number(),
  mean: z.number(),
  rms: z.number(),
  std_dev: z.number(),
  tolerance: z.number(),
  within_tolerance_percent: z.number(),
  above_tolerance_percent: z.number(),
  below_tolerance_percent: z.number(),
  histogram: z.array(HistogramBin),
  volume: VolumeReport,
});
export type Deviation = z.infer<typeof Deviation>;

export const CompareResult = z.object({
  deviation: Deviation,
  region: z.string().nullable(),
  cell: z.number(),
  files: z.array(z.string()),
});
export type CompareResult = z.infer<typeof CompareResult>;

export const SeriesStep = z.object({
  label: z.string(),
  since_start: VolumeReport,
  since_previous: VolumeReport,
});
export type SeriesStep = z.infer<typeof SeriesStep>;

export const SeriesResult = z.object({
  base: z.string(),
  region: z.string().nullable(),
  steps: z.array(SeriesStep),
  cell: z.number(),
});
export type SeriesResult = z.infer<typeof SeriesResult>;

export const FilesResult = z.object({ files: z.array(z.string()) }).loose();
export type FilesResult = z.infer<typeof FilesResult>;

export const Job = z.object({
  id: z.number(),
  siteId: z.number(),
  kind: z.string(),
  params: z.record(z.string(), z.unknown()),
  status: z.string(),
  result: z.unknown().nullable(),
  error: z.string().nullable(),
  createdAt: z.number(),
  startedAt: z.number().nullable(),
  finishedAt: z.number().nullable(),
});
export type Job = z.infer<typeof Job>;

const Empty = z.undefined();

// ---- Aufrufe ---------------------------------------------------------------------------

export const api = {
  authState: () => request("GET", "/auth/state", AuthState),
  setup: (name: string, password: string) =>
    request("POST", "/auth/setup", Empty, { name, password }),
  login: (name: string, password: string) =>
    request("POST", "/auth/login", Empty, { name, password }),
  logout: () => request("POST", "/auth/logout", Empty, {}),
  changePassword: (current: string, next: string) =>
    request("POST", "/auth/password", Empty, { current, new: next }),
  users: () => request("GET", "/users", z.array(User)),
  addUser: (name: string, password: string, role: Role) =>
    request("POST", "/users", z.object({ id: z.number() }), { name, password, role }),
  deleteUser: (id: number) => request("DELETE", `/users/${id}`, Empty),
  sites: () => request("GET", "/sites", z.array(SiteItem)),
  site: (id: number) => request("GET", `/sites/${id}`, Site),
  createSite: (name: string, description: string) =>
    request("POST", "/sites", Site, { name, description }),
  deleteSite: (id: number) => request("DELETE", `/sites/${id}`, Empty),
  captures: (siteId: number) => request("GET", `/sites/${siteId}/captures`, z.array(Capture)),
  deleteCapture: (id: number) => request("DELETE", `/captures/${id}`, Empty),
  regions: (siteId: number) => request("GET", `/sites/${siteId}/regions`, z.array(Region)),
  addRegions: (siteId: number, geojson: unknown, name?: string) =>
    request("POST", `/sites/${siteId}/regions`, z.array(Region), { geojson, name }),
  deleteRegion: (id: number) => request("DELETE", `/regions/${id}`, Empty),
  jobs: (siteId: number) => request("GET", `/sites/${siteId}/jobs`, z.array(Job)),
  job: (id: number) => request("GET", `/jobs/${id}`, Job),
  createJob: (siteId: number, kind: string, params: Record<string, unknown>) =>
    request("POST", `/sites/${siteId}/jobs`, Job, { kind, params }),
  deleteJob: (id: number) => request("DELETE", `/jobs/${id}`, Empty),
};

/** Datei hochladen, mit Fortschritt (fetch kennt keinen Upload-Fortschritt) */
export const uploadCapture = (
  siteId: number,
  file: File,
  name: string,
  onProgress: (fraction: number) => void,
): Promise<Capture> =>
  new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    const query = new URLSearchParams({ file: file.name, name });
    xhr.open("PUT", `/api/sites/${siteId}/captures?${query}`);
    for (const [k, v] of Object.entries(requestHeaders("PUT", false))) xhr.setRequestHeader(k, v);
    xhr.setRequestHeader("content-type", "application/octet-stream");
    xhr.upload.onprogress = (e) => e.lengthComputable && onProgress(e.loaded / e.total);
    xhr.onload = () => {
      const res = new Response(xhr.responseText, { status: xhr.status });
      if (xhr.status >= 200 && xhr.status < 300)
        res.json().then((j) => resolve(Capture.parse(j)), reject);
      else errorOf(res).then(reject, reject);
    };
    xhr.onerror = () => reject(new ApiError(0, "http.0"));
    xhr.send(file);
  });
