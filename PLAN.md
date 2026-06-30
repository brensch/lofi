# Lofi Mesh Groovebox Plan

## Product Target

Build a family of small lo-fi grooveboxes that generate music locally and coordinate over ESP-NOW. The first hardware target is ESP32-S3 N16R8 with:

- start/stop button
- small cheap LCD
- built-in speaker path
- external I2S DAC
- no_std Rust firmware
- ESP-NOW mesh sync

The system should feel immediate locally, while other boxes respond on a shared musical timeline. Example: one device performs a call now, then other devices answer exactly four bars later.

## Current Repo State

- `lofi-core` is `no_std` and contains timing, clock discipline, transport, scheduled events, protocol frame primitives, procedural groove generation, and groove-mode extension hooks.
- `lofi-sim` is a host simulator that renders WAVs and simulates drifting clocks, packet loss, staged mesh connectivity, and procedural groove audio.
- `proto/lofi/v1/lofi.proto` defines the semantic communication schema.
- `buf.yaml` validates the protobuf schema.
- `docs/` captures product, mesh sync, hardware portability, music engine, and simulator UI direction.

Current demo:

```sh
cargo run -p lofi-sim -- --nodes 8 --duration-ms 18000 --sync-start-ms 2500 --group-join-ms 8000 --wav target/lofi-two-clusters-merge.wav
```

This creates two isolated four-device clusters, one on the left channel and one on the right. Each cluster syncs internally first, then cross-cluster links open and the two groups converge into one mesh.

## Important Clarification

The current simulator is mesh-style, not single-leader. Every reachable node can broadcast its mesh-time estimate and every other reachable node can slew toward it.

It is still a simplified mesh. The production goal is a stronger NTP-style pairwise measurement system with weighted averaging, outlier rejection, uncertainty, and split/merge epochs.

## Architecture

Layering:

```text
lofi-core
  no_std sync math, transport, events, protocol semantics, music generation

lofi-firmware
  no_std app state machine, Embassy tasks, audio scheduling

board crates
  ESP32-S3 pins, I2S DAC, LCD, buttons, speaker amp, radio setup

lofi-sim
  host simulation kernel, WAV export, future realtime audio

lofi-ui
  future desktop UI over the simulator
```

Rules:

- `lofi-core` must not depend on `esp-hal`.
- Hardware drivers should sit behind narrow traits.
- Network and UI may be late or lossy; audio must never wait for them.
- Musical events should be scheduled on future ticks.
- Device behavior should derive from shared state, role, tick, and deterministic seeds.

## Mesh Sync Plan

### Phase 1: Current Baseline

Keep the current all-to-all mesh beacon simulator as an audible baseline. It is useful for demonstrating staged convergence and split/merge behavior.

### Phase 2: Pairwise Probe Model

Add NTP-style exchanges:

```text
A sends probe at A:t1
B receives at B:t2
B sends response at B:t3
A receives at A:t4

offset = ((t2 - t1) + (t3 - t4)) / 2
delay  = ((t4 - t1) - (t3 - t2)) / 2
```

Track per-peer:

- offset estimate
- delay
- jitter
- age
- packet loss
- confidence score

Reject or down-weight high-delay samples.

### Phase 3: Weighted Mesh Consensus

Each node computes a trimmed weighted average from peer estimates:

- lower delay gets more weight
- lower jitter gets more weight
- stale peers decay
- outliers are rejected
- corrections are slewed, not hard-set

Add tests for:

- convergence under drift
- convergence under loss
- no backwards mesh time
- split clusters
- cluster merge
- peer disappearance

### Phase 4: Firmware Integration

Move the mesh state machine into `lofi-core` or a no_std sync module. ESP-NOW tasks only timestamp, encode/decode, and deliver messages to the state machine.

## Protocol Plan

Protobuf is the semantic source of truth:

- `Envelope`
- `Hello`
- `TimeProbeRequest`
- `TimeProbeResponse`
- `TimeBeacon`
- `TransportState`
- `ScheduledEvent`
- `GrooveState`
- `RoleMap`

Decision still open: exact embedded wire encoding.

Options:

- use protobuf-compatible no_std encoding if practical
- generate compact postcard-like packets from the same semantics
- keep protobuf for tooling/docs and write a small fixed encoder for hot-path ESP-NOW frames

Constraint: ESP-NOW sync packets should stay tiny even though v2 supports larger payloads.

## Music Engine Plan

The goal is infinite evolving lo-fi, not fixed loops.

Shared state:

- transport
- groove mode
- seed
- section
- density
- swing
- variation
- role map
- scheduled events

Generation timescales:

- step: hits, arps, ghost notes
- bar: fills, inversions, bass movement
- phrase: motifs, call/response, density
- section: intro, groove, drop, breakdown
- generation: seed/progression refresh

`GrooveModeEngine` is the current extension hook. Add modes as separate modules before expanding `groove.rs` further.

Near-term modes:

- DustyTape: current procedural demo
- JazzHop: extended chords, softer drums, walking bass fragments
- AmbientStudy: sparse drums, pads, texture
- DrumOnly: utility/percussion role
- SampleBacked: static PCM one-shots

## Call/Response Plan

Local device responds immediately to user input. It also schedules a future response:

```text
call at tick C
response at next_bar(C) + 4 bars
action = CallResponse(call_id, source_node, phrase ids)
```

Other devices derive their response from:

- shared seed
- call id
- role
- section
- groove mode

No streamed notes are required.

## Hardware Plan

First board:

- ESP32-S3 N16R8
- external I2S DAC
- small LCD
- start/stop button
- speaker amp/path

Firmware milestones:

1. Board crate with clock, button, LCD, DAC, and radio setup.
2. I2S DMA audio with a hardcoded procedural groove.
3. Start/stop transport button.
4. LCD status view: play state, BPM, role, section, peer count, sync quality.
5. ESP-NOW send/receive with protobuf envelope subset.
6. Mesh sync state machine.
7. Scheduled transport/groove events.
8. Call/response.

## Simulator UI Plan

The simulator needs a real UI after the core mesh model improves.

Target features:

- add/remove virtual devices
- set clock drift/offset per device
- start/stop sync
- split and merge groups
- listen in real time
- pan/solo devices
- render WAVs
- trigger calls
- inspect scheduled responses
- show each device LCD
- show mesh links and sync quality

Suggested stack:

- Rust simulation kernel
- `egui`/`eframe` for UI
- `cpal` for realtime audio
- existing WAV export retained for regression artifacts

## Immediate Engineering Backlog

1. Split `lofi-sim/src/main.rs` into smaller modules.
2. Split `lofi-core/src/groove.rs` before adding more synthesis.
3. Add mesh-state tests separate from WAV rendering.
4. Implement pairwise probe simulation.
5. Implement weighted peer table and consensus.
6. Add no-backwards-time tests.
7. Add transport/groove state snapshots for late joiners.
8. Add display-state model shared by firmware and simulator.
9. Add firmware skeleton for ESP32-S3.
10. Add I2S DMA audio proof on hardware.

## Quality Bar

Follow [CODE_QUALITY.md](CODE_QUALITY.md). In particular:

- files under 300 lines
- no_std core
- modular hardware boundaries
- no allocation or blocking in audio
- focused tests for shared logic
- docs/proto updated with behavior changes

## Current Verification Commands

```sh
cargo fmt --check
cargo test
cargo check -p lofi-core --no-default-features
buf lint
```
