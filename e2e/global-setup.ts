import { mkdirSync } from "node:fs";

/** Ordner für die Sitzung des Admins (setup.spec.ts schreibt sie) */
export default function globalSetup() {
  mkdirSync("e2e/.auth", { recursive: true });
}
