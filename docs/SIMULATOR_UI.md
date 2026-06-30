# Simulator UI Plan

## Goal

The simulator should become a real desktop lab for the groovebox mesh.

Required UI features:

- add/remove virtual devices
- adjust clock offset and drift per device
- start/stop sync
- start/stop transport
- listen in real time
- view each device's LCD
- inspect peer links, packet loss, jitter, and sync quality
- trigger local calls and watch scheduled responses propagate

## Suggested Shape

Keep the simulation kernel in Rust and expose it to a UI:

```text
lofi-core       no_std models
lofi-sim        simulation kernel, realtime audio, state snapshots
lofi-ui         desktop UI
```

Good first UI stack options:

- `egui`/`eframe`: fastest native Rust path for controls and LCD-like panels
- `cpal`: realtime desktop audio output
- optional WAV export remains for deterministic regression artifacts

## Device Panel

Each virtual device panel should render:

- small LCD preview
- play state
- role
- local clock error
- mesh quality
- peer count
- current section
- pending call/response count

The LCD preview should use the same display-state struct the firmware uses.

## Audio Routing

The UI should allow:

- stereo mix
- solo a device
- pan devices
- render stems
- exaggerate sync error for debugging

The current CLI renderer is still useful for repeatable tests. The UI should call the same rendering code.
