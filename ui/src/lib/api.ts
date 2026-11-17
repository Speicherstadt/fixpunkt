/**
 * Zugriff auf die API des Rust-Servers. Ändernde Anfragen tragen das CSRF-Token der Sitzung im
 * Header; Fehler kommen als Schlüssel (`{"error":"auth.wrongCredentials"}`) und werden in der UI
 * übersetzt (src/lib/errors.ts). Antworten werden mit Zod geprüft.
 */
import { z } from "zod";
import { FitReport } from "@/lib/georef";
import { resumableUpload } from "@/lib/resumable";

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
  /** [Breite, Länge] für die Karte; ohne geprüfte Daten leer */
  center: z.tuple([z.number(), z.number()]).nullable().optional(),
});

export const MarkerKind = z.enum(["constraint", "photo", "note"]);
export type MarkerKind = z.infer<typeof MarkerKind>;

/** Markierung der Dokumentation (Zwangspunkt, Foto, Notiz) */
export const Marker = z.object({
  id: z.number(),
  siteId: z.number(),
  kind: MarkerKind,
  title: z.string(),
  note: z.string(),
  x: z.number().nullable(),
  y: z.number().nullable(),
  z: z.number().nullable(),
  photo: z.string().nullable(),
  takenAt: z.number().nullable(),
  createdAt: z.number(),
});
export type Marker = z.infer<typeof Marker>;

export type NewMarker = {
  kind: MarkerKind;
  title: string;
  note: string;
  x?: number;
  y?: number;
  z?: number;
};

export const Plan = z.object({
  id: z.number(),
  siteId: z.number(),
  name: z.string(),
  fileName: z.string(),
  info: z.object({
    layers: z.array(z.string()),
    paths: z.number(),
    spots: z.number(),
    labels: z.number(),
    zoneOffset: z.number(),
    bbox: z.tuple([z.number(), z.number(), z.number(), z.number()]).nullable(),
  }),
  createdAt: z.number(),
});
export type Plan = z.infer<typeof Plan>;

/** Aufbereiteter DXF-Plan: Linienzüge, Texte, Höhen (Koordinaten ohne Zonenkennung) */
export const PlanGeometry = z.object({
  paths: z.array(
    z.object({
      layer: z.string(),
      points: z.array(z.tuple([z.number(), z.number()])),
      closed: z.boolean(),
    }),
  ),
  labels: z.array(
    z.object({
      layer: z.string(),
      at: z.tuple([z.number(), z.number()]),
      text: z.string(),
      height: z.number(),
    }),
  ),
  spots: z.array(
    z.object({ layer: z.string(), at: z.tuple([z.number(), z.number(), z.number()]) }),
  ),
  layers: z.array(z.string()),
});
export type PlanGeometry = z.infer<typeof PlanGeometry>;

/** Überblick einer Baustelle (crates/server/src/dashboard.rs) */
const FlightLine = z.object({
  start: z.tuple([z.number(), z.number()]),
  end: z.tuple([z.number(), z.number()]),
});

/** Flugplan je Bereich (crates/server/src/flightplan.rs) */
export const FlightPlan = z.object({
  region: z.string(),
  camera: z.string(),
  plan: z.object({
    altitude: z.number(),
    lineSpacing: z.number(),
    photoSpacing: z.number(),
    speed: z.number(),
    lines: z.array(FlightLine),
    length: z.number(),
    duration: z.number(),
    photos: z.number(),
    area: z.number(),
  }),
  terrain: z.object({ capture: z.string(), min: z.number(), max: z.number() }).nullable(),
});
export type FlightPlan = z.infer<typeof FlightPlan>;

export const Dashboard = z.object({
  captures: z.object({
    count: z.number(),
    ready: z.number(),
    last: z.object({ id: z.number(), name: z.string(), at: z.number() }).nullable(),
  }),
  progress: z.array(
    z.object({
      jobId: z.number(),
      captureId: z.number(),
      name: z.string(),
      at: z.number(),
      cut: z.number(),
      fill: z.number(),
      net: z.number(),
    }),
  ),
  latestCompare: z
    .object({
      jobId: z.number(),
      at: z.number(),
      within: z.number().nullable(),
      tolerance: z.number().nullable(),
    })
    .nullable(),
  jobs: z.object({ queued: z.number(), running: z.number(), failed: z.number() }),
  constraints: z.object({
    total: z.number(),
    checked: z.number(),
    violated: z.array(z.object({ id: z.number(), title: z.string(), diff: z.number() })),
    against: z.string().nullable(),
  }),
  documentation: z.object({ photos: z.number(), notes: z.number() }),
  plans: z.number(),
});
export type Dashboard = z.infer<typeof Dashboard>;

export const Settings = z.object({
  mapTiles: z.object({ url: z.string(), attribution: z.string() }).nullable(),
});
export type Settings = z.infer<typeof Settings>;
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
  /** Orthofoto: Bildgröße und Ecken (links oben, rechts oben, links unten) im System der Baustelle */
  ortho: z
    .object({
      width: z.number(),
      height: z.number(),
      corners: z.tuple([
        z.tuple([z.number(), z.number()]),
        z.tuple([z.number(), z.number()]),
        z.tuple([z.number(), z.number()]),
      ]),
    })
    .nullable()
    .optional(),
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
  /** 0–100 bei Photogrammetrie, sonst leer */
  progress: z.number().nullable().optional(),
});
export type Job = z.infer<typeof Job>;

