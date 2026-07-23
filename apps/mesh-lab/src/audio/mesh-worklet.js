const MAX_NODES = 10;
const PACKET_SLOTS = 256;
const WIRE_CAPACITY = 72;
const TELEMETRY_HZ = 10;
const NETWORK_POLL_HZ = 200;
const PRE_ROLL_US = 2_000_000;
const BLOCK_SMOOTHING = 1 - (1 - 0.004) ** 128;
const WORKLET_VERSION = "2026.07.16.3";
const PROCESS_TIMING_INTERVAL = 16;
const LARGE_BOUNDARY_JUMP = 0.15;

class LofiProcessor extends AudioWorkletProcessor {
  constructor(options) {
    super();
    try {
      const config = options.processorOptions;
      this.module = new WebAssembly.Module(config.wasmBytes);
      this.seed = config.seed;
      this.bpmMilli = config.bpmMilli ?? 80_000;
      this.startPhrase = config.startPhrase ?? 0;
      this.engine = config.engine === "loops" ? 1 : 0;
      this.telemetryEnabled = config.telemetry !== false;
      this.disposed = false;
      this.nodes = [];
      this.nextNodeId = 1;
      this.globalFrame = 0;
      this.randomState = 0x6d2b79f5;
      this.network = { enabled: true, lossPercent: 0, latencyMs: 2, jitterMs: 0.5 };
      this.packetStats = { sent: 0, delivered: 0, dropped: 0 };
      this.processCalls = 0;
      this.renderedFrames = 0;
      this.quantumGaps = 0;
      this.maxQuantumGapFrames = 0;
      this.lastEngineFrame = null;
      this.lastCallbackAtMs = null;
      this.maxCallbackIntervalMs = 0;
      this.lateCallbacks = 0;
      this.sampledProcessCalls = 0;
      this.processTimeTotalMs = 0;
      this.processTimeMaxMs = 0;
      this.outputPeak = 0;
      this.clippedSamples = 0;
      this.boundaryJumpMax = 0;
      this.largeBoundaryJumps = 0;
      this.lastLeftSample = 0;
      this.lastRightSample = 0;
      this.packets = Array.from({ length: PACKET_SLOTS }, () => ({
        active: false,
        deliverAt: 0,
        targetId: 0,
        length: 0,
        bytes: new Uint8Array(WIRE_CAPACITY),
      }));

      for (let index = 0; index < config.initialNodes; index += 1) this.addNode();
      this.preRoll(PRE_ROLL_US);
      this.networkFrames = Math.max(128, Math.round(sampleRate / NETWORK_POLL_HZ));
      this.nextNetworkFrame = this.globalFrame;
      this.telemetryFrames = Math.max(128, Math.round(sampleRate / TELEMETRY_HZ));
      this.nextTelemetryFrame = this.globalFrame;
      this.port.onmessage = (event) => this.handleCommand(event.data);
      this.postTelemetry("ready");
    } catch (error) {
      this.failed = true;
      this.port.postMessage({ type: "error", message: String(error) });
    }
  }

  addNode() {
    if (this.nodes.length >= MAX_NODES) return;
    const id = this.nextNodeId++;
    const instance = new WebAssembly.Instance(this.module, {});
    const exports = instance.exports;
    if (
      exports.lofi_render_frames() !== 128 ||
      exports.lofi_wire_capacity() !== WIRE_CAPACITY ||
      exports.lofi_status_fields() !== 15 ||
      typeof exports.lofi_init_at_position !== "function"
    ) {
      throw new Error("WASM ABI does not match the browser substrate");
    }
    const songZeroLow = lowMicros(PRE_ROLL_US);
    const songZeroHigh = highMicros(PRE_ROLL_US);
    exports.lofi_init_at_position(
      Math.round(sampleRate),
      this.seed,
      id,
      songZeroLow,
      songZeroHigh,
      this.bpmMilli,
      this.startPhrase,
    );
    if (this.engine !== 0 && typeof exports.lofi_set_engine === "function") {
      exports.lofi_set_engine(this.engine);
    }
    this.nodes.push({
      id,
      instance,
      exports,
      offsetUs: id === 1 ? 0 : (id % 2 === 0 ? 45_000 : -32_000),
      driftPpm: id === 1 ? 0 : id % 2 === 0 ? 65 : -48,
      // Real modules are independent mono speakers. Keep the combined monitor
      // centered until the listener deliberately places them with the pan control.
      pan: 0,
      currentPan: 0,
      gain: 0.9,
      currentGain: 0,
      pcm: null,
      mute: false,
      solo: false,
      radio: true,
      removeAt: 0,
    });
  }

