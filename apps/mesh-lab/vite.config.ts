import { execFileSync } from "node:child_process";
import { appendFile, mkdir, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const repositoryRoot = fileURLToPath(new URL("../..", import.meta.url));
const judgementFile = fileURLToPath(
  new URL("../../target/listening-study/judgements.jsonl", import.meta.url),
);
const judgementIds = new Set<string>();
let judgementIdsLoaded: Promise<void> | undefined;

export default defineConfig({
  define: {
    __BUILD_REVISION__: JSON.stringify(buildRevision()),
  },
  plugins: [react(), judgementApi()],
  server: {
    allowedHosts: ["schbox.tail3752a2.ts.net"],
    host: "0.0.0.0",
    port: 5173,
    strictPort: true,
  },
  preview: {
    host: "0.0.0.0",
    port: 4173,
    strictPort: true,
  },
  build: {
    target: "es2022",
  },
});

function buildRevision() {
  try {
    const revision = execFileSync("git", ["rev-parse", "--short", "HEAD"], {
      cwd: repositoryRoot,
      encoding: "utf8",
    }).trim();
    const dirty = execFileSync("git", ["status", "--porcelain"], {
      cwd: repositoryRoot,
      encoding: "utf8",
    }).trim();
    return `${revision}${dirty ? "+dirty" : ""}`;
  } catch {
    return "unknown";
  }
}

function judgementApi() {
  const middleware = async (
    request: import("node:http").IncomingMessage,
    response: import("node:http").ServerResponse,
    next: () => void,
  ) => {
    if (request.url !== "/api/judgements") {
      next();
      return;
    }
    if (request.method !== "POST") {
      response.writeHead(405, { Allow: "POST" }).end();
      return;
    }

    try {
      const body = await readJsonBody(request);
      if (!isJudgement(body)) {
        response.writeHead(400, { "Content-Type": "application/json" });
        response.end(JSON.stringify({ error: "Invalid judgement record" }));
        return;
      }
      await loadJudgementIds();
      if (judgementIds.has(body.id as string)) {
        response.writeHead(200, { "Content-Type": "application/json" });
        response.end(JSON.stringify({ duplicate: true, ok: true }));
        return;
      }
      judgementIds.add(body.id as string);
      await mkdir(fileURLToPath(new URL("../../target/listening-study", import.meta.url)), {
        recursive: true,
      });
      const stored = { ...body, receivedAt: new Date().toISOString() };
      try {
        await appendFile(judgementFile, `${JSON.stringify(stored)}\n`, "utf8");
      } catch (error) {
        judgementIds.delete(body.id as string);
        throw error;
      }
      response.writeHead(201, { "Content-Type": "application/json" });
      response.end(JSON.stringify({ ok: true }));
    } catch (error) {
      const message = error instanceof Error ? error.message : "";
      const status = message === "Body too large" ? 413 : 500;
      response.writeHead(status, { "Content-Type": "application/json" });
      response.end(JSON.stringify({ error: status === 413 ? message : "Could not store judgement" }));
    }
  };

  return {
    name: "lofi-judgement-api",
    configureServer(server: import("vite").ViteDevServer) {
      server.middlewares.use(middleware);
    },
    configurePreviewServer(server: import("vite").PreviewServer) {
      server.middlewares.use(middleware);
    },
  };
}

function loadJudgementIds() {
  judgementIdsLoaded ??= readFile(judgementFile, "utf8")
    .then((content) => {
      for (const line of content.split("\n")) {
        if (!line) continue;
        try {
          const record = JSON.parse(line) as { id?: unknown };
          if (typeof record.id === "string") judgementIds.add(record.id);
        } catch {
          // Preserve valid records if a manually edited line is malformed.
        }
      }
    })
    .catch((error: NodeJS.ErrnoException) => {
      if (error.code !== "ENOENT") throw error;
    });
  return judgementIdsLoaded;
}

function readJsonBody(request: import("node:http").IncomingMessage): Promise<unknown> {
  return new Promise((resolve, reject) => {
    let body = "";
    request.setEncoding("utf8");
    request.on("data", (chunk: string) => {
      body += chunk;
      if (body.length > 64 * 1024) reject(new Error("Body too large"));
    });
    request.on("end", () => {
      try {
        resolve(JSON.parse(body));
      } catch (error) {
        reject(error);
      }
    });
    request.on("error", reject);
  });
}

function isJudgement(value: unknown): value is Record<string, unknown> {
  if (!value || typeof value !== "object") return false;
  const record = value as Record<string, unknown>;
  const candidate = record.candidate as Record<string, unknown> | undefined;
  return record.schemaVersion === 1
    && typeof record.id === "string" && record.id.length <= 80
    && typeof record.sessionId === "string" && record.sessionId.length <= 80
    && (record.verdict === "like" || record.verdict === "dislike")
    && Array.isArray(record.tags) && record.tags.length <= 12
    && record.tags.every((tag) => typeof tag === "string" && tag.length <= 40)
    && typeof record.note === "string" && record.note.length <= 1_000
    && typeof record.submittedAt === "string"
    && Number.isFinite(Date.parse(record.submittedAt))
    && Number.isInteger(record.listenedMs) && Number(record.listenedMs) >= 24_000
    && Number.isInteger(record.replayCount) && Number(record.replayCount) >= 0
    && Number.isInteger(record.decisionMs) && Number(record.decisionMs) >= 0
    && !!candidate
    && Number.isInteger(candidate.seed) && Number(candidate.seed) >= 0
    && Number(candidate.seed) <= 0xffff_ffff
    && Number.isInteger(candidate.sequence) && Number(candidate.sequence) >= 1
    && candidate.bars === 8
    && Number.isInteger(candidate.bpm) && Number(candidate.bpm) >= 60 && Number(candidate.bpm) <= 100
    && Number.isInteger(candidate.durationMs)
    && Number(candidate.durationMs) === Math.round((8 * 4 * 60_000) / Number(candidate.bpm))
    && candidate.nodeCount === 3
    && (candidate.engine === undefined || candidate.engine === "loops" || candidate.engine === "symbolic")
    && (candidate.profileId === undefined || (typeof candidate.profileId === "string" && candidate.profileId.length <= 40))
    && (candidate.sourceSlot === undefined || (Number.isInteger(candidate.sourceSlot) && Number(candidate.sourceSlot) >= 0 && Number(candidate.sourceSlot) <= 8))
    && (candidate.startPhrase === undefined || (Number.isInteger(candidate.startPhrase) && Number(candidate.startPhrase) >= 0 && Number(candidate.startPhrase) <= 10_000))
    && Number.isInteger(candidate.sampleRate) && Number(candidate.sampleRate) > 0
    && typeof candidate.buildRevision === "string" && candidate.buildRevision.length <= 80;
}
