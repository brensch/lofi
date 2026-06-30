# Lofi

Foundation for an ESP32-S3 lo-fi box that keeps multiple devices in time over ESP-NOW and can be prototyped on a desktop first.

## Current Shape

- `crates/lofi-core`: `no_std` timing, sync, packet, transport, scheduled events, deterministic role-based generation, modular groove-mode hooks, tiny synth helpers, and a procedural lo-fi groove engine.
- `crates/lofi-app`: `no_std` **device runtime** — the code that runs identically on hardware and in the simulator. A `Device` owns one box's clock, transport, groove, event queue, play state, and LCD model; it renders mono audio and a 128x64 SSD1306 framebuffer from shared mesh state. Neither the firmware nor the simulator re-implements the musical behavior.
- `crates/lofi-sim`: host simulation kernel (library + WAV bin) that drives real `Device`s through a simulated ESP-NOW network (drift, loss, latency), a scheduled drop, and a stereo monitor mix. Pan/solo/volume are listener-side controls; a real box just drives one mono speaker.
- `crates/lofi-ui`: `eframe`/`egui` desktop lab — per-device dot-matrix LCD panels, start/stop, mute/solo, and pan/volume/drift/offset sliders, with realtime audio via `cpal`. Reuses the `lofi-sim` kernel so the lab matches firmware.
- `proto/lofi/v1/lofi.proto`: protobuf source of truth for mesh, transport, scheduled event, and groove-state messages.
- `docs/`: product, mesh sync, hardware portability, music engine, and simulator UI notes.

### Realtime simulator UI

```sh
# Linux build needs ALSA dev headers: sudo apt-get install libasound2-dev
cargo run -p lofi-ui
```

Four boxes appear, two panned left and two right, mesh sync on. Each shows its own LCD (BPM, section, peer count, sync error, bar position). Add/remove devices, toggle start/stop, solo/mute, and drag each box's clock drift and offset to watch — and hear — the mesh pull back into time. Build headless (no audio) with `--no-default-features`.

#### WSL / WSLg notes

The app auto-detects WSL and steers to the X11/XWayland backend with software GL
(WSLg's Wayland path crashes winit, and its hardware GL/zink is unstable). For
the window you need the X11 keyboard library once:

```sh
sudo apt-get install libxkbcommon-x11-0
```

Audio just works: WSL has no ALSA sound card, so `cpal` finds no device, and the
app falls back to writing straight to WSLg's PulseAudio server via the
`libpulse-simple` already on the system (no install, no ALSA bridge). You'll see
`lofi-ui: audio via PulseAudio` on startup. If even that is unavailable it runs
silently and the LCDs/sync still animate. On native Linux/macOS/Windows, `cpal`
drives the default device directly.

### Batch WAV render

```sh
cargo run -p lofi-sim -- --nodes 8 --duration-ms 18000 --sync-start-ms 2500 --group-join-ms 8000 --wav target/lofi-two-clusters-merge.wav
```

Open `target/lofi-two-clusters-merge.wav` with headphones. Four virtual devices start on the left, four start on the right. Each side syncs internally from 2.5 seconds, then the two clusters can hear each other from 8 seconds and converge into one mesh.

The current groove is generated without samples: kick, snare, hats, bass, harmony, wobble, and bitcrush are all math in `no_std` code.

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
- [Simulator UI](docs/SIMULATOR_UI.md)
