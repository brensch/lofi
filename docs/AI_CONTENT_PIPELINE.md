# AI-Assisted Content Pipeline

## Boundary

AI generation is an offline composition and production reference tool. It does
not run on the ESP32-S3, does not participate in real-time playback, and does
not send audio over ESP-NOW. The device ships only reviewed fixed-size data and
small redistributable samples.

```text
prompt + seed -> reference render -> stems -> audio slices + root estimates
              -> quality gates -> indexed sample pack -> no_std sampler
```

Generated audio is never promoted automatically into a release pack. Stem
separation and root-pitch estimates are unreliable for quiet or polyphonic
parts, so the forge rejects weak detections. A human must approve the musical
result and the commercial provenance of every shipped asset.

## First Harvest

The first reference batch was generated locally with ACE-Step 1.5 at 80 BPM,
G major, 4/4, and 32 seconds. Its prompt requested sparse muted guitar, close
voice-led Rhodes, dry boom-bap drums, round bass, clear rests, and section
changes through subtraction rather than continuous novelty.

Reference seeds:

| Reference | Seed | Use |
| --- | ---: | --- |
| `planned-theme-a.wav` | `168400724` | sparse foreground and open high end |
| `planned-theme-b.wav` | `235834419` | late kick pocket and guitar response |
| `planned-theme-c.wav` | `3552974743` | darker drums, late hats, steady bass |

The raw renders, Demucs six-stem output, offline pYIN pitch estimates, and generated
reports live under `target/ai-reference/` and are intentionally ignored by Git.
The reviewable preview of the embedded implementation is
`target/sample-only-catalog-preview.wav`.

The firmware content currently contains:

- 233 sample elements in an 8.75 MiB fixed binary pack;
- 94 kick, snare, and hat variants;
- 95 root-tagged bass, keys, and lead one-shots;
- 44 drum, bass, harmony, melody, and texture loops;
- four-bar trigger, timing, dynamics, and tone signatures;
- six scale-degree patterns that schedule sampled melodic fragments.

No MIDI file or synthesized note voice ships in the runtime. Scale degrees and
chord voicings only select a target transposition; the audible result is always
a harvested sample. Root detection remains an offline labelling tool.

## Reproducing A Report

ACE-Step, Demucs, librosa, NumPy, and SoundFile are workstation tools and are
not Cargo or npm dependencies. The complete unattended loop is:

```sh
~/.cache/lofi-tools/audio-analysis/.venv/bin/python \
  tools/content-forge/forge.py --start-server --forever
```

Every run records its prompt and seed, generates through ACE-Step, separates six
stems, checks vocals/clipping/stem activity, slices audio, estimates roots,
deduplicates, and rebuilds a balanced size-bounded pack. Generated runs remain
under `target/content-forge/`; only a reviewed pack is published to `assets/`.

## Acceptance Gates

Before adding a content pack:

1. Confirm the render prompt, model, seed, and local source files are recorded.
2. Reject vocals, recognizable copyrighted melodies, clipping, and broken stems.
3. Rewrite candidate notes into tonic-relative scale degrees and simplify them.
4. Preserve a stable four-bar identity; vary only endings, density, or register.
5. Confirm strong melody notes agree with the selected mode and chord targets.
6. Render every signature with multiple kits and arrangements.
7. Run the full workspace tests and inspect discontinuities in a WAV render.
8. Complete a separate commercial-rights review before shipping any audio bytes.

## Growth Model

The useful unit is a content pack, not a generated song. Future packs should add
compatible families of motifs, drum pockets, bass maps, chord templates, and
tone signatures. The app can later install versioned packs, while firmware keeps
the same bounded renderer. This provides new musical material without adding a
model, allocator, filesystem, or network audio dependency to the device.
