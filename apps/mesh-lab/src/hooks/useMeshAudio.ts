import { useCallback, useEffect, useRef, useState } from "react";

import workletUrl from "../audio/mesh-worklet.js?url";
import type {
  MeshTelemetry,
  NetworkControlKey,
  NodeControlKey,
  WorkletCommand,
  WorkletMessage,
} from "../types/mesh";

export type RuntimeState = "error" | "offline" | "paused" | "running" | "starting";

const EMPTY_TELEMETRY: MeshTelemetry = {
  type: "telemetry",
  sampleRate: 0,
  nodes: [],
  network: {
    enabled: true,
    lossPercent: 0,
    latencyMs: 2,
    jitterMs: 0.5,
    queued: 0,
    sent: 0,
    delivered: 0,
    dropped: 0,
  },
};

export function useMeshAudio(initialSeed: number, initialVolume: number) {
  const contextRef = useRef<AudioContext | null>(null);
  const workletRef = useRef<AudioWorkletNode | null>(null);
  const gainRef = useRef<GainNode | null>(null);
  const readyRef = useRef<{ resolve: () => void; reject: (error: Error) => void } | null>(null);
  const [runtime, setRuntime] = useState<RuntimeState>("offline");
  const [error, setError] = useState("");
  const [telemetry, setTelemetry] = useState<MeshTelemetry>(EMPTY_TELEMETRY);
  const [analyser, setAnalyser] = useState<AnalyserNode>();

  const handleMessage = useCallback((event: MessageEvent<WorkletMessage>) => {
    const message = event.data;
    if (message.type === "error") {
      const nextError = new Error(message.message);
      readyRef.current?.reject(nextError);
      readyRef.current = null;
      setError(message.message);
      setRuntime("error");
      return;
    }
    setTelemetry(message);
    if (message.type === "ready") {
      readyRef.current?.resolve();
      readyRef.current = null;
    }
  }, []);

  const createGraph = useCallback(async () => {
    const context = new AudioContext({ latencyHint: "playback" });
    contextRef.current = context;
    let timeout = 0;
    const ready = new Promise<void>((resolve, reject) => {
      timeout = window.setTimeout(() => reject(new Error("AudioWorklet startup timed out")), 5_000);
      readyRef.current = {
        resolve: () => { window.clearTimeout(timeout); resolve(); },
        reject: (error) => { window.clearTimeout(timeout); reject(error); },
      };
    });
    const [wasmResponse] = await Promise.all([
      fetch("/lofi_web.wasm"),
      context.audioWorklet.addModule(workletUrl),
    ]);
    if (!wasmResponse.ok) throw new Error(`WASM request failed: ${wasmResponse.status}`);

    const worklet = new AudioWorkletNode(context, "lofi-processor", {
      numberOfInputs: 0,
      numberOfOutputs: 1,
      outputChannelCount: [2],
      processorOptions: {
        wasmBytes: await wasmResponse.arrayBuffer(),
        seed: initialSeed,
        initialNodes: 3,
      },
    });
    const gain = new GainNode(context, { gain: initialVolume });
    const nextAnalyser = new AnalyserNode(context, {
      fftSize: 2048,
      smoothingTimeConstant: 0.72,
    });
    worklet.connect(gain).connect(nextAnalyser).connect(context.destination);
    worklet.port.onmessage = handleMessage;
    workletRef.current = worklet;
    gainRef.current = gain;
    setAnalyser(nextAnalyser);
    await ready;
  }, [handleMessage, initialSeed, initialVolume]);

  const toggle = useCallback(async () => {
    setError("");
    try {
      let created = false;
      if (!contextRef.current) {
        setRuntime("starting");
        await createGraph();
        created = true;
      }
      const context = contextRef.current!;
      if (!created && context.state === "running") {
        await context.suspend();
        setRuntime("paused");
      } else {
        await context.resume();
        setRuntime("running");
      }
    } catch (cause) {
      workletRef.current?.disconnect();
      await contextRef.current?.close();
      workletRef.current = null;
      gainRef.current = null;
      contextRef.current = null;
      readyRef.current = null;
      setAnalyser(undefined);
      const message = cause instanceof Error ? cause.message : String(cause);
      setError(message);
      setRuntime("error");
    }
  }, [createGraph]);

  const send = useCallback((command: WorkletCommand) => {
    workletRef.current?.port.postMessage(command);
  }, []);

  const addNode = useCallback(() => send({ type: "addNode" }), [send]);
  const removeNode = useCallback((id: number) => send({ type: "removeNode", id }), [send]);
  const updateNode = useCallback(
    (id: number, key: NodeControlKey, value: number | boolean) =>
      send({ type: "node", id, key, value }),
    [send],
  );
  const updateNetwork = useCallback(
    (key: NetworkControlKey, value: number | boolean) => send({ type: "network", key, value }),
    [send],
  );

  const setVolume = useCallback((value: number) => {
    const context = contextRef.current;
    const gain = gainRef.current;
    if (context && gain) gain.gain.setTargetAtTime(value, context.currentTime, 0.015);
  }, []);

  const setSeed = useCallback(
    (value: number, restoreVolume: number) => {
      const context = contextRef.current;
      const gain = gainRef.current;
      if (!context || !gain) return;
      gain.gain.setTargetAtTime(0, context.currentTime, 0.012);
      window.setTimeout(() => {
        send({ type: "seed", value });
        gain.gain.setTargetAtTime(restoreVolume, context.currentTime, 0.025);
      }, 80);
    },
    [send],
  );

  useEffect(
    () => () => {
      workletRef.current?.disconnect();
      void contextRef.current?.close();
    },
    [],
  );

  return {
    addNode,
    analyser,
    error,
    removeNode,
    runtime,
    setSeed,
    setVolume,
    telemetry,
    toggle,
    updateNetwork,
    updateNode,
  };
}
