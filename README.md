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

Open <http://localhost:5173/> in the host browser and press **Start**.
The Vite predev hook builds the Rust WASM module and stages it for the app.
Three WASM devices start by default. Add/remove modules, disconnect individual
modules, change loss/latency/jitter, alter device clock drift, and monitor root
election, multi-role assignment, peer count, and sync quality in real time. The
lab supports ten modules; every box remains locally musical while the full group
keeps distinct rhythm and tonal responsibilities. Sequencing,
sample playback, arrangement, filtering, wire encoding, peer tracking, and clock
discipline remain in the same `no_std` runtime intended for firmware. JavaScript
only models the radio medium, mixes mono speaker outputs for monitoring, and
renders controls.

The music evolves automatically at shared eight-bar phrase boundaries. The
settings panel counts down to the next synchronized change; there is no
user-facing seed or fixed tape selector.

Open <http://localhost:5173/judge> for the blinded eight-bar listening study.
Its tick/cross judgements, structured tags, and optional notes persist locally
and on this development box for reproducible taste-model analysis.

### Exact browser-path WAV render

```sh
npm run build:web
node tools/listen-qa/render.mjs --seed 2 --nodes 5 --duration 96 \
  --output target/listen-qa/seed-2.wav
```

This executes the production AudioWorklet and one WASM instance per module,
including the simulated mesh substrate and browser listener mix. See
[Listen QA](docs/LISTEN_QA.md) for the automated and human acceptance gates.

The default groove is **symbolic**: every note and hit exists as data — lane,
step, pitch, velocity, micro-delay — derived from `(seed, roster, mesh time)`
before it touches a sample. Root-tagged one-shots from the fixed 5.95 MiB pack
supply timbre only; harmony, basslines, comping, and motifs come from the
`no_std` composer described in [Symbolic Music](docs/SYMBOLIC_MUSIC.md). The
prior loop engine (source-coherent stem scenes) remains selectable for blinded
A/B listening studies. No oscillator, allocator, or mutable playback cursor
runs in the audio path, and the exact score of any session is dumpable as
JSONL for property testing without listening.

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
- [Symbolic Music](docs/SYMBOLIC_MUSIC.md)
- [Music Engine](docs/MUSIC_ENGINE.md)
- [Listen QA](docs/LISTEN_QA.md)
- [AI Content Pipeline](docs/AI_CONTENT_PIPELINE.md)
- [Simulator UI](docs/SIMULATOR_UI.md)
- [Commercialization Roadmap](docs/COMMERCIALIZATION_ROADMAP.md)