  handleCommand(command) {
    if (command.type === "dispose") {
      this.disposed = true;
      this.clearPackets();
      this.nodes.length = 0;
      this.packets.length = 0;
      this.module = null;
      this.port.close?.();
      return;
    }
    if (this.disposed) return;
    if (command.type === "addNode") {
      this.addNode();
    } else if (command.type === "diagnostics") {
      this.postTelemetry("diagnostics", command.requestId);
      return;
    } else if (command.type === "removeNode") {
      const node = this.nodeById(command.id);
      if (node && this.nodes.length > 1) node.removeAt = this.globalFrame + Math.round(sampleRate * 0.12);
    } else if (command.type === "node") {
      const node = this.nodeById(command.id);
      if (node && Object.hasOwn(node, command.key)) node[command.key] = command.value;
    } else if (command.type === "network") {
      if (Object.hasOwn(this.network, command.key)) this.network[command.key] = command.value;
      if (!this.network.enabled) this.clearPackets();
    }
    this.postTelemetry("telemetry");
  }

  process(_inputs, outputs) {
    if (this.failed || this.disposed) return false;
    const callbackAtMs = clockNowMs();
    if (callbackAtMs !== null && this.lastCallbackAtMs !== null) {
      const intervalMs = callbackAtMs - this.lastCallbackAtMs;
      this.maxCallbackIntervalMs = Math.max(this.maxCallbackIntervalMs, intervalMs);
      if (intervalMs > (128 / sampleRate) * 1_000 * 1.75) this.lateCallbacks += 1;
    }
    this.lastCallbackAtMs = callbackAtMs;
    const engineFrame = typeof currentFrame === "number" ? currentFrame : null;
    if (engineFrame !== null && this.lastEngineFrame !== null) {
      const gap = engineFrame - this.lastEngineFrame;
      if (gap !== 128) {
        this.quantumGaps += 1;
        this.maxQuantumGapFrames = Math.max(this.maxQuantumGapFrames, gap);
      }
    }
    this.lastEngineFrame = engineFrame;
    const timed = this.processCalls % PROCESS_TIMING_INTERVAL === 0 && callbackAtMs !== null;
    const processStartedAt = timed ? callbackAtMs : 0;
    const output = outputs[0];
    if (!output || output.length === 0) return true;
    const left = output[0];
    const right = output[1] || output[0];
    left.fill(0);
    if (right !== left) right.fill(0);

    const globalUs = (this.globalFrame * 1_000_000) / sampleRate;
    if (this.globalFrame >= this.nextNetworkFrame) {
      this.processNetwork(globalUs);
      this.nextNetworkFrame += this.networkFrames;
    }
    let anySolo = false;
    for (const node of this.nodes) {
      if (node.solo && !node.removeAt) {
        anySolo = true;
        break;
      }
    }
    let monitoredNodes = 0;
    for (const node of this.nodes) {
      if (!node.removeAt && (!anySolo || node.solo)) monitoredNodes += 1;
    }
    // This only controls the combined browser monitor. Physical modules retain
    // their full local level regardless of group size.
    // Modules share sample-accurate sources, so duplicate lanes add coherently.
    // Linear normalization is required here; sqrt(N) still clips larger groups.
    const monitorScale = 3 / Math.max(3, monitoredNodes);
    for (const node of this.nodes) {
      const localUs = this.localTime(node, globalUs);
      const pointer = node.exports.lofi_render(lowMicros(localUs), highMicros(localUs));
      if (!node.pcm) node.pcm = new Int16Array(node.exports.memory.buffer, pointer, 128);
      const targetGain = node.mute || (anySolo && !node.solo) || node.removeAt ? 0 : node.gain;
      node.currentGain += (targetGain - node.currentGain) * BLOCK_SMOOTHING;
      node.currentPan += (node.pan - node.currentPan) * BLOCK_SMOOTHING;
      const angle = (Math.max(-1, Math.min(1, node.currentPan)) + 1) * Math.PI * 0.25;
      const leftGain = Math.cos(angle) * node.currentGain * monitorScale;
      const rightGain = Math.sin(angle) * node.currentGain * monitorScale;
      for (let frame = 0; frame < 128; frame += 1) {
        const sample = node.pcm[frame] / 32768;
        left[frame] += sample * leftGain;
        right[frame] += sample * rightGain;
      }
    }
    for (let frame = 0; frame < 128; frame += 1) {
      left[frame] = Math.max(-1, Math.min(1, left[frame] * 0.88));
      right[frame] = Math.max(-1, Math.min(1, right[frame] * 0.88));
      const peak = Math.max(Math.abs(left[frame]), Math.abs(right[frame]));
      this.outputPeak = Math.max(this.outputPeak, peak);
      if (peak >= 0.9999) this.clippedSamples += 1;
    }

    const boundaryJump = Math.max(
      Math.abs(left[0] - this.lastLeftSample),
      Math.abs(right[0] - this.lastRightSample),
    );
    this.boundaryJumpMax = Math.max(this.boundaryJumpMax, boundaryJump);
    if (boundaryJump >= LARGE_BOUNDARY_JUMP) this.largeBoundaryJumps += 1;
    this.lastLeftSample = left[127];
    this.lastRightSample = right[127];

    this.globalFrame += 128;
    this.renderedFrames += 128;
    this.processCalls += 1;
    for (let index = this.nodes.length - 1; index >= 0; index -= 1) {
      const node = this.nodes[index];
      if (node.removeAt && this.globalFrame >= node.removeAt) this.nodes.splice(index, 1);
    }
    if (this.telemetryEnabled && this.globalFrame >= this.nextTelemetryFrame) {
      this.nextTelemetryFrame = this.globalFrame + this.telemetryFrames;
      this.postTelemetry("telemetry");
    }
    if (timed) {
      const finishedAt = clockNowMs();
      if (finishedAt !== null) {
        const elapsedMs = finishedAt - processStartedAt;
        this.sampledProcessCalls += 1;
        this.processTimeTotalMs += elapsedMs;
        this.processTimeMaxMs = Math.max(this.processTimeMaxMs, elapsedMs);
      }
    }
    return true;
  }

