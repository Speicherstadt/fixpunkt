/**
 * NodeODM-Nachbau für die E2E-Tests (`bun e2e/nodeodm-stub.ts <port>`): dieselbe REST-API wie NodeODM,
 * aber ohne Photogrammetrie. Jede Aufgabe läuft zwei Abfragen lang und liefert dann das Testgelände nach
 * dem Aushub als Punktwolke (LAZ, erzeugt mit `fixpunkt ground` aus der echten Programmdatei).
 */
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { terrainXyz } from "./fixtures";

const port = Number(process.argv[2] ?? 28081);
const dir = mkdtempSync(join(tmpdir(), "nodeodm-"));
writeFileSync(join(dir, "gelaende.xyz"), terrainXyz(true));
const made = spawnSync("./target/debug/fixpunkt", [
  "ground",
  join(dir, "gelaende.xyz"),
  join(dir, "model.laz"),
]);
if (made.status !== 0) throw new Error(String(made.stderr));
const model = readFileSync(join(dir, "model.laz"));

type Task = { name: string; images: number; polls: number };
const tasks = new Map<string, Task>();
let next = 1;

const json = (v: unknown) => Response.json(v);

const handle = async (req: Request): Promise<Response> => {
  const url = new URL(req.url);
  const parts = url.pathname.split("/").filter(Boolean);
  if (url.pathname === "/info") {
    return json({ version: "2.5.2", taskQueueCount: tasks.size, engineVersion: "3.5.6" });
  }
  if (url.pathname === "/task/new/init") {
    const form = await req.formData();
    const uuid = `aufgabe-${next++}`;
    tasks.set(uuid, { name: String(form.get("name")), images: 0, polls: 0 });
    return json({ uuid });
  }
  if (parts[0] === "task" && parts[1] === "new" && parts[2] === "upload") {
    const t = tasks.get(parts[3] ?? "");
    if (!t) return json({ error: "unbekannte Aufgabe" });
    t.images += (await req.formData()).getAll("images").length;
    return json({ success: true });
  }
  if (parts[0] === "task" && parts[1] === "new" && parts[2] === "commit") {
    return json({ uuid: parts[3] });
  }
  if (url.pathname === "/task/remove") return json({ success: true });
  const byId = tasks.get(parts[1] ?? "");
  if (parts[0] === "task" && byId && parts[2] === "info") {
    byId.polls++;
    return json(
      byId.polls < 2
        ? { status: { code: 20 }, progress: 60, imagesCount: byId.images }
        : { status: { code: 40 }, progress: 100, processingTime: 754_000 },
    );
  }
  if (parts[0] === "task" && byId && parts[2] === "download") {
    return new Response(model);
  }
  if (parts[0] === "task" && byId && parts[2] === "output") return json([]);
  return new Response("nicht gefunden", { status: 404 });
};

/** node:http → Request/Response wie fetch */
createServer(async (incoming, outgoing) => {
  const chunks: Buffer[] = [];
  for await (const c of incoming) chunks.push(c as Buffer);
  const body = incoming.method === "GET" ? undefined : Buffer.concat(chunks);
  const headers = new Headers();
  for (const [k, v] of Object.entries(incoming.headers))
    if (typeof v === "string") headers.set(k, v);
  const res = await handle(
    new Request(`http://127.0.0.1:${port}${incoming.url}`, {
      method: incoming.method,
      headers,
      body,
    }),
  );
  outgoing.writeHead(res.status, Object.fromEntries(res.headers));
  outgoing.end(Buffer.from(await res.arrayBuffer()));
}).listen(port, "127.0.0.1");
console.log(`NodeODM-Nachbau auf ${port}`);
