# Architecture

## Goal

Build the timing and sequencing core once, then use it from both browser-hosted WASM devices and ESP32-S3 firmware.

The first audible milestone is deliberately simple: every node outputs the same short beep on the same root-time beat grid. If that works under drift, packet loss, and latency jitter in simulation, the beat generator can grow into drums, swing, pattern state, and sample playback without rewriting synchronization.

## Crates

- `lofi-core`: `no_std` packet, clock, sequencing, transport, event, groove, and groove-mode primitives. The shared timing/music math.
- `lofi-app`: `no_std` **device runtime**. A `Device` is the whole audible behavior of one box — clock discipline, transport, groove, scheduled events, play state, peer tracking, mono audio render, and the 128x64 LCD framebuffer. This is the faithfulness boundary: firmware and the simulator both drive `Device`s through the same methods, so neither re-implements the music. Hardware (I2S DMA, ESP-NOW, SSD1306) sits behind firmware glue; the simulator supplies the same surfaces with a drifting virtual clock and host audio.
- `lofi-sim`: `std` host simulation kernel (library + WAV bin). It owns the "hardware truth" of per-device oscillator drift, the simulated ESP-NOW network (loss/latency), and the stereo monitor mix. Mono device output is panned listener-side; the device itself is mono, like the real speaker.
- `lofi-web`: raw `no_std` WASM ABI for one `Device`. The browser creates one independent instance per virtual box and may only provide local time, move encoded mesh packets, and consume fixed audio/status buffers.
- `apps/mesh-lab`: Vite + React + TypeScript browser mesh lab. An `AudioWorklet` owns all WASM instances, a fixed packet pool that mocks the ESP-NOW medium, and the monitor mix. React owns controls and telemetry rendering on the main thread only.
- `proto/lofi/v1`: protobuf source of truth for comms.

`lofi-core` deliberately owns the musical timeline too. The important function is:

```text
tick = f(root_time, song_zero, bpm, ticks_per_beat)
```

That is what lets each box compute the same sequence position locally. Network packets schedule future absolute events; audio never waits for a packet.

## ESP-NOW Notes

ESP-NOW v2 supports larger payloads than v1, but synchronization packets should stay tiny. The current frame is 40 bytes, which is below the v1 250-byte body limit and far below the v2 1470-byte body limit. Small sync packets reduce airtime, collision probability, and receive timestamp ambiguity.

Current sync frame:

- protocol magic/version
- message kind
- sender node id
- root node id
- sequence number
- sender's root-time estimate
- beat period
- flags

## Clock Model

Each follower maintains an affine estimate:

```text
root_time = local_time + offset + local_time * rate
```

`offset` corrects immediate phase error. `rate` corrects oscillator drift. The firmware should feed this model with a monotonic microsecond hardware timer, not wall-clock time.

The production sync protocol is **implemented** in `lofi-core::mesh` and drives both the simulator and (eventually) the firmware via one `SyncEngine` per box: a leaderless-emergent root (lowest live id), NTP-style pairwise probes to upstream peers, a weighted peer table with outlier rejection, a monotonic disciplined clock, and clean split/merge. See [Mesh Sync](docs/MESH_SYNC.md). It is validated by `crates/lofi-core/tests/mesh_convergence.rs` (convergence under drift+loss, monotonic mesh time, root failover, cluster merge).

## Local Prototyping

Run:

```sh
cargo run -p lofi-sim -- --nodes 8 --duration-ms 18000 --sync-start-ms 2500 --group-join-ms 8000 --wav target/lofi-two-clusters-merge.wav
```

The reported phase spread is the sync health check. The WAV is the human check: four virtual devices start on the left and four on the right. Each side syncs internally first; then cross-cluster links open and both sides converge into one mesh. Tight sync sounds like one timing grid with sampled drums, procedural bass, chord stabs, and opposing arpeggios locked together. The simulator also schedules a future drop and seed change so the event system is exercised.

## Firmware Plan

1. Add an ESP32-S3 firmware crate using `esp-hal` and `esp-radio`.
2. Initialize Wi-Fi radio in ESP-NOW mode.
3. Use receive callbacks/tasks to timestamp incoming sync frames as close to interrupt time as the stack allows.
4. Feed decoded frames into `ClockModel::observe`.
5. Schedule audio from root time using `ClockModel::local_from_root`.
6. Add two-way delay probes before accepting relayed mesh time as authoritative.

As of the current Espressif Rust docs, `esp-hal` is the bare-metal `no_std` HAL. ESP-NOW is in `esp-radio`; for ESP32-S3 it is still exposed through `unstable` feature gates and the radio runtime may need `esp-alloc` plus `esp-rtos`. That is acceptable as long as allocator/runtime use stays out of the audio render path.

## Audio Direction

Audio should be I2S DMA into an external DAC. The render path should:

- fill fixed-size DMA buffers
- allocate nothing
- avoid blocking locks
- read only atomic/shared scalar state or precomputed local state
- convert musical ticks into note events before or during buffer fill

For prototyping, `lofi-core::synth` renders into a caller-provided sample slice. Firmware can use the same shape for I2S DMA buffers.

`lofi-core::groove` retains a no-sample reference path. The production music
engine is the symbolic composer in `lofi-core::music::score`: every note is
derived as data from `(seed, roster, mesh tick)` and voiced through root-tagged
mu-law one-shots decoded from flash. The prior loop-scene engine remains
selectable for A/B listening studies. Both paths are allocation-free, and the
symbolic score is dumpable as JSONL for property testing without listening
(see [Symbolic Music](docs/SYMBOLIC_MUSIC.md)).

## Hard Truths

No math can make one-way wireless broadcast timestamps perfectly synchronized if packet latency is unknown and variable. For musical sync, the practical solution is:

- small packets
- future scheduled beat epochs instead of "play now"
- continuous drift correction
- delay/uncertainty estimation
- rejecting low-quality relays
- measuring the actual hardware path with a scope or logic analyzer once boards exist