  preRoll(durationUs) {
    for (let globalUs = 0; globalUs < durationUs; globalUs += 2_500) this.processNetwork(globalUs);
    this.globalFrame = Math.round((durationUs * sampleRate) / 1_000_000);
  }

  processNetwork(globalUs) {
    this.deliverPackets(globalUs);
    if (!this.network.enabled) return;
    for (const node of this.nodes) {
      if (!node.radio || node.removeAt) continue;
      const localUs = this.localTime(node, globalUs);
      const low = lowMicros(localUs);
      const high = highMicros(localUs);
      const beaconLength = node.exports.lofi_poll_beacon(low, high);
      if (beaconLength > 0) this.transmit(node, beaconLength, globalUs);
      const probeLength = node.exports.lofi_poll_probe(low, high);
      if (probeLength > 0) this.transmit(node, probeLength, globalUs);
    }
  }

  transmit(source, length, globalUs) {
    const destination = source.exports.lofi_tx_destination();
    const pointer = source.exports.lofi_tx_ptr();
    const bytes = new Uint8Array(source.exports.memory.buffer, pointer, length);
    this.packetStats.sent += 1;
    if (destination === 0) {
      for (const target of this.nodes) {
        if (target.id !== source.id) this.schedulePacket(source, target, bytes, globalUs);
      }
    } else {
      const target = this.nodeById(destination);
      if (target) this.schedulePacket(source, target, bytes, globalUs);
    }
  }

  schedulePacket(source, target, bytes, globalUs) {
    if (
      !this.network.enabled || !source.radio || !target.radio ||
      this.random() * 100 < this.network.lossPercent
    ) {
      this.packetStats.dropped += 1;
      return;
    }
    const slot = this.packets.find((packet) => !packet.active);
    if (!slot) {
      this.packetStats.dropped += 1;
      return;
    }
    const jitter = (this.random() * 2 - 1) * this.network.jitterMs;
    slot.active = true;
    slot.deliverAt = globalUs + Math.max(0, this.network.latencyMs + jitter) * 1_000;
    slot.targetId = target.id;
    slot.length = bytes.length;
    slot.bytes.set(bytes.subarray(0, bytes.length));
  }

