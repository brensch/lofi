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

- 192 sample elements in a 5.95 MiB fixed binary pack;
- 82 kick, snare, and hat variants;
- 99 root-tagged bass, keys, and lead one-shots;
- 11 phase-aligned bass, harmony, melody, and texture loops;
- source-wide tempo/downbeat maps, source hashes, bounded drum hits, and phrase
  phases used in coherent scenes;
- tone signatures used for bounded per-device coloration.

No MIDI file or synthesized note voice ships in the runtime. The current audible
path uses grid-conformed tonal loops and source-matched drum one-shots. Bass
loops are RMS-normalized to -21 dB before packing so source peak differences do
not become large loudness differences at runtime.
Root-tagged tonal one-shots remain in the catalogue for future source-compatible
arrangements but are not currently scheduled.

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
3. Require every audible stem in a scene to share its source hash and phrase phase.
4. Preserve a stable four-bar identity; vary only reviewed aligned takes or stems.
5. Run shipped-pack transient and loop-seam tests.
6. Render every selectable seed through the real browser AudioWorklet/WASM path.
7. Require technical, CLAP, Audiobox, and human listening approval.
8. Complete a separate commercial-rights review before shipping any audio bytes.

Harmony-only scenes stay catalogued but are not eligible for automatic playback
while reviewed melody-backed scenes exist. This prevents a technically valid
but perceptually weak scene from entering the organic rotation.

## Growth Model

The useful unit is a reviewed source-coherent scene, not an isolated generated
note. Future packs should add complete aligned drum, bass, harmony, melody, and
texture families. The app can later install versioned packs while firmware keeps
the same bounded renderer. This adds musical material without adding a model,
allocator, filesystem, or network audio dependency to the device.
