import { useCallback, useEffect, useRef, useState } from "react";

import workletUrl from "../audio/mesh-worklet.js?url";
import type { CandidateIdentity } from "../types/judgement";
import type { WorkletDiagnostics, WorkletMessage } from "../types/mesh";
import { APP_VERSION, versionedAssetUrl } from "../version";

export type JudgementRuntime = "complete" | "error" | "idle" | "loading" | "paused" | "playing";

const PROGRESS_REFRESH_MS = 100;
const WORKLET_URL = versionedAssetUrl(workletUrl);
const WASM_URL = versionedAssetUrl("/lofi_web.wasm");
const DEBUG_TIMEOUT_MS = 750;

interface DiagnosticWaiter {
  requestId: number;
  resolve: (diagnostics: WorkletDiagnostics | undefined) => void;
  timeout: number;
}

export function useJudgementAudio(initialVolume: number) {
  const contextRef = useRef<AudioContext | undefined>(undefined);
  const wasmBytesRef = useRef<ArrayBuffer | undefined>(undefined);
  const masterGainRef = useRef<GainNode | undefined>(undefined);
  const candidateGainRef = useRef<GainNode | undefined>(undefined);
  const workletRef = useRef<AudioWorkletNode | undefined>(undefined);
  const analyserRef = useRef<AnalyserNode | undefined>(undefined);
  const progressTimerRef = useRef(0);
  const disconnectTimerRef = useRef(0);
  const generationRef = useRef(0);
  const startedAtRef = useRef(0);
  const durationMsRef = useRef(0);
  const diagnosticRequestRef = useRef(0);
  const diagnosticWaiterRef = useRef<DiagnosticWaiter | undefined>(undefined);
  const lastDiagnosticsRef = useRef<WorkletDiagnostics | undefined>(undefined);
  const disposedWorkletsRef = useRef(new WeakSet<AudioWorkletNode>());
  const lifecycleRef = useRef({
    contextsClosed: 0,
    contextsCreated: 0,
    contextsResumed: 0,
    contextsSuspended: 0,
    eventLoopDelaysOver50Ms: 0,
    eventLoopMaxDelayMs: 0,
    gainNodesDisconnected: 0,
    stateChanges: [] as Array<{ at: string; state: AudioContextState }>,
    workletsCreated: 0,
    workletsDisposed: 0,
  });
  const [analyser, setAnalyser] = useState<AnalyserNode>();
  const [elapsedMs, setElapsedMs] = useState(0);
  const [error, setError] = useState("");
  const [runtime, setRuntime] = useState<JudgementRuntime>("idle");
  const [sampleRate, setSampleRate] = useState(0);

  const stopProgress = useCallback(() => {
    window.clearTimeout(progressTimerRef.current);
    progressTimerRef.current = 0;
  }, []);

  const acceptDiagnostics = useCallback((diagnostics: WorkletDiagnostics) => {
    lastDiagnosticsRef.current = diagnostics;
    const waiter = diagnosticWaiterRef.current;
    if (!waiter || waiter.requestId !== diagnostics.requestId) return;
    window.clearTimeout(waiter.timeout);
    diagnosticWaiterRef.current = undefined;
    waiter.resolve(diagnostics);
  }, []);

  const requestDiagnostics = useCallback((worklet = workletRef.current) => {
    if (!worklet || disposedWorkletsRef.current.has(worklet)) {
      return Promise.resolve(lastDiagnosticsRef.current);
    }
    const requestId = ++diagnosticRequestRef.current;
    const previous = diagnosticWaiterRef.current;
    if (previous) {
      window.clearTimeout(previous.timeout);
      previous.resolve(lastDiagnosticsRef.current);
    }
    return new Promise<WorkletDiagnostics | undefined>((resolve) => {
      const timeout = window.setTimeout(() => {
        if (diagnosticWaiterRef.current?.requestId === requestId) {
          diagnosticWaiterRef.current = undefined;
        }
        resolve(lastDiagnosticsRef.current);
      }, DEBUG_TIMEOUT_MS);
      diagnosticWaiterRef.current = { requestId, resolve, timeout };
      worklet.port.postMessage({ type: "diagnostics", requestId });
    });
  }, []);

  const disposeWorklet = useCallback((worklet: AudioWorkletNode | undefined) => {
    if (!worklet || disposedWorkletsRef.current.has(worklet)) return;
    disposedWorkletsRef.current.add(worklet);
    lifecycleRef.current.workletsDisposed += 1;
    worklet.port.postMessage({ type: "dispose" });
    worklet.port.onmessage = null;
    worklet.disconnect();
  }, []);

  const finish = useCallback(() => {
    stopProgress();
    const gain = candidateGainRef.current;
    const worklet = workletRef.current;
    const context = contextRef.current;
    const generation = generationRef.current;
    void requestDiagnostics(worklet);
    window.clearTimeout(disconnectTimerRef.current);
    disconnectTimerRef.current = window.setTimeout(async () => {
      disposeWorklet(worklet);
      if (gain) {
        gain.disconnect();
        lifecycleRef.current.gainNodesDisconnected += 1;
      }
      if (workletRef.current === worklet) workletRef.current = undefined;
      if (candidateGainRef.current === gain) candidateGainRef.current = undefined;
      if (generationRef.current === generation && context?.state === "running") {
        try {
          await context.suspend();
          lifecycleRef.current.contextsSuspended += 1;
          if (generationRef.current !== generation) await context.resume();
        } catch {
          // Navigation or device loss may close the context during teardown.
        }
      }
    }, 60);
    setElapsedMs(durationMsRef.current);
    setRuntime("complete");
  }, [disposeWorklet, requestDiagnostics, stopProgress]);

  const updateProgress = useCallback(() => {
    const context = contextRef.current;
    if (!context) return;
    const elapsed = Math.min(
      durationMsRef.current,
      Math.max(0, (context.currentTime - startedAtRef.current) * 1_000),
    );
    setElapsedMs(elapsed);
    if (elapsed >= durationMsRef.current) {
      finish();
      return;
    }
    progressTimerRef.current = window.setTimeout(updateProgress, PROGRESS_REFRESH_MS);
  }, [finish]);

  const ensureAudio = useCallback(async () => {
    if (contextRef.current && wasmBytesRef.current && masterGainRef.current) {
      return contextRef.current;
    }
    const context = new AudioContext({ latencyHint: "playback" });
    lifecycleRef.current.contextsCreated += 1;
    const recordState = () => {
      const stateChanges = lifecycleRef.current.stateChanges;
      stateChanges.push({ at: new Date().toISOString(), state: context.state });
      if (stateChanges.length > 12) stateChanges.shift();
    };
    context.addEventListener("statechange", recordState);
    recordState();
    const [wasmResponse] = await Promise.all([
      fetch(WASM_URL, { cache: "no-store" }),
      context.audioWorklet.addModule(WORKLET_URL),
    ]);
    if (!wasmResponse.ok) throw new Error(`WASM request failed: ${wasmResponse.status}`);
    const masterGain = new GainNode(context, { gain: initialVolume });
    const nextAnalyser = new AnalyserNode(context, {
      fftSize: 512,
      smoothingTimeConstant: 0.72,
    });
    masterGain.connect(nextAnalyser).connect(context.destination);
    contextRef.current = context;
    masterGainRef.current = masterGain;
    analyserRef.current = nextAnalyser;
    wasmBytesRef.current = await wasmResponse.arrayBuffer();
    setAnalyser(nextAnalyser);
    setSampleRate(context.sampleRate);
    return context;
  }, [initialVolume]);

  const start = useCallback(async (candidate: CandidateIdentity) => {
    const generation = ++generationRef.current;
    window.clearTimeout(disconnectTimerRef.current);
    stopProgress();
    setError("");
    setElapsedMs(0);
    durationMsRef.current = candidate.durationMs;
    setRuntime("loading");
    try {
      const context = await ensureAudio();
      await context.resume();
      lifecycleRef.current.contextsResumed += 1;
      if (generation !== generationRef.current) return;

      disposeWorklet(workletRef.current);
      if (candidateGainRef.current) {
        candidateGainRef.current.disconnect();
        lifecycleRef.current.gainNodesDisconnected += 1;
      }
      workletRef.current = undefined;
      candidateGainRef.current = undefined;
      const worklet = new AudioWorkletNode(context, "lofi-processor", {
        numberOfInputs: 0,
        numberOfOutputs: 1,
        outputChannelCount: [2],
        processorOptions: {
          bpmMilli: candidate.bpm * 1_000,
          initialNodes: candidate.nodeCount,
          seed: candidate.seed,
          startPhrase: candidate.startPhrase,
          telemetry: false,
          wasmBytes: wasmBytesRef.current!,
        },
      });
      lifecycleRef.current.workletsCreated += 1;
      const candidateGain = new GainNode(context, { gain: 0 });
      worklet.connect(candidateGain).connect(masterGainRef.current!);
      workletRef.current = worklet;
      candidateGainRef.current = candidateGain;

      await waitUntilReady(worklet, acceptDiagnostics);
      if (generation !== generationRef.current) {
        disposeWorklet(worklet);
        candidateGain.disconnect();
        lifecycleRef.current.gainNodesDisconnected += 1;
        return;
      }
      const startedAt = context.currentTime;
      const endsAt = startedAt + candidate.durationMs / 1_000;
      candidateGain.gain.setValueAtTime(0, startedAt);
      candidateGain.gain.linearRampToValueAtTime(1, startedAt + 0.035);
      candidateGain.gain.setValueAtTime(1, endsAt - 0.045);
      candidateGain.gain.linearRampToValueAtTime(0, endsAt);
      startedAtRef.current = startedAt;
      setRuntime("playing");
      updateProgress();
    } catch (cause) {
      if (generation !== generationRef.current) return;
      disposeWorklet(workletRef.current);
      workletRef.current = undefined;
      candidateGainRef.current?.disconnect();
      candidateGainRef.current = undefined;
      setError(cause instanceof Error ? cause.message : String(cause));
      setRuntime("error");
    }
  }, [acceptDiagnostics, disposeWorklet, ensureAudio, stopProgress, updateProgress]);

  const togglePause = useCallback(async () => {
    const context = contextRef.current;
    if (!context) return;
    if (runtime === "playing") {
      stopProgress();
      await context.suspend();
      lifecycleRef.current.contextsSuspended += 1;
      setRuntime("paused");
    } else if (runtime === "paused") {
      await context.resume();
      lifecycleRef.current.contextsResumed += 1;
      setRuntime("playing");
      updateProgress();
    }
  }, [runtime, stopProgress, updateProgress]);

  const setVolume = useCallback((volume: number) => {
    const context = contextRef.current;
    const gain = masterGainRef.current;
    if (context && gain) gain.gain.setTargetAtTime(volume, context.currentTime, 0.015);
  }, []);

  const debugSnapshot = useCallback(async (candidate: CandidateIdentity) => {
    const diagnostics = await requestDiagnostics();
    const context = contextRef.current;
    const performanceWithMemory = performance as Performance & {
      memory?: { jsHeapSizeLimit: number; totalJSHeapSize: number; usedJSHeapSize: number };
    };
    const navigatorWithDeviceMemory = navigator as Navigator & { deviceMemory?: number };
    return JSON.stringify({
      appVersion: APP_VERSION,
      capturedAt: new Date().toISOString(),
      candidate,
      playback: {
        durationMs: durationMsRef.current,
        elapsedMs,
        generation: generationRef.current,
        runtime,
      },
      context: context ? {
        baseLatencySeconds: context.baseLatency,
        currentTime: context.currentTime,
        outputLatencySeconds: "outputLatency" in context ? context.outputLatency : null,
        sampleRate: context.sampleRate,
        state: context.state,
      } : null,
      lifecycle: lifecycleRef.current,
      graph: {
        analyser: Boolean(analyserRef.current),
        candidateGain: Boolean(candidateGainRef.current),
        masterGain: Boolean(masterGainRef.current),
        worklet: Boolean(workletRef.current),
      },
      worklet: diagnostics ?? null,
      browser: {
        deviceMemoryGiB: navigatorWithDeviceMemory.deviceMemory ?? null,
        hardwareConcurrency: navigator.hardwareConcurrency,
        language: navigator.language,
        userAgent: navigator.userAgent,
        visibilityState: document.visibilityState,
      },
      display: {
        devicePixelRatio: window.devicePixelRatio,
        height: window.innerHeight,
        width: window.innerWidth,
      },
      memory: performanceWithMemory.memory ?? null,
    }, null, 2);
  }, [elapsedMs, requestDiagnostics, runtime]);

  useEffect(() => {
    let timer = 0;
    let expectedAt = performance.now() + 1_000;
    const sampleEventLoop = () => {
      const now = performance.now();
      const delay = Math.max(0, now - expectedAt);
      lifecycleRef.current.eventLoopMaxDelayMs = Math.max(
        lifecycleRef.current.eventLoopMaxDelayMs,
        delay,
      );
      if (delay >= 50) lifecycleRef.current.eventLoopDelaysOver50Ms += 1;
      expectedAt = now + 1_000;
      timer = window.setTimeout(sampleEventLoop, 1_000);
    };
    timer = window.setTimeout(sampleEventLoop, 1_000);
    return () => window.clearTimeout(timer);
  }, []);

  useEffect(() => () => {
    generationRef.current += 1;
    window.clearTimeout(disconnectTimerRef.current);
    stopProgress();
    disposeWorklet(workletRef.current);
    candidateGainRef.current?.disconnect();
    masterGainRef.current?.disconnect();
    analyserRef.current?.disconnect();
    if (contextRef.current) {
      lifecycleRef.current.contextsClosed += 1;
      void contextRef.current.close();
    }
  }, [disposeWorklet, stopProgress]);

  return {
    analyser,
    debugSnapshot,
    elapsedMs,
    error,
    runtime,
    sampleRate,
    setVolume,
    start,
    togglePause,
  };
}

function waitUntilReady(
  worklet: AudioWorkletNode,
  onDiagnostics: (diagnostics: WorkletDiagnostics) => void,
) {
  return new Promise<void>((resolve, reject) => {
    const timeout = window.setTimeout(() => reject(new Error("AudioWorklet startup timed out")), 5_000);
    worklet.port.onmessage = (event: MessageEvent<WorkletMessage>) => {
      if (event.data.type === "error") {
        window.clearTimeout(timeout);
        reject(new Error(event.data.message));
      } else if (event.data.type === "ready") {
        window.clearTimeout(timeout);
        if (event.data.version !== APP_VERSION) {
          reject(new Error(`Audio version mismatch: page ${APP_VERSION}, processor ${event.data.version}`));
        } else {
          resolve();
        }
      } else if (event.data.type === "diagnostics") {
        onDiagnostics(event.data);
      }
    };
  });
}
