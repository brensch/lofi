# Lofi

Foundation for an ESP32-S3 lo-fi box that keeps multiple devices in time over ESP-NOW and can be prototyped in a browser first.

## Current Shape

- `crates/lofi-core`: `no_std` timing, sync, packet, transport, scheduled events, deterministic role-based sample scheduling, fixed-pack lookup, stateless sample playback, and bounded mix DSP.
- `crates/lofi-app`: `no_std` **device runtime** — the code that runs identically on hardware and in the simulator. A `Device` owns one box's clock, transport, groove, event queue, play state, and LCD model; it renders mono audio and a 128x64 SSD1306 framebuffer from shared mesh state. Neither the firmware nor the simulator re-implements the musical behavior.
- `crates/lofi-sim`: host simulation kernel (library + WAV bin) that drives real `Device`s through a simulated ESP-NOW network (drift, loss, latency), a scheduled drop, and a stereo monitor mix. Pan/solo/volume are listener-side controls; a real box just drives one mono speaker.
- `crates/lofi-web`: allocation-free `no_std` WebAssembly ABI around one real `lofi-app::Device`. Every virtual module is a separate WASM instance with independent memory, clock, peer table, and synth state.
- `apps/mesh-lab`: the sole interactive app, built with Vite, React, and TypeScript. Its `AudioWorklet` hosts the WASM devices and a bounded ESP-NOW substrate that transports their real encoded beacon/probe frames with simulated disconnects, loss, latency, jitter, offsets, and drift.
- `proto/lofi/v1/lofi.proto`: protobuf source of truth for mesh, transport, scheduled event, and groove-state messages.
- `docs/`: product, mesh sync, hardware portability, music engine, and simulator UI notes.

### Browser mesh lab

```sh
npm install
npm run dev
```

Open <http://localhost:5173/> in the host browser and press **Launch mesh**.
The Vite predev hook builds the Rust WASM module and stages it for the app.
Three WASM devices start by default. Add/remove modules, disconnect individual
modules, change loss/latency/jitter, alter device clock drift, and monitor root
election, role assignment, peer count, and sync quality in real time. Sequencing,
sample playback, arrangement, filtering, wire encoding, peer tracking, and clock
discipline remain in the same `no_std` runtime intended for firmware. JavaScript
only models the radio medium, mixes mono speaker outputs for monitoring, and
renders controls.

### Exact browser-path WAV render

```sh
npm run build:web
node tools/listen-qa/render.mjs --seed 2 --nodes 3 --duration 45 \
  --output target/listen-qa/seed-2.wav
```

This executes the production AudioWorklet and one WASM instance per module,
including the simulated mesh substrate and browser listener mix. See
[Listen QA](docs/LISTEN_QA.md) for the automated and human acceptance gates.

The current groove is sample-only. A fixed 5.95 MiB pack supplies 192 harvested
elements. The audible path selects one source-coherent scene per seed and deals
source-matched, transient-bounded drum one-shots plus grid-conformed bass,
harmony, melody, and texture loops across modules.
No oscillator, unrelated sample shuffle, allocator, or mutable playback cursor
runs in the audio path.

## Hardware Direction

The firmware target should use `esp-hal` plus `esp-radio`/ESP-NOW on ESP32-S3 in `no_std`. Current Espressif Rust radio support exposes ESP-NOW through `esp-radio` with the `esp-now` and `unstable` features. The radio runtime may require `esp-alloc`/`esp-rtos`, but the audio engine and musical core should remain fixed-capacity and allocation-free.

ESP-NOW v2 supports 1470-byte payloads, but the current sync frame is 40 bytes so it keeps airtime low and remains comfortably below v1's 250-byte payload size.

Next hardware milestones:

1. Add an `lofi-firmware-esp32s3` crate with `esp-hal`, Embassy timers, ESP-NOW receive/send tasks, LCD, button input, and I2S DAC output.
2. Feed every received ESP-NOW packet into `lofi_core::protocol::Frame::decode`.
3. Use a monotonic microsecond hardware timer as the local clock source for `ClockModel`.
4. Schedule audio edges from root time via `ClockModel::local_from_root`.

## Design Docs

- [Product Notes](docs/PRODUCT.md)
- [Mesh Sync](docs/MESH_SYNC.md)
- [Hardware Portability](docs/HARDWARE_PORTABILITY.md)
- [Music Engine](docs/MUSIC_ENGINE.md)
- [Listen QA](docs/LISTEN_QA.md)
- [AI Content Pipeline](docs/AI_CONTENT_PIPELINE.md)
- [Simulator UI](docs/SIMULATOR_UI.md)
- [Commercialization Roadmap](docs/COMMERCIALIZATION_ROADMAP.md)
