# Arrangement Status

The original procedural one-shot compositor is retired. It selected independently
harvested pitched fragments for bass, chords, and melody. Although its events
were synchronized, the fragments did not share a performance or reliable tonal
context, so the result sounded disconnected and failed listen QA.

## Active Arrangement

The shared seed now selects one `LoopScene`. All audible samples in that scene
carry the same `source_hash`; tonal stems retain their original four-bar phase.
Kick, snare, and hats are scheduled on a fixed shared grid from aligned
one-shots. The forge first conforms all stems from a source to one measured beat
map and prevents each drum hit from carrying the following source transient.
The mesh roster deals five playback roles across available modules, and every
module derives hit and loop positions from the same transport tick.

This deliberately trades infinite note-level mutation for musical coherence.
The session evolves automatically every eight bars. Shared arrangement
parameters vary kick and hat patterns, ghost notes, fills, bass and harmony
weight, texture balance, and motif activity. A restrained four-phrase energy
arc prevents each change from feeling random. Continuous stem-level changes
crossfade over the first beat; discrete rhythm changes start on the exact phrase
boundary.

Variation currently comes from:

- two melody-backed reviewed-development source scenes;
- deterministic eight-bar arrangement changes;
- different module counts and physical placement;
- deterministic tone profiles and vinyl character.

## Reintroducing Procedural Variation

Further variation must operate inside a source-compatible family. Acceptable
extensions include choosing alternate aligned takes, changing four-bar
sections, and selecting one-shots tagged with the active scene's source, key,
and progression. Random cross-source pitched selection is not acceptable.

Any extension must:

1. remain deterministic from shared state and mesh time;
2. remain allocation-free in `lofi-core`;
3. preserve bounded CPU work per sample;
4. render through `tools/listen-qa/render.mjs`;
5. pass technical, CLAP, and Audiobox checks for every selectable scene;
6. receive human listening approval before release.

See [Music Engine](MUSIC_ENGINE.md), [AI Content Pipeline](AI_CONTENT_PIPELINE.md),
and [Listen QA](LISTEN_QA.md).
