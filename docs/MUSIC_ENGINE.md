# Music Engine (loop scenes)

> **Status**: the loop engine documented here is no longer the product
> default. The audible path is the symbolic composer described in
> [Symbolic Music](SYMBOLIC_MUSIC.md); this engine remains selectable
> (`Engine::Loops`, `--engine loops`, judge-deck profiles) for blinded A/B
> listening studies against it.

## Contract

The loop engine plays coherent sampled scenes on a shared mesh timeline:

```text
audible output = f(scene seed, assigned role plan, mesh time)
```

A scene contains stems harvested from one source performance. The catalogue
resolver will not combine a drum loop from one source with bass or harmony from
another. This is the central musical invariant.

## Scene Selection

`PackedCatalog::loop_scene` selects a reviewed melody-backed anchor from the
shared session seed, then resolves matching source hashes for:

- four one-bar drum loops with phrase phases `0..3`;
- transient-aligned kick, snare, and hat one-shots;
- one four-bar bass loop;
- an optional four-bar melody loop;
- an optional four-bar harmony loop;
- one four-bar texture loop.

Harmony-only material remains in the catalogue as a fallback for future packs,
but it is not automatically selected while melody-backed scenes are available.
The lookup runs only when a device starts or changes catalogue. The selected
`LoopScene` is copied into the device. There is no catalogue scan inside the
per-sample render loop.

## Organic Evolution

The experience has no user-facing tape or music-set selector. `Arrangement`
derives a new bounded parameter set at every shared eight-bar boundary. Those
parameters change drum density and fills, stem prominence, texture balance, and
motif activity inside a four-phrase energy arc. The result evolves continually
without replacing the source-coherent scene or shuffling unrelated samples.

Each phrase also has exactly one spotlight role, derived from its newest feature
card. The spotlight is the only lane allowed a foreground flourish. `Low`
spotlights sequence a bounded final-bar pickup from the catalogue's root-tagged
bass one-shots; pitch is conformed to the active scene key, and the tail fades
before the shared phrase boundary. Other lanes remain in their supporting loop
and pattern roles, avoiding simultaneous competing fills.

All modules resolve the same phrase from mesh time, so structural changes land
on the same sample-aligned boundary. Continuous stem levels use a smoothstep
crossfade over the first beat of the new phrase. Discrete drum-pattern changes
land directly on the boundary. The browser exposes the same transport boundary
as a countdown rather than exposing the internal seed.

## Distributed Roles

The fixed role order is `Pulse`, `Pocket`, `Low`, `Color`, and `Motif`:

- `Pulse`: kick on beats one and three;
- `Pocket`: backbeats plus swung eighth-note hats;
- `Low`: bass stem;
- `Color`: harmony plus a restrained texture stem;
- `Motif`: melody, or a quiet harmony fallback.

A lone module plays every role. Two modules split all five roles 3/2. From three
modules upward, each module receives exactly one rhythm role (`Pulse` or
`Pocket`) and one tonal role (`Low`, `Color`, or `Motif`). Every five-role layer
remains covered, no module renders the complete mix, and no module becomes
silent when the roster grows to ten.

Each role plan has a fixed makeup trim before the bounded saturator. Sparse
pairings receive more drive than kick/bass pairings so every physical module is
useful as a local mono speaker. This adds no state or allocation. Browser
panning is listener-side monitoring only and does not exist in firmware.

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
5. applies the bass-role low-pass, bounded saturation, vinyl air, and the kit
   low-pass;
6. converts to signed 16-bit PCM with fixed headroom.

The core is `no_std`, allocation-free, lock-free, and contains no filesystem,
network audio, decoder state, oscillator, or workstation ML dependency.

## Content Policy

The catalogue includes aligned one-shots and root metadata. Bass spotlights use
one sampled voice at a time and repitch it to the active scene's major or minor
pentatonic scale. The previous runtime that simultaneously assembled chords,
bass, and melody from unrelated fragments remains rejected. Further one-shot
arrangements must preserve source/key compatibility and pass the exact
browser-path render gate in [Listen QA](LISTEN_QA.md).

## Current Limitations

- Three melody-backed source scenes currently meet the automatic-selection gate,
  although two are perceptually similar.
- Four-bar source loops repeat inside an evolving eight-bar arrangement phrase.
- Commercial release still requires human listening and rights approval for
  every shipped source, regardless of automated scores.
