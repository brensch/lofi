# Procedural Arrangement Spec

## Goal

Make the lo-fi boxes behave like a distributed generative arranger rather than identical loop players.

The system must:

- evolve automatically over time
- choose feature variations procedurally
- let devices take turns deciding the next feature
- keep every generated part compatible with every other part
- assign clearly different musical jobs across devices
- show each device's job and the upcoming arrangement identity on the screen
- give each active feature combination a coined, non-descriptive codename derived from the deterministic state

The core rule is:

```text
audible part = f(seed, mesh roster, phrase, role, mesh time)
```

Every box computes the same arrangement locally. No note stream is sent over the mesh.

## Timescales

### Generation

Generation is the slowest layer. It is keyed by the shared seed and produces long-lived musical material:

- key center
- chord progression template
- base rhythmic palette
- role catalog

Generation changes only when the seed changes. That can later be scheduled as a rare event, but the initial implementation keeps the existing seed lifecycle.

### Arrangement Phrase

An arrangement phrase is 8 bars.

At each phrase boundary, one device in the mesh roster becomes the selector. The selector is chosen by phrase index:

```text
selector = sorted_roster[phrase_index % roster_len]
```

The selector deterministically picks one feature from the catalog:

```text
feature = catalog[hash(seed, phrase_index, selector_id) % catalog_len]
```

This means all devices agree on the result, but different devices have different "taste" because the selector id changes the hash.

### Active Feature Window

The current arrangement is not just the newest feature. It is a sliding window of recent feature picks.

Initial implementation:

- phrase length: 8 bars
- active feature window: last 4 phrases
- newest feature: exposed as `incoming`
- active parameters: base params plus all features in the window applied in phrase order

This creates rotation: old features age out, new features enter, and the track changes without hard section switches.

### Performance

Performance happens per sample from mesh time and transport:

- drums render kick, snare, hats, and ghost notes
- bass renders root/sub/walking movement
- keys render chord comping and extensions
- lead renders sparse motifs
- texture renders pad/crackle/dust

All roles use the same progression and scale source. Rhythm and pitch choices are deterministic, so devices stay synchronized even when they render different roles.

## Roles

Roles are structural jobs, not one-off features. Each job owns one clear part so
the combined mix stays intelligible and the realtime renderer does not synthesize
the same voice repeatedly on neighboring boxes.

Initial role catalog:

- `PULSE`: kick
- `POCKET`: snare, hats, and ghost hits
- `LOW`: bass movement
- `COLOR`: Rhodes and pad texture
- `MOTIF`: short melodic motifs

Assignment is deterministic from the sorted mesh roster:

```text
role_j belongs to device (j % roster_len)
```

Behavior:

- one box plays all roles
- two boxes split rhythm/tonal work by round-robin role ownership
- more boxes spread roles further
- if there are fewer boxes than roles, a box can own multiple roles
- if there are more boxes than roles, only the first role slots are active until later expansion

Each box has a primary role for display: the first role assigned to that device.

## Feature Catalog

Features are composable parameter deltas, not hand-authored clips. They must be safe in any combination and safe with any role subset.

Initial features:

- `DoubleHats`: denser hats
- `SparseHats`: lighter hats
- `OpenHats`: brighter hats
- `Ghosts`: ghost snare notes
- `KickA`: alternate kick pattern
- `KickB`: alternate kick pattern
- `HalfTime`: slower drum backbeat
- `DrumFill`: rotating fill variation
- `SwingHard`: additional swing
- `Walk`: walking bass movement
- `BassSkip`: alternate bass pattern
- `SubBass`: bass octave drop
- `BusyBass`: extra bass hits
- `RichChords`: added chord color
- `KeyStabs`: alternate comping pattern
- `SparseKeys`: reduced comping
- `LeadIn`: enable lead motifs
- `BusyLead`: denser lead motifs
- `NewMotif`: alternate melodic shape
- `Dusty`: more hiss/crackle
- `PadPulse`: alternate texture rhythm
- `Reharm`: reseed progression choice

Each feature maps mostly to one role for future UI/debug use, but the arrangement applies globally so all roles can adapt to the shared state.

## Compatibility Rules

Generated parts must fit regardless of layering. The implementation enforces this by constraining all features to a shared generator:

