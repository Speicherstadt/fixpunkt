import { describe, expect, it } from "vitest";
import { ApiError } from "@/lib/api";
import { errorText } from "@/lib/errors";

describe("errorText", () => {
  it("übersetzt Schlüssel des Servers", () => {
    expect(errorText(new ApiError(401, "auth.wrongCredentials"))).toBe(
      "Name oder Passwort stimmt nicht.",
    );
    expect(errorText(new ApiError(409, "jobs.captureNotReady"))).toMatch(/noch geprüft/);
  });

  it("unbekannt und offline", () => {
    expect(errorText(new ApiError(500, "gibt.esNicht"))).toMatch(/schiefgegangen/);
    expect(errorText(new ApiError(0, "http.0"))).toMatch(/nicht verloren/);
    expect(errorText(new Error("x"))).toMatch(/schiefgegangen/);
  });
});
