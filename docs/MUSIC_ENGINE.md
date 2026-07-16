# Music Engine

## Goal

Generate infinite evolving lo-fi grooves from deterministic state, without requiring streamed notes or a cloud model.

The core idea is:

```text
musical output = f(shared state, role, tick, local taste)
```

Shared state keeps devices coherent. Role and local parameters keep them distinct.

## State

The minimum shared state:

- transport: playing, BPM, song zero, ticks per beat
- groove mode: dusty tape, jazz-hop, ambient study, drum-only, etc.
- seed
- section
- density
- swing
- variation
- role map
- scheduled events

## Groove Modes

Groove modes are modular engines. In `lofi-core`, `mode::GrooveModeEngine` is the current extension point:

- `DustyTape`: current procedural demo
- `JazzHop`: future brushed drums, upright-ish bass, extended chords
- `AmbientStudy`: sparse drums, pads, texture
- `DrumOnly`: percussion-focused utility role
- sample-backed modes: static mu-law one-shots decoded directly from flash

## Embedded Samples

The first hybrid engine replaces procedural kick, snare, and hat voices with a
small CC0 acoustic bank. Source FLAC is converted offline to mono 22.05 kHz
G.711 mu-law. Runtime playback is stateless, constant-time, allocation-free,
and indexed from mesh note age, so a timing correction cannot desynchronize a
sample cursor. Bass, keys, pads, and melody remain procedural for now.

Modes must be deterministic and allocation-free in the audio path.

## Infinite Evolution

Programmatic infinite lo-fi is plausible if generation is structured at multiple timescales:

- per step: drum hits, arp notes, ghost notes
- per bar: fills, chord inversions, bass movement
- per phrase: call/response, density shifts, melody motifs
- per section: intro/groove/drop/breakdown
- per generation: seed and progression refresh

GPT-like infinite music is not needed on-device. The useful embedded approximation is a set of deterministic phrase grammars plus stochastic variation from shared seeds.

## Call/Response

Local actions happen now on the touched device. The mesh schedules responses for other roles in the future:

```text
user triggers call at tick 1024
device plays local phrase immediately
mesh schedules response at tick 1024 + 4 bars
other devices derive response material from call_id + seed + role
```

This creates the feeling of conversation without requiring low-latency network audio.
