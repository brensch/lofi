# Product Notes

## Device

Each unit is a small standalone lo-fi groovebox:

- ESP32-S3 N16R8 as the first hardware target
- no_std Rust firmware
- start/stop button
- small cheap LCD panel
- built-in speaker
- external I2S DAC
- local procedural audio generation
- ESP-NOW mesh sync with other units

The ESP32-S3 firmware crate should be pluggable enough that future boards can swap chip, display, DAC, buttons, and speaker driver without rewriting the musical core.

## Musical Direction

The system should generate continually evolving lo-fi grooves, not fixed loops. Devices should share enough deterministic state to stay coherent:

- transport: playing, BPM, song zero, ticks per beat
- groove state: seed, mode, section, density, swing, variation
- role map: which device is drums, bass, chords, melody, texture, FX
- future scheduled events: drops, section changes, seed changes, call/response events

Dense real-time musical choices should be local. Network messages should mostly schedule future intentions.

## Interaction Direction

The local device should feel immediate. If the user presses start/stop or triggers a call, that device responds locally now. Other devices should receive a scheduled response on the shared timeline, for example four bars later. This keeps the box tactile while still making the group feel coordinated.

Actions should be absolute and idempotent:

- good: "set section = DROP at tick 3072"
- good: "respond to call 42 at tick 4608"
- bad: "advance section now"

This lets late or lossy devices recover from the next state packet.

## Simulator Direction

The simulator should become the main design lab:

- add/remove virtual devices
- set per-device clock drift and offset
- start/stop sync
- listen in real time
- inspect each device's LCD
- inspect mesh links, packet loss, time error, and scheduled events
- render WAVs for repeatable tests

The current CLI/WAV simulator is only the audio and timing kernel. A UI should sit on top of the same simulation state instead of becoming a separate implementation.
