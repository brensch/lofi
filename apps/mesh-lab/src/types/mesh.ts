export const MAX_NODES = 10;

export type NodeControlKey =
  | "driftPpm"
  | "gain"
  | "mute"
  | "pan"
  | "radio"
  | "solo";

export type NetworkControlKey = "enabled" | "jitterMs" | "latencyMs" | "lossPercent";

export interface MeshNode {
  id: number;
  rootId: number;
  peers: number;
  dispersionUs: number;
  role: number;
  roleMask: number;
  spotlight: number;
  phrase: number;
  selectorId: number;
  synced: boolean;
  meshOffsetUs: number;
  beatPhase: number;
  beatsToChangeMilli: number;
  upcomingChange: number;
  isRoot: boolean;
  driftPpm: number;
  offsetUs: number;
  pan: number;
  gain: number;
  mute: boolean;
  solo: boolean;
  radio: boolean;
}

export interface NetworkState {
  enabled: boolean;
  lossPercent: number;
  latencyMs: number;
  jitterMs: number;
  queued: number;
  sent: number;
  delivered: number;
  dropped: number;
}

export interface MeshTelemetry {
  type: "ready" | "telemetry";
  version: string;
  sampleRate: number;
  nodes: MeshNode[];
  network: NetworkState;
}

export interface AudioDiagnostics {
  boundaryJumpMax: number;
  clippedSamples: number;
  currentFrame: number | null;
  globalFrame: number;
  largeBoundaryJumps: number;
  lateCallbacks: number;
  maxCallbackIntervalMs: number;
  maxProcessMs: number;
  maxQuantumGapFrames: number;
  meanSampledProcessMs: number | null;
  outputPeak: number;
  processCalls: number;
  quantumBudgetMs: number;
  quantumGaps: number;
  renderedFrames: number;
  sampledProcessCalls: number;
  timingSource: "date" | "performance";
}

export interface WorkletDiagnostics extends Omit<MeshTelemetry, "type"> {
  type: "diagnostics";
  requestId: number;
  audio: AudioDiagnostics;
}

export interface WorkletError {
  type: "error";
  message: string;
}

export type WorkletMessage = MeshTelemetry | WorkletDiagnostics | WorkletError;

export type WorkletCommand =
  | { type: "dispose" }
  | { type: "diagnostics"; requestId: number }
  | { type: "addNode" }
  | { type: "removeNode"; id: number }
  | { type: "node"; id: number; key: NodeControlKey; value: number | boolean }
  | { type: "network"; key: NetworkControlKey; value: number | boolean };
