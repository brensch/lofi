# Code Quality Standard

This repo should stay easy to reason about on embedded hardware and in the simulator.

## File Size

- Keep source files under 300 lines.
- If a file approaches 250 lines, consider splitting by responsibility before adding more behavior.
- Exceptions require a short comment in the plan or PR explaining why a split would make the code worse.

Current audit exceptions:

- `crates/lofi-core/src/groove.rs` is at the limit and should be split before adding more synthesis modes.
- `crates/lofi-core/src/music/kit.rs` is mostly declarative presets, but should be split into preset families before the catalogue grows.
- `crates/lofi-core/src/music/beat.rs` and `music/arrangement.rs` should be split by rendering, patterns, features, and naming before adding behavior.
- `crates/lofi-core/src/mesh/engine.rs`, `crates/lofi-app/src/device.rs`, and `crates/lofi-sim/src/sim/mod.rs` are at or near the limit; split by responsibility before growing them.

## Modularity

- `lofi-core` stays hardware-independent and `no_std`.
- Board-specific code must not leak into `lofi-core`.
- Browser controls, the AudioWorklet radio substrate, firmware drivers, and protocol generation should remain separate modules.
- Prefer narrow traits for hardware boundaries: audio output, display, buttons, radio, clock, and storage.

## Testing

- Every shared timing, protocol, transport, event, and generation primitive needs focused tests.
- Add regression tests for any bug found by simulation or hardware.
- Clock-sync changes must include drift, packet loss, and convergence tests.
- Transport changes must test continuity and no accidental tick regression.
- Protocol changes must pass `buf lint` and include encode/decode compatibility tests where applicable.

## Realtime Discipline

- The audio path must allocate nothing.
- The audio path must not block on locks, network, storage, display, logging, or UI.
- Fixed-size buffers and bounded queues are preferred.
- Any expensive generation should happen outside the hard realtime render path or be bounded per sample/block.

## Embedded Constraints

- Assume limited CPU, RAM, flash, and unstable wireless timing.
- Prefer deterministic integer or fixed-table math in `lofi-core`.
- Use `f32` only when it is measured to fit the audio budget.
- Keep packets small even though ESP-NOW v2 allows larger payloads.

## Protocol Discipline

- Protobuf in `proto/` is the semantic source of truth.
- Embedded wire encoding may be more compact, but it must preserve protobuf field meanings and versioning.
- Network actions should be absolute, idempotent, and scheduled in the future.
- Avoid "do this now" messages for musical coordination.

## Documentation

- Keep architecture docs current when changing sync, transport, protocol, hardware boundaries, or simulator behavior.
- Capture tradeoffs explicitly, especially where simulation differs from production firmware.
- Document known simplifications instead of letting prototypes look more finished than they are.

## Review Checklist

- Does this keep `lofi-core` `no_std`?
- Does this preserve audio realtime safety?
- Are files short and responsibility-focused?
- Are tests added at the same level as the risk?
- Does the simulator still exercise the behavior being changed?
- Are docs and protobuf schema updated when behavior or messages change?
