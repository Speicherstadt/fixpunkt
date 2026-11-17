import { describe, expect, it } from "vitest";
import { signedM3 } from "./overview-stock";

describe("Bestand", () => {
  it("zeigt Änderungen mit Vorzeichen, auf m³ gerundet", () => {
    expect(signedM3(40.2)).toBe("+40 m³");
    expect(signedM3(-450)).toBe("−450 m³");
    expect(signedM3(-1234.6)).toBe("−1.235 m³");
    expect(signedM3(0.3)).toBe("±0 m³");
  });
});
