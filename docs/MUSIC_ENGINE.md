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

The sample-only engine stores mono 22.05 kHz G.711 mu-law audio in one indexed
binary pack. Runtime playback is stateless, constant-time, allocation-free, and
indexed from mesh note age, so a timing correction cannot desynchronize a
sample cursor. Bass, keys, melody, drums, and texture are all sampled. Pitched
roles change sample playback rate through linear interpolation; symbolic notes
never invoke an oscillator voice.

Modes must be deterministic and allocation-free in the audio path.

## Harvested Content

AI references are reduced offline into reviewed one-shots, compatible loops,
four-bar trigger signatures, velocity contours, role timing, spectral targets,
and tape character. A seed chooses sampled variants deterministically. The pack
provides eight constant-time variants per target pitch, so larger harvests add
real timbral variety without a linear scan in the audio callback.

Melodies are authored against the progression's key and mode. They no longer
select a new chord tone independently at each onset, which preserves their
identity across harmony changes. Chords use nearest non-crossing inversions, and
secondary key attacks are single-note answers instead of repeated full voicings.

See [AI-Assisted Content Pipeline](AI_CONTENT_PIPELINE.md) for provenance,
analysis, curation, and commercial acceptance gates.

## Infinite Evolution

Programmatic infinite lo-fi is plausible if generation is structured at multiple timescales:

- per step: drum hits, arp notes, ghost notes
- per bar: fills, chord inversions, bass movement
- per phrase: call/response, density shifts, melody motifs
- per section: intro/groove/drop/breakdown
- per generation: seed and progression refresh

GPT-like infinite music is not needed on-device. The useful embedded approximation is a set of deterministic phrase grammars plus stochastic variation from shared seeds.

## Composition Constraints

The generator treats repetition as the foundation and variation as a limited
resource. These constraints are intentional:

- Kick, snare, hat, and bass events form one stable four-bar pocket. The full
  pocket repeats until a feature deliberately changes its density or adds a
  restrained turnaround.
- Backbeats establish the meter. Syncopation comes from a few kick, bass, and
  chord placements rather than every role competing off-grid.
- An eight-bar phrase can carry at most two feature cards. A feature changes one
  musical idea such as hat density, bass movement, chord color, or a lead entry.
- The lead uses a sparse four-bar call and response with seven to twelve attacks.
  It remains recognizable across chord changes and can change only when a motif
  variation is selected.
- Normal keys use two attacks per bar, normal bass uses two or three, and denser
  modes remain capped. Extended harmony does not imply dense rhythm.
- Timing is role-specific: the kick stays close to the grid, the snare sits later,
  hats carry the swing, and deterministic humanization remains subtle.
- Pads retrigger every two bars by default so chord releases and silence remain
  audible parts of the arrangement.

These rules follow groove research that finds an inverted-U response to rhythmic
complexity: moderate syncopation tends to feel better than either a rigid grid or
maximal complexity. Repetition supplies the prediction that makes a restrained
violation feel like groove rather than noise.

Research basis:

- [Syncopation creates the sensation of groove in synthesized music](https://pmc.ncbi.nlm.nih.gov/articles/PMC4165312/)
- [The effects of syncopation, body movement and pleasure on groove](https://pmc.ncbi.nlm.nih.gov/articles/PMC3989225/)
- [Groove in drum patterns as a function of syncopation and event density](https://pmc.ncbi.nlm.nih.gov/articles/PMC6025871/)

## Call/Response

Local actions happen now on the touched device. The mesh schedules responses for other roles in the future:

```text
user triggers call at tick 1024
device plays local phrase immediately
mesh schedules response at tick 1024 + 4 bars
other devices derive response material from call_id + seed + role
```

This creates the feeling of conversation without requiring low-latency network audio.
