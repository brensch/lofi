#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";

const BLOCK_FRAMES = 128;
const SAMPLE_RATE = 48_000;
const iterations = Number(process.argv[2] ?? 20_000);
const seed = Number(process.argv[3] ?? 11);
const startPhrase = Number(process.argv[4] ?? 14);
const warmupIterations = 500;

if (!Number.isInteger(iterations) || iterations < 1 || !Number.isInteger(seed) || !Number.isInteger(startPhrase)) {
  throw new Error("iterations, seed, and start phrase must be integers");
}

globalThis.sampleRate = SAMPLE_RATE;
let Processor;
globalThis.AudioWorkletProcessor = class {
  constructor() {
    this.port = { close() {}, onmessage: null, postMessage() {} };
  }
};
globalThis.registerProcessor = (_name, constructor) => {
  Processor = constructor;
};

const workletPath = path.resolve("apps/mesh-lab/src/audio/mesh-worklet.js");
await import(pathToFileURL(workletPath).href);
const processor = new Processor({
  processorOptions: {
    bpmMilli: 80_000,
    initialNodes: 3,
    seed,
    startPhrase,
    telemetry: false,
    wasmBytes: fs.readFileSync("apps/mesh-lab/public/lofi_web.wasm"),
  },
});
if (processor.failed) throw new Error("worklet initialization failed");

const left = new Float32Array(BLOCK_FRAMES);
const right = new Float32Array(BLOCK_FRAMES);
const outputs = [[left, right]];
for (let index = 0; index < warmupIterations; index += 1) {
  processor.process([], outputs);
}

globalThis.gc?.();
const heapBefore = process.memoryUsage().heapUsed;
const timings = new Float64Array(iterations);
for (let index = 0; index < iterations; index += 1) {
  const startedAt = performance.now();
  processor.process([], outputs);
  timings[index] = performance.now() - startedAt;
}
globalThis.gc?.();
const memory = process.memoryUsage();

timings.sort();
const percentile = (fraction) => timings[Math.min(iterations - 1, Math.floor(iterations * fraction))];
const totalMs = timings.reduce((sum, value) => sum + value, 0);
const budgetMs = (BLOCK_FRAMES / SAMPLE_RATE) * 1_000;
const result = {
  iterations,
  seed,
  startPhrase,
  simulatedSeconds: (iterations * BLOCK_FRAMES) / SAMPLE_RATE,
  budgetMs,
  timingMs: {
    mean: totalMs / iterations,
    p95: percentile(0.95),
    p99: percentile(0.99),
    p999: percentile(0.999),
    max: timings[iterations - 1],
  },
  heapGrowthBytes: memory.heapUsed - heapBefore,
  residentBytes: memory.rss,
};

processor.handleCommand({ type: "dispose" });
if (processor.process([], outputs) !== false) {
  throw new Error("disposed AudioWorklet processor remained active");
}
process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
