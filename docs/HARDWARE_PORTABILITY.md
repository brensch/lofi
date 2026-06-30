# Hardware Portability

## First Target

The first physical target is ESP32-S3 N16R8 with:

- external I2S DAC
- small LCD
- start/stop button
- speaker path
- ESP-NOW radio

## Crate Shape

The firmware should be split into hardware-independent and board-specific layers:

```text
lofi-core                 no_std timing, protocol, transport, music
lofi-firmware            no_std app state machine and tasks
lofi-board-esp32s3-dev    ESP32-S3 pins, DAC, display, buttons
future board crate        C6, custom PCB, alternate DAC/display
```

`lofi-core` should never depend on `esp-hal`.

## Driver Traits

The firmware app should talk to narrow traits:

- `AudioOut`: submit/fill DMA audio buffers
- `Display`: draw the small device status view
- `Buttons`: start/stop and future controls
- `Radio`: send/receive protobuf envelopes
- `Clock`: monotonic microsecond local time
- `Storage`: optional persisted device id/settings

The first implementation can keep these traits local to the firmware crate. Move them into `lofi-core` only if they become useful to the simulator or tests.

## UI Hardware

The LCD only needs to show operational state at first:

- playing/stopped
- BPM
- role
- section
- sync quality
- peer count
- call/response pending indicator

Keep the display model tiny and deterministic. The simulator UI should render the same display state so desktop and hardware stay aligned.