/** NodeODM: eingerichtet, erreichbar, Version und Warteschlange */
export const OdmStatus = z.object({
  configured: z.boolean(),
  reachable: z.boolean().optional(),
  version: z.string().optional(),
  queue: z.number().optional(),
  error: z.string().optional(),
});
export type OdmStatus = z.infer<typeof OdmStatus>;

export const OdmQuality = z.enum(["fast", "standard", "high"]);
export type OdmQuality = z.infer<typeof OdmQuality>;

export type OdmParams = {
  name: string;
  capturedAt: number;
  quality: OdmQuality;
  heightOffset: number;
};

/** Ergebnis der Photogrammetrie */
export const OdmResult = z.object({
  captureId: z.number(),
  images: z.number(),
  processingMs: z.number(),
  sourceEpsg: z.number().nullable(),
  converted: z.boolean(),
  heightOffset: z.number(),
});
export type OdmResult = z.infer<typeof OdmResult>;

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
  deleteOrtho: (captureId: number) => request("DELETE", `/captures/${captureId}/ortho`, Empty),
  regions: (siteId: number) => request("GET", `/sites/${siteId}/regions`, z.array(Region)),
  addRegions: (siteId: number, geojson: unknown, name?: string) =>
    request("POST", `/sites/${siteId}/regions`, z.array(Region), { geojson, name }),
  deleteRegion: (id: number) => request("DELETE", `/regions/${id}`, Empty),
  jobs: (siteId: number) => request("GET", `/sites/${siteId}/jobs`, z.array(Job)),
  job: (id: number) => request("GET", `/jobs/${id}`, Job),
  createJob: (siteId: number, kind: string, params: Record<string, unknown>) =>
    request("POST", `/sites/${siteId}/jobs`, Job, { kind, params }),
  deleteJob: (id: number) => request("DELETE", `/jobs/${id}`, Empty),
  odmStatus: () => request("GET", "/odm", OdmStatus),
  settings: () => request("GET", "/settings", Settings),
  flightPlan: (regionId: number, query: string) =>
    request("GET", `/regions/${regionId}/flightplan?${query}`, FlightPlan),
  dashboard: (siteId: number) => request("GET", `/sites/${siteId}/dashboard`, Dashboard),
  markers: (siteId: number) => request("GET", `/sites/${siteId}/markers`, z.array(Marker)),
  createMarker: (siteId: number, m: NewMarker) =>
    request("POST", `/sites/${siteId}/markers`, Marker, m),
  deleteMarker: (id: number) => request("DELETE", `/markers/${id}`, Empty),
  plans: (siteId: number) => request("GET", `/sites/${siteId}/plans`, z.array(Plan)),
  planGeometry: (id: number) => request("GET", `/plans/${id}/geometry`, PlanGeometry),
  deletePlan: (id: number) => request("DELETE", `/plans/${id}`, Empty),
  georefFit: (mode: string, points: unknown[]) =>
    request("POST", "/georef/fit", FitReport, { mode, points }),
  createOdm: (siteId: number, params: OdmParams) =>
    request("POST", `/sites/${siteId}/odm`, Job, params),
  startOdm: (jobId: number) => request("POST", `/jobs/${jobId}/start`, Job, {}),
};

/** Datei als Body per PUT, mit Fortschritt (fetch kennt keinen Upload-Fortschritt) */
const putFile = <T>(
  url: string,
  file: File,
  schema: z.ZodType<T>,
  onProgress: (fraction: number) => void,
): Promise<T> =>
  new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("PUT", url);
    for (const [k, v] of Object.entries(requestHeaders("PUT", false))) xhr.setRequestHeader(k, v);
    xhr.setRequestHeader("content-type", "application/octet-stream");
    xhr.upload.onprogress = (e) => e.lengthComputable && onProgress(e.loaded / e.total);
    xhr.onload = () => {
      const res = new Response(xhr.responseText, { status: xhr.status });
      if (xhr.status >= 200 && xhr.status < 300)
        res.json().then((j) => resolve(schema.parse(j)), reject);
      else errorOf(res).then(reject, reject);
    };
    xhr.onerror = () => reject(new ApiError(0, "http.0"));
    xhr.send(file);
  });

/** Foto zur Dokumentation hochladen; Lage aus GPS, falls vorhanden */
export const uploadPhoto = (
  siteId: number,
  file: File,
  onProgress: (fraction: number) => void,
): Promise<Marker> =>
  putFile(
    `/api/sites/${siteId}/photos?${new URLSearchParams({ file: file.name })}`,
    file,
    Marker,
    onProgress,
  );

/** DXF-Plan hochladen */
export const uploadPlan = (siteId: number, file: File): Promise<Plan> =>
  putFile(
    `/api/sites/${siteId}/plans?${new URLSearchParams({ file: file.name })}`,
    file,
    Plan,
    () => {},
  );

/** Aufnahme fortsetzbar hochladen (tus) */
export const uploadCapture = (
  siteId: number,
  file: File,
  name: string,
  onProgress: (fraction: number) => void,
  capturedAt?: number,
): Promise<void> =>
  resumableUpload(file, { target: "capture", siteId, name, capturedAt }, onProgress);

/** Foto zu einem Photogrammetrie-Entwurf fortsetzbar hochladen */
export const uploadImage = (
  jobId: number,
  file: File,
  onProgress: (fraction: number) => void,
): Promise<void> => resumableUpload(file, { target: "image", jobId }, onProgress);

/** Orthofoto (GeoTIFF) an eine Aufnahme hängen, fortsetzbar */
export const uploadOrtho = (
  captureId: number,
  file: File,
  onProgress: (fraction: number) => void,
): Promise<void> => resumableUpload(file, { target: "ortho", captureId }, onProgress);
