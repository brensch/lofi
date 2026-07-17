#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const FRAME_COUNT = 128;

function parseArgs(argv) {
  const args = {
    bpm: 80,
    duration: 45,
    nodes: 3,
    output: "target/listen-qa/browser-mix.wav",
    sampleRate: 48_000,
    seed: 2,
    solo: 0,
    startPhrase: 0,
    wasm: "apps/mesh-lab/public/lofi_web.wasm",
    worklet: "apps/mesh-lab/src/audio/mesh-worklet.js",
  };
  for (let index = 0; index < argv.length; index += 2) {
    const name = argv[index];
    const value = argv[index + 1];
    if (value === undefined) throw new Error(`${name} requires a value`);
    if (name === "--bpm") args.bpm = Number(value);
    else if (name === "--duration") args.duration = Number(value);
    else if (name === "--nodes") args.nodes = Number(value);
    else if (name === "--output") args.output = value;
    else if (name === "--sample-rate") args.sampleRate = Number(value);
    else if (name === "--seed") args.seed = Number(value);
    else if (name === "--solo") args.solo = Number(value);
    else if (name === "--start-phrase") args.startPhrase = Number(value);
    else if (name === "--wasm") args.wasm = value;
    else if (name === "--worklet") args.worklet = value;
    else throw new Error(`unknown argument ${name}`);
  }
  if (!(args.bpm > 0) || !(args.duration > 0) || !(args.sampleRate > 0)) {
    throw new Error("invalid tempo, duration, or rate");
  }
  if (!Number.isInteger(args.startPhrase) || args.startPhrase < 0) {
    throw new Error("start phrase must be a non-negative integer");
  }
  if (!Number.isInteger(args.nodes) || args.nodes < 1 || args.nodes > 10) {
    throw new Error("nodes must be an integer from 1 through 10");
  }
  return args;
}

function writeWav(filename, left, right, sampleRate) {
  const frames = left.length;
  const bytes = Buffer.allocUnsafe(44 + frames * 4);
  bytes.write("RIFF", 0);
  bytes.writeUInt32LE(36 + frames * 4, 4);
  bytes.write("WAVEfmt ", 8);
  bytes.writeUInt32LE(16, 16);
  bytes.writeUInt16LE(1, 20);
  bytes.writeUInt16LE(2, 22);
  bytes.writeUInt32LE(sampleRate, 24);
  bytes.writeUInt32LE(sampleRate * 4, 28);
  bytes.writeUInt16LE(4, 32);
  bytes.writeUInt16LE(16, 34);
  bytes.write("data", 36);
  bytes.writeUInt32LE(frames * 4, 40);
  for (let frame = 0; frame < frames; frame += 1) {
    const leftPcm = Math.round(Math.max(-1, Math.min(1, left[frame])) * 32_767);
    const rightPcm = Math.round(Math.max(-1, Math.min(1, right[frame])) * 32_767);
    bytes.writeInt16LE(leftPcm, 44 + frame * 4);
    bytes.writeInt16LE(rightPcm, 46 + frame * 4);
  }
  fs.mkdirSync(path.dirname(filename), { recursive: true });
  fs.writeFileSync(filename, bytes);
}

const args = parseArgs(process.argv.slice(2));
globalThis.sampleRate = args.sampleRate;
let Processor;
globalThis.AudioWorkletProcessor = class {
  constructor() {
    this.port = { onmessage: null, postMessage() {} };
  }
};
globalThis.registerProcessor = (_name, constructor) => {
  Processor = constructor;
};
await import(pathToFileURL(path.resolve(args.worklet)).href);
if (!Processor) throw new Error("worklet did not register a processor");

const wasmBytes = fs.readFileSync(args.wasm);
const processor = new Processor({
  processorOptions: {
    bpmMilli: Math.round(args.bpm * 1_000),
    initialNodes: args.nodes,
    seed: args.seed,
    startPhrase: args.startPhrase,
    wasmBytes,
  },
});
if (processor.failed) throw new Error("worklet initialization failed");
if (args.solo) {
  if (!Number.isInteger(args.solo) || args.solo < 1 || args.solo > args.nodes) {
    throw new Error("solo must identify an active node");
  }
  processor.handleCommand({ type: "node", id: args.solo, key: "solo", value: true });
}

const totalFrames = Math.round(args.duration * args.sampleRate);
const left = new Float32Array(totalFrames);
const right = new Float32Array(totalFrames);
for (let offset = 0; offset < totalFrames; offset += FRAME_COUNT) {
  const blockLeft = new Float32Array(FRAME_COUNT);
  const blockRight = new Float32Array(FRAME_COUNT);
  processor.process([], [[blockLeft, blockRight]]);
  const length = Math.min(FRAME_COUNT, totalFrames - offset);
  left.set(blockLeft.subarray(0, length), offset);
  right.set(blockRight.subarray(0, length), offset);
}

writeWav(args.output, left, right, args.sampleRate);
processor.handleCommand({ type: "dispose" });
const stopped = processor.process(
  [],
  [[new Float32Array(FRAME_COUNT), new Float32Array(FRAME_COUNT)]],
);
if (stopped !== false) throw new Error("disposed AudioWorklet processor remained active");
process.stdout.write(
  `${args.output}: ${args.duration.toFixed(1)}s, ${args.nodes} nodes, seed ${args.seed}, ` +
    `${args.bpm} BPM, phrase ${args.startPhrase}` +
    `${args.solo ? `, solo node ${args.solo}` : ""}\n`,
);
