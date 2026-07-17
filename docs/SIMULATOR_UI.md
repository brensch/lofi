# Browser Mesh Lab

## Faithfulness Boundary

The browser is the only interactive simulation surface. Each virtual module is
a separate instance of `lofi-web`, containing one real `lofi-app::Device` with
independent WASM memory, clock discipline, peer table, transport, roles, synth,
filter state, RX buffer, and TX buffer.

Instances never share musical or mesh state. The AudioWorklet may only:

- supply each instance with its simulated monotonic local clock
- copy encoded `lofi_core::mesh::wire` frames between RX/TX buffers
- apply bounded radio latency, jitter, loss, disconnects, and clock drift
- mix each mono speaker output into a listener-side stereo monitor, with
  browser-only linear group normalization for coherent duplicate lanes
- publish low-rate status snapshots to the browser main thread

This matches the firmware boundary: hardware will replace the JavaScript radio,
clock, DAC, and controls without replacing device behavior.

## Realtime Shape

- Web Audio calls the worklet in fixed 128-frame render quanta.
- The worklet owns up to ten independent WASM instances.
- A fixed 256-slot packet pool prevents radio traffic from allocating during
  playback.
- UI rendering and control events stay on the browser main thread.
- The batch `lofi-sim` WAV path remains for deterministic regression artifacts.

## Controls

The lab exposes module add/remove, monitor pan/level/mute/solo, per-module sync,
oscillator drift, global network enable, latency, jitter, packet loss, packet
counters, root, role plan, peer count, and sync quality. Modules default to
centered mono, matching the physical product; pan remains available for
deliberate room simulation.
