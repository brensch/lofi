# Music Engine

## Current Contract

The runtime plays coherent sampled scenes on a shared mesh timeline:

```text
audible output = f(scene seed, assigned role, mesh time)
```

A scene contains stems harvested from one source performance. The catalogue
resolver will not combine a drum loop from one source with bass or harmony from
another. This is the central musical invariant.

## Scene Selection

`PackedCatalog::loop_scene` selects a melody or harmony anchor from the shared
seed, then resolves matching source hashes for:

- four one-bar drum loops with phrase phases `0..3`;
- transient-aligned kick, snare, and hat one-shots;
- one four-bar bass loop;
- an optional four-bar melody loop;
- an optional four-bar harmony loop;
- one four-bar texture loop.

The lookup runs only when a device starts, changes seed, or changes catalogue.
The selected `LoopScene` is copied into the device. There is no catalogue scan
inside the per-sample render loop.

## Distributed Roles

The fixed role order is `Pulse`, `Pocket`, `Low`, `Color`, and `Motif`. Roles are
dealt round-robin across the current mesh roster:

- `Pulse`: kick on beats one and three;
- `Pocket`: backbeats plus swung eighth-note hats;
- `Low`: bass stem;
- `Color`: harmony plus a restrained texture stem;
- `Motif`: melody, or a quiet harmony fallback.

A lone module plays every role. Additional modules take distinct roles while
remaining sample-aligned. Browser panning is listener-side monitoring only and
does not exist in firmware.

## Timing

Every hit and loop position is derived statelessly from `Transport` and mesh
time. One bar is 384 ticks at 96 ticks per beat. Longer stems restart on the
same four-bar boundary. Playback rate follows transport BPM relative to the
source BPM, so a tempo change cannot create an independent cursor on one box.

Kick and snare use a fixed one-bar grid. Hats alone receive a bounded 12% offbeat
delay; no random timing or generated drum-loop performance enters the audible
path. Before slicing, the offline forge measures the drum performance's beat
period and phase, then applies the same small resample and downbeat shift to
every stem from that source. Drum one-shots end before the next detected source
transient, with shipped durations bounded to 200 ms for hats, 400 ms for snares,
and 450 ms for kicks. The packer also fades loop edges, and catalogue tests
require every shipped seam to be near zero.

## Audio Path

For each DMA/Web Audio block, `Device::render_audio`:

1. disciplines local hardware time to mesh time once;
2. resolves the roster and this device's roles;
3. reads the already selected `LoopScene`;
4. renders only those roles from flash-resident G.711 mu-law samples;
5. applies bounded saturation, vinyl air, and the kit low-pass;
6. converts to signed 16-bit PCM with fixed headroom.

The core is `no_std`, allocation-free, lock-free, and contains no filesystem,
network audio, decoder state, oscillator, or workstation ML dependency.

## Content Policy

The catalogue still includes aligned one-shots and root metadata for future
work, but the previous runtime that assembled chords from unrelated harvested
fragments was rejected in listening review. A new arrangement may activate
one-shots only when it preserves source/key compatibility and passes the exact
browser-path render gate in [Listen QA](LISTEN_QA.md).

## Current Limitations

- Only three source scenes currently have enough aligned tonal stems to play.
- Four-bar repetition is deliberate until more reviewed source scenes exist.
- Arrangement codenames and feature cards currently affect display state and
  tone character, not stem selection within a scene.
- Commercial release still requires human listening and rights approval for
  every shipped source, regardless of automated scores.
