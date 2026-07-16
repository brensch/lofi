export const MAX_NODES = 8;

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
  synced: boolean;
  meshOffsetUs: number;
  beatPhase: number;
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
  sampleRate: number;
  nodes: MeshNode[];
  network: NetworkState;
}

export interface WorkletError {
  type: "error";
  message: string;
}

export type WorkletMessage = MeshTelemetry | WorkletError;

export type WorkletCommand =
  | { type: "addNode" }
  | { type: "removeNode"; id: number }
  | { type: "node"; id: number; key: NodeControlKey; value: number | boolean }
  | { type: "network"; key: NetworkControlKey; value: number | boolean }
  | { type: "seed"; value: number };
