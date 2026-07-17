# Hardware Portability

## First Target

The first physical target is ESP32-S3 N16R8 with:

- external I2S DAC
- small LCD
- start/stop button
- speaker path
- ESP-NOW radio

The current development content pack is 5.95 MiB. N16R8 is therefore the
minimum useful module, not a generous target. A production partition table
should put the catalogue in one read-only data partition and keep firmware in
separate OTA slots, so firmware updates do not duplicate the audio bytes.
N32R8 is preferable if multiple installed packs or rollback of pack updates is
required. An 8 MiB flash module cannot hold this catalogue plus the application.

## Crate Shape

The firmware should be split into hardware-independent and board-specific layers:

```text
lofi-core                 no_std timing, protocol, transport, music
lofi-firmware            no_std app state machine and tasks
lofi-board-esp32s3-dev    ESP32-S3 pins, DAC, display, buttons
future board crate        C6, custom PCB, alternate DAC/display
```

`lofi-core` should never depend on `esp-hal`.

All deterministic selectors take their modulus in the fixed-width hash type
before converting to `usize`. This is regression-tested because native hosts
use 64-bit indices while WASM and ESP32 use 32-bit indices; casting first would
make the same seed choose different arrangements on the simulator and device.

## Driver Traits

The firmware app should talk to narrow traits:

- `AudioOut`: submit/fill DMA audio buffers
- `Display`: draw the small device status view
- `Buttons`: start/stop and future controls
- `Radio`: send/receive protobuf envelopes
- `Clock`: monotonic microsecond local time
- `Storage`: persisted settings plus a memory-mapped read-only catalogue slice

The first implementation can keep these traits local to the firmware crate. The
catalogue parser already borrows `&'static [u8]` without allocation, so firmware
can point it at memory-mapped flash rather than copying samples into RAM.

## UI Hardware

The LCD only needs to show operational state at first:

- playing/stopped
- BPM
- primary role and compact role mask
- section
- sync quality
- peer count
- call/response pending indicator

Keep the display model tiny and deterministic. The simulator UI should render the same display state so desktop and hardware stay aligned.
