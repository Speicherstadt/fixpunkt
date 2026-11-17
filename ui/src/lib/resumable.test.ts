import { describe, expect, it } from "vitest";
import { fingerprintOf, metadataOf } from "@/lib/resumable";

describe("resumable", () => {
  it("Metadaten wie uploads.rs sie erwartet", () => {
    expect(metadataOf("flug.laz", { target: "capture", siteId: 7, name: "Aushub Süd" })).toEqual({
      filename: "flug.laz",
      target: "capture",
      siteId: "7",
      name: "Aushub Süd",
    });
    expect(metadataOf("a.jpg", { target: "image", jobId: 3 })).toEqual({
      filename: "a.jpg",
      target: "image",
      jobId: "3",
    });
  });

  it("dieselbe Datei für ein anderes Ziel setzt keinen alten Upload fort", () => {
    const f = { name: "flug.laz", size: 10, lastModified: 1 };
    const a = fingerprintOf(f, { target: "capture", siteId: 1, name: "x" });
    const b = fingerprintOf(f, { target: "capture", siteId: 2, name: "x" });
    expect(a).not.toBe(b);
    expect(fingerprintOf(f, { target: "capture", siteId: 1, name: "x" })).toBe(a);
  });
});
