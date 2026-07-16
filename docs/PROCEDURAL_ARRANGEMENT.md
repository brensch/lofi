# Arrangement Status

The original procedural one-shot compositor is retired. It selected independently
harvested pitched fragments for bass, chords, and melody. Although its events
were synchronized, the fragments did not share a performance or reliable tonal
context, so the result sounded disconnected and failed listen QA.

## Active Arrangement

The shared seed now selects one `LoopScene`. All audible stems in that scene
carry the same `source_hash` and original four-bar phase. The mesh roster deals
five playback roles across available modules, and every module derives its loop
position from the same transport tick.

This deliberately trades infinite note-level mutation for musical coherence.
Variation currently comes from:

- three reviewed-development source scenes;
- different module counts and physical placement;
- deterministic tone profiles and vinyl character;
- scheduled seed/scene changes on shared bar boundaries.

## Reintroducing Procedural Variation

Future variation must operate inside a source-compatible family. Acceptable
extensions include muting stems, choosing alternate aligned takes, changing
four-bar sections, and selecting one-shots tagged with the active scene's source,
key, and progression. Random cross-source pitched selection is not acceptable.

Any extension must:

1. remain deterministic from shared state and mesh time;
2. remain allocation-free in `lofi-core`;
3. preserve bounded CPU work per sample;
4. render through `tools/listen-qa/render.mjs`;
5. pass technical, CLAP, and Audiobox checks for every selectable scene;
6. receive human listening approval before release.

See [Music Engine](MUSIC_ENGINE.md), [AI Content Pipeline](AI_CONTENT_PIPELINE.md),
and [Listen QA](LISTEN_QA.md).
