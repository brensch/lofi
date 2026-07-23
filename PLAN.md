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
- `lofi-app` is the shared `no_std` device runtime used by hardware and simulation surfaces.
- `lofi-sim` is a host simulator that renders WAVs and simulates drifting clocks, packet loss, staged mesh connectivity, and procedural groove audio.
- `lofi-web` and `apps/mesh-lab` form the realtime browser lab. Every module is an independent WASM device connected through a mocked ESP-NOW packet substrate.
- `proto/lofi/v1/lofi.proto` defines the semantic communication schema.
- `buf.yaml` validates the protobuf schema.
- `docs/` captures product, mesh sync, hardware portability, music engine, and simulator UI direction.

Current demo:

```sh
cargo run -p lofi-sim -- --nodes 8 --duration-ms 18000 --sync-start-ms 2500 --group-join-ms 8000 --wav target/lofi-two-clusters-merge.wav
```

This creates two isolated four-device clusters, one on the left channel and one on the right. Each cluster syncs internally first, then cross-cluster links open and the two groups converge into one mesh.

## Important Clarification

The mesh has no fixed leader or infrastructure. Nodes discover an emergent lowest-live-id root, measure upstream peers with NTP-style probes, and slew toward weighted references. The fixed-capacity peer table, outlier rejection, monotonic clock discipline, root failover, and split/merge handling are implemented. Remaining work is hardware integration and tuning against measured ESP-NOW latency.

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
  host simulation kernel and WAV export

lofi-web
  no_std ABI for one independently instantiated device

apps/mesh-lab
  Vite/React control panel, AudioWorklet ESP-NOW substrate, and monitor mix
```

Rules:

- `lofi-core` must not depend on `esp-hal`.
- Hardware drivers should sit behind narrow traits.
- Network and UI may be late or lossy; audio must never wait for them.
- Musical events should be scheduled on future ticks.
- Device behavior should derive from shared state, role, tick, and deterministic seeds.

## Mesh Sync Status

Implemented in `lofi-core::mesh` and exercised by the simulator:

NTP-style exchange:

```text
A sends probe at A:t1
B receives at B:t2
B sends response at B:t3
A receives at A:t4

offset = ((t2 - t1) + (t3 - t4)) / 2
delay  = ((t4 - t1) - (t3 - t2)) / 2
```

- fixed-capacity tracking of peer delay, jitter, age, measured error, topology, and sample count
- rejection or down-weighting of high-delay samples
- emergent lowest-live-id root with failover
- weighted discipline from upstream peers
- monotonic scheduling time
- split and merge recovery
- tests for drift, loss, convergence, failover, offset steps, and no backward time

Firmware integration remains:

- timestamp ESP-NOW RX/TX close to the radio operation
- encode/decode production frames
- deliver messages to `SyncEngine`
- tune gains and rejection thresholds from physical measurements

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

## Music Engine Status

The goal is infinite evolving lo-fi, not fixed loops.

Implemented in `lofi-core::music`:

- deterministic multi-timescale arrangement features
- rotating role assignment across the live mesh roster
- shared chord progressions and compatible bass, keys, lead, drums, and texture
- data-driven patches and curated kits
- per-device display role and arrangement codenames

Still planned: call/response input, late-join state snapshots, more modes, and physical audio tuning.

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

`GrooveModeEngine` remains the legacy extension hook. New modes should build on `music` as separate modules; do not expand `groove.rs` further.

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
5. ESP-NOW send/receive with the production fixed-frame encoding.
6. Connect radio timestamps and frames to the existing mesh sync state machine.
7. Scheduled transport/groove events.
8. Call/response.

## Simulator UI Status

Implemented:

- add/remove virtual devices
- set clock drift/offset per device
- start/stop sync
- start/stop transport
- listen in real time
- pan/solo devices
- show each device LCD

The CLI retains deterministic WAV export and staged cluster merge. UI work still planned:

- interactive split and merge groups
- trigger calls and inspect scheduled responses
- show mesh links, packet loss, jitter, and sync quality
- render WAVs/stems from the UI

## Symbolic Engine Status

The audible default is now the symbolic composer (`lofi-core::music::score`,
see [Symbolic Music](docs/SYMBOLIC_MUSIC.md)): notes are data derived from
`(seed, roster, mesh tick)`, voiced through the pack's root-tagged one-shots.
Iteration runs without ears via `score_dump` + `tools/listen-qa/symbolic_gates.py`
+ `scorecard.py` + `candidates.py`; the `/judge` deck A/Bs both engines blind.
Follow-ups queued from the first instrumented sweeps:

- Measure symbolic-engine CPU on target hardware; the per-block lane lookback
  and the one-shot kick autocorrelation at scene resolve need ESP32 numbers.
- Widen the shipped pack's keys-note roots (only 10 elements today) so shell
  voicings gain register freedom.
- Reintroduce a key-matched texture bed for sparse phrases.
- Train the taste model from engine-tagged judgement records.

## Immediate Engineering Backlog

1. Add the ESP32-S3 firmware and board crates.
2. Prove I2S DMA audio, SSD1306 display, button input, and ESP-NOW on target hardware.
3. Add transport/groove state snapshots for late joiners.
4. Add explicit group pairing so nearby customer swarms do not merge accidentally.
5. Define the compact production wire encoding and compatibility tests.
6. Split the oversized music preset, arrangement, and beat modules before expanding them.
7. Add a production test mode for speaker, display, button, radio, and device identity.
8. Measure end-to-end sync and audio latency on physical boards.
9. Add reproducible firmware artifacts and ESP32-S3 cross-builds to CI.
10. Complete the launch gates in [the commercialization roadmap](docs/COMMERCIALIZATION_ROADMAP.md).

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
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --no-default-features -- -D warnings
cargo test --workspace --all-targets --no-default-features
buf lint
```
