import { describe, expect, it, vi } from "vitest";
import type { Job } from "@/lib/api";
import { parseOffset, photosOnly, uploadFlight } from "@/lib/odm";

const job = (status: string): Job => ({
  id: 7,
  siteId: 1,
  kind: "odm",
  params: {},
  status,
  result: null,
  error: null,
  createdAt: 0,
  startedAt: null,
  finishedAt: null,
});

const file = (name: string, size: number) => new File([new Uint8Array(size)], name);

describe("odm", () => {
  it("legt an, lädt nacheinander hoch und startet; Fortschritt nach Bytes", async () => {
    const calls: string[] = [];
    const progress: number[] = [];
    const started = await uploadFlight(
      {
        create: async () => {
          calls.push("create");
          return job("draft");
        },
        upload: async (id, f, onProgress) => {
          calls.push(`upload ${id} ${f.name}`);
          onProgress(0.5);
        },
        start: async (id) => {
          calls.push(`start ${id}`);
          return job("queued");
        },
      },
      { name: "Flug", capturedAt: 0, quality: "standard", heightOffset: 0 },
      [file("a.jpg", 100), file("b.jpg", 300)],
      (p) => progress.push(p.fraction),
    );
    expect(calls).toEqual(["create", "upload 7 a.jpg", "upload 7 b.jpg", "start 7"]);
    expect(started.status).toBe("queued");
    // halbes a (50/400), a fertig (100/400), halbes b (250/400), alles (400/400)
    expect(progress).toEqual([0.125, 0.25, 0.625, 1]);
  });

  it("startet nicht, wenn ein Upload scheitert", async () => {
    const start = vi.fn();
    await expect(
      uploadFlight(
        {
          create: async () => job("draft"),
          upload: async () => {
            throw new Error("weg");
          },
          start,
        },
        { name: "Flug", capturedAt: 0, quality: "fast", heightOffset: 0 },
        [file("a.jpg", 1)],
        () => {},
      ),
    ).rejects.toThrow("weg");
    expect(start).not.toHaveBeenCalled();
  });

  it("filtert Fotos und liest den Höhenversatz", () => {
    expect(
      photosOnly([file("DJI_0001.JPG", 1), file("x.laz", 1), file("b.tif", 1)]).map((f) => f.name),
    ).toEqual(["DJI_0001.JPG", "b.tif"]);
    expect(parseOffset("47,25")).toBe(47.25);
    expect(parseOffset(" ")).toBe(0);
    expect(parseOffset("-1.5")).toBe(-1.5);
    expect(parseOffset("abc")).toBeNull();
    expect(parseOffset("500")).toBeNull();
  });
});
