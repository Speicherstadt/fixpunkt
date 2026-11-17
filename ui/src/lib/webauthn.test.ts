import { describe, expect, it } from "vitest";
import { creationOptions, fromB64url, isCancelled, requestOptions, toB64url } from "./webauthn";

describe("WebAuthn", () => {
  it("base64url hin und zurück, ohne Auffüllzeichen", () => {
    // RFC 4648 Testvektoren: "f" → "Zg", "foob" → "Zm9vYg"
    expect(toB64url(new TextEncoder().encode("f"))).toBe("Zg");
    expect(toB64url(new TextEncoder().encode("foob"))).toBe("Zm9vYg");
    // Bytes 0xfb 0xff ergeben in base64 „+/8=“, in base64url „-_8“
    expect(toB64url(new Uint8Array([0xfb, 0xff]))).toBe("-_8");
    expect([...fromB64url("-_8")]).toEqual([0xfb, 0xff]);
    const random = crypto.getRandomValues(new Uint8Array(37));
    expect([...fromB64url(toB64url(random))]).toEqual([...random]);
  });

  it("wandelt die Binärfelder der Optionen", () => {
    const create = creationOptions({
      publicKey: {
        rp: { id: "localhost", name: "Fixpunkt" },
        user: { id: "AQID", name: "admin", displayName: "admin" },
        challenge: "BAUG",
        pubKeyCredParams: [{ type: "public-key", alg: -7 }],
        excludeCredentials: [{ id: "CQ", type: "public-key" }],
      },
    }).publicKey!;
    expect([...(create.user.id as Uint8Array)]).toEqual([1, 2, 3]);
    expect([...(create.challenge as Uint8Array)]).toEqual([4, 5, 6]);
    expect([...(create.excludeCredentials![0]!.id as Uint8Array)]).toEqual([9]);
    expect(create.rp.id).toBe("localhost");

    const get = requestOptions({
      publicKey: { challenge: "BAUG", allowCredentials: [{ id: "CQ", type: "public-key" }] },
    }).publicKey!;
    expect([...(get.allowCredentials![0]!.id as Uint8Array)]).toEqual([9]);
  });

  it("erkennt Abbruch", () => {
    expect(isCancelled(new DOMException("x", "NotAllowedError"))).toBe(true);
    expect(isCancelled(new Error("x"))).toBe(false);
  });
});
