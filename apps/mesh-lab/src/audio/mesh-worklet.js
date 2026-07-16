const MAX_NODES = 8;
const PACKET_SLOTS = 256;
const WIRE_CAPACITY = 72;
const TELEMETRY_HZ = 10;

class LofiProcessor extends AudioWorkletProcessor {
  constructor(options) {
    super();
    try {
      const config = options.processorOptions;
      this.module = new WebAssembly.Module(config.wasmBytes);
      this.seed = config.seed;
      this.nodes = [];
      this.nextNodeId = 1;
      this.globalFrame = 0;
      this.randomState = 0x6d2b79f5;
      this.network = { enabled: true, lossPercent: 0, latencyMs: 2, jitterMs: 0.5 };
      this.packetStats = { sent: 0, delivered: 0, dropped: 0 };
      this.packets = Array.from({ length: PACKET_SLOTS }, () => ({
        active: false,
        deliverAt: 0,
        targetId: 0,
        length: 0,
        bytes: new Uint8Array(WIRE_CAPACITY),
      }));

      for (let index = 0; index < config.initialNodes; index += 1) this.addNode();
      this.preRoll(2_000_000);
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
      exports.lofi_status_fields() !== 10
    ) {
      throw new Error("WASM ABI does not match the browser substrate");
    }
    exports.lofi_init(Math.round(sampleRate), this.seed, id);
    const spread = this.nodes.length - (Math.max(3, this.nodes.length + 1) - 1) / 2;
    const pan = Math.max(-0.7, Math.min(0.7, spread * 0.5));
    this.nodes.push({
      id,
      instance,
      exports,
      offsetUs: id === 1 ? 0 : (id % 2 === 0 ? 45_000 : -32_000),
      driftPpm: id === 1 ? 0 : id % 2 === 0 ? 65 : -48,
      pan,
      currentPan: pan,
      gain: 0.9,
      currentGain: 0,
      mute: false,
      solo: false,
      radio: true,
      removeAt: 0,
    });
  }

  handleCommand(command) {
    if (command.type === "addNode") {
      this.addNode();
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
    if (this.failed) return true;
    const output = outputs[0];
    if (!output || output.length === 0) return true;
    const left = output[0];
    const right = output[1] || output[0];
    left.fill(0);
    if (right !== left) right.fill(0);

    const globalUs = (this.globalFrame * 1_000_000) / sampleRate;
    this.processNetwork(globalUs);
    const anySolo = this.nodes.some((node) => node.solo && !node.removeAt);
    for (const node of this.nodes) {
      const [low, high] = splitMicros(this.localTime(node, globalUs));
      const pointer = node.exports.lofi_render(low, high);
      const pcm = new Int16Array(node.exports.memory.buffer, pointer, 128);
      const targetGain = node.mute || (anySolo && !node.solo) || node.removeAt ? 0 : node.gain;
      for (let frame = 0; frame < 128; frame += 1) {
        node.currentGain += (targetGain - node.currentGain) * 0.004;
        node.currentPan += (node.pan - node.currentPan) * 0.004;
        const angle = (Math.max(-1, Math.min(1, node.currentPan)) + 1) * Math.PI * 0.25;
        const sample = (pcm[frame] / 32768) * node.currentGain;
        left[frame] += sample * Math.cos(angle);
        right[frame] += sample * Math.sin(angle);
      }
    }
    for (let frame = 0; frame < 128; frame += 1) {
      left[frame] = Math.max(-1, Math.min(1, left[frame] * 0.88));
      right[frame] = Math.max(-1, Math.min(1, right[frame] * 0.88));
    }

    this.globalFrame += 128;
    this.nodes = this.nodes.filter((node) => !node.removeAt || this.globalFrame < node.removeAt);
    if (this.globalFrame >= this.nextTelemetryFrame) {
      this.nextTelemetryFrame = this.globalFrame + this.telemetryFrames;
      this.postTelemetry("telemetry");
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
      const [low, high] = splitMicros(this.localTime(node, globalUs));
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
      const [low, high] = splitMicros(this.localTime(target, globalUs));
      const replyLength = target.exports.lofi_receive(packet.length, low, high);
      this.packetStats.delivered += 1;
      if (replyLength > 0) this.transmit(target, replyLength, globalUs);
    }
  }

  postTelemetry(type) {
    const globalUs = (this.globalFrame * 1_000_000) / sampleRate;
    const nodes = this.nodes.map((node) => {
      const [low, high] = splitMicros(this.localTime(node, globalUs));
      const pointer = node.exports.lofi_status(low, high);
      const status = new Int32Array(node.exports.memory.buffer, pointer, 10);
      return {
        id: status[0], rootId: status[1], peers: status[2], dispersionUs: status[3],
        role: status[4], synced: status[5] === 1, meshOffsetUs: status[6],
        beatPhase: status[7], isRoot: status[8] === 1, driftPpm: node.driftPpm,
        changeInMs: status[9],
        offsetUs: node.offsetUs, pan: node.pan, gain: node.gain, mute: node.mute,
        solo: node.solo, radio: node.radio,
      };
    });
    this.port.postMessage({
      type,
      sampleRate,
      nodes,
      network: {
        ...this.network,
        queued: this.packets.filter((packet) => packet.active).length,
        ...this.packetStats,
      },
    });
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

function splitMicros(value) {
  const micros = Math.trunc(value);
  return [micros >>> 0, Math.floor(micros / 0x1_0000_0000) | 0];
}

registerProcessor("lofi-processor", LofiProcessor);