- pitch material comes from the shared progression, voicing, or scale snap
- bass and lead movement use scale-safe intervals
- chord changes come from progression reseeding, not arbitrary note insertion
- rhythm features alter density or onset positions inside fixed bar grids
- sidechain is computed from the deterministic kick schedule, even on boxes that do not render drums
- final color/saturation clamps output to a bounded range

No feature may introduce a sample, chord, pitch, or free-running LFO that depends on local unsynchronized state.

## Screen Contract

The LCD must show:

- device id
- play/stop status
- primary role, prominently
- current arrangement codename
- next arrangement codename
- bars remaining until the next phrase
- peer count
- sync error
- bar progress

The screen should not show literal feature names like "DOUBLE HATS" or "WALKING BASS" in the normal performance view. It should show the coined identity of the current feature combination and the next coined identity.

## Codename Rules

The codename is a short pronounceable word derived from the active feature fingerprint.

Requirements:

- deterministic for the same seed, roster, phrase, and feature window
- changes when the active combination changes
- non-descriptive; it should not reveal feature names
- ASCII-only for the firmware font path
- fixed small maximum length for the 128x64 LCD

Example shape:

```text
Consonant + vowel + consonant + vowel [+ consonant + vowel]
```

## Mesh Roster

The roster is derived locally from the sync engine:

- include this node
- include fresh peers
- sort by node id
- dedupe
- expose this node's index

This roster drives both role assignment and feature selection turns. Because all healthy peers should converge on the same fresh peer set, every box should derive the same arrangement.

If rosters temporarily differ during join/leave churn, boxes may briefly compute different role splits or codenames. That is acceptable as a transitional mesh state; audio remains local and bounded.

## Audio API Changes

The old full-mix renderer is replaced by per-role rendering:

```rust
render_role(role, mesh_us, BeatCtx) -> f32
color(sum, mesh_us, sample_rate, Tone) -> f32
```

`BeatCtx` includes:

- transport
- seed
- sample rate
- resolved arrangement params
- the active `Kit` (vibe): instrument timbres + tape/vinyl tone

`Device::render_audio` resolves the roster, arrangement, and kit once per block,
determines this device's assigned roles, renders only those roles per sample,
then applies the vibe's `color` (tape saturation + vinyl air + grit) and the
kit-tuned master lowpass.

## Sound Framework

The synthesis layer is deliberately *data-driven* so the catalogue can grow to
hundreds of timbres and vibes without new render code.

### Instruments are data (`music::patch`)

Every instrument is a `Patch` value — additive `partials`, an optional 2-op `fm`
"bark", a `noise` band, a `pitch_env` drop, plus amp envelope, vibrato, tremolo,
and drive. One pure function, `render_patch(patch, freq, age, nz)`, renders them
all, so *adding an instrument is adding a `const Patch`* — no engine changes, and
it inherits determinism for free.

### Vibes are curated bundles (`music::kit`)

A `Kit` names one `Patch` per role (keys/bass/lead/pad/kick/snare/hat) plus a
`Tone` (tape wow/flutter, master cutoff, saturation, vinyl air). A kit is a vibe:
instruments chosen to sound like one record. Kits are selected deterministically
from the seed, so the whole mesh lands on the same vibe.

### Tape/vinyl character (`music::character`)

`warble` is a shared, deterministic tape pitch wobble (slow wow + faster flutter)
applied to every pitched voice; `vinyl` is the hiss/crackle bed. Both are pure
functions of mesh time, so the swarm wows as one tape.

### Extending the catalogue

1. Add a `const Patch` in `kit.rs` (or a new preset module) and list it in
   `ALL_PATCHES` so the property tests cover it.
2. Add a `const Kit` wiring presets to roles + a `Tone`, and list it in `KITS`.
3. That's it — `kit_for(seed)` can now land on the new vibe; no changes to
   `beat`, `device`, or the mesh. The bounded/decay tests guard every preset.

## Implementation Status

Implemented:

- `music::arrangement` roles, parameters, features, phrases, codenames, and deterministic tests
- sorted mesh roster view and local node index
- per-role beat rendering with lead, texture, shared sidechain, and bounded output tests
- device audio resolution and role assignment
- role/current/next codename display state and framebuffer tests

Next work:

- split the large arrangement, beat, and preset modules before extending them
- schedule shared seed/state snapshots for late joiners
- add call/response input and future events
- tune patches, filters, and output gain on the physical speaker path