  deliverPackets(globalUs) {
    for (const packet of this.packets) {
      if (!packet.active || packet.deliverAt > globalUs) continue;
      packet.active = false;
      const target = this.nodeById(packet.targetId);
      if (!target || !target.radio || target.removeAt) continue;
      const pointer = target.exports.lofi_rx_ptr();
      new Uint8Array(target.exports.memory.buffer, pointer, packet.length)
        .set(packet.bytes.subarray(0, packet.length));
      const localUs = this.localTime(target, globalUs);
      const low = lowMicros(localUs);
      const high = highMicros(localUs);
      const replyLength = target.exports.lofi_receive(packet.length, low, high);
      this.packetStats.delivered += 1;
      if (replyLength > 0) this.transmit(target, replyLength, globalUs);
    }
  }

  postTelemetry(type, requestId) {
    const globalUs = (this.globalFrame * 1_000_000) / sampleRate;
    const nodes = this.nodes.map((node) => {
      const localUs = this.localTime(node, globalUs);
      const low = lowMicros(localUs);
      const high = highMicros(localUs);
      const pointer = node.exports.lofi_status(low, high);
      const status = new Int32Array(node.exports.memory.buffer, pointer, 15);
      return {
        id: status[0], rootId: status[1], peers: status[2], dispersionUs: status[3],
        role: status[4], synced: status[5] === 1, meshOffsetUs: status[6],
        beatPhase: status[7], isRoot: status[8] === 1, driftPpm: node.driftPpm,
        beatsToChangeMilli: status[9], upcomingChange: status[10],
        roleMask: status[11],
        spotlight: status[12], phrase: status[13], selectorId: status[14],
        offsetUs: node.offsetUs, pan: node.pan, gain: node.gain, mute: node.mute,
        solo: node.solo, radio: node.radio,
      };
    });
    const message = {
      type,
      version: WORKLET_VERSION,
      sampleRate,
      nodes,
      network: {
        ...this.network,
        queued: this.packets.filter((packet) => packet.active).length,
        ...this.packetStats,
      },
    };
    if (type === "diagnostics") {
      message.requestId = requestId;
      message.audio = {
        boundaryJumpMax: this.boundaryJumpMax,
        clippedSamples: this.clippedSamples,
        currentFrame: this.lastEngineFrame,
        globalFrame: this.globalFrame,
        largeBoundaryJumps: this.largeBoundaryJumps,
        lateCallbacks: this.lateCallbacks,
        maxCallbackIntervalMs: this.maxCallbackIntervalMs,
        maxProcessMs: this.processTimeMaxMs,
        maxQuantumGapFrames: this.maxQuantumGapFrames,
        meanSampledProcessMs: this.sampledProcessCalls
          ? this.processTimeTotalMs / this.sampledProcessCalls
          : null,
        outputPeak: this.outputPeak,
        processCalls: this.processCalls,
        quantumBudgetMs: (128 / sampleRate) * 1_000,
        quantumGaps: this.quantumGaps,
        renderedFrames: this.renderedFrames,
        sampledProcessCalls: this.sampledProcessCalls,
        timingSource: typeof globalThis.performance?.now === "function" ? "performance" : "date",
      };
    }
    this.port.postMessage(message);
  }

  localTime(node, globalUs) {
    return Math.round(globalUs + node.offsetUs + (globalUs * node.driftPpm) / 1_000_000);
  }

  nodeById(id) {
    return this.nodes.find((node) => node.id === id);
  }

  clearPackets() {
    for (const packet of this.packets) packet.active = false;
  }

  random() {
    let value = this.randomState >>> 0;
    value ^= value << 13;
    value ^= value >>> 17;
    value ^= value << 5;
    this.randomState = value >>> 0;
    return this.randomState / 0x1_0000_0000;
  }
}

function lowMicros(value) {
  const micros = Math.trunc(value);
  return micros >>> 0;
}

function highMicros(value) {
  return Math.floor(Math.trunc(value) / 0x1_0000_0000) | 0;
}

function clockNowMs() {
  if (typeof globalThis.performance?.now === "function") return globalThis.performance.now();
  if (typeof globalThis.Date?.now === "function") return globalThis.Date.now();
  return null;
}

registerProcessor("lofi-processor", LofiProcessor);
