# Listen QA

## Verdict

**Automated lo-fi verdict: PASS. Human approval: pending.**

The prior one-shot compositor is explicitly rejected. User listening reported
that it sounded bad even though a genre classifier called it lo-fi. Human
rejection overrides every automated score.

The current candidate uses source-coherent, grid-conformed stem scenes,
transient-bounded drum hits, and deterministic eight-bar phrase evolution. On
2026-07-16, seeds `0`, `1`, and `2` were each rendered for 96 seconds with five
modules through the real `mesh-worklet.js` and `lofi_web.wasm` path. This covers
the complete four-phrase macro arc.

| Seed | RMS | Peak | Tempo | Beat jitter | Phrase range | Stereo balance | Result |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 0 | -28.03 dBFS | -9.09 dBFS | 80.36 BPM | 10.67 ms | 1.26 dB | 0.90 dB | PASS |
| 1 | -29.54 dBFS | -10.61 dBFS | 80.36 BPM | 10.67 ms | 1.36 dB | 0.15 dB | PASS |
| 2 | -28.27 dBFS | -9.28 dBFS | 80.36 BPM | 10.67 ms | 2.01 dB | 1.45 dB | PASS |

All three pass level, headroom, crest factor, clipping, active-stereo, scale,
rhythm, and musical-evolution checks. No render contains a clipped sample. The
10.67 ms figure is one 512-sample analysis hop, not measured transport drift.

Isolated bass renders also pass a dedicated gate. Their RMS levels span only
0.68 dB across the three selectable seeds, peaks remain below -16.3 dBFS, and
the ratio of energy above 800 Hz remains below 0.002%. The runtime adds a cheap
420 Hz low-pass to the bass role; the forge normalizes source bass loops to a
shared RMS target before packing them.

An isolated five-module render placed all 30 strong kick attacks within one
64-sample analysis hop of their scheduled boundaries over 45 seconds. Before
the fix, a harvested 700 ms "kick" contained strong later attacks from the
source drum performance. Drum harvesting now ends a hit before the next source
onset. The matching Pocket render contained exactly the expected 120 attacks,
with a worst-case 7.83 ms onset-detector offset and no off-grid events.
Artificial vinyl impulses have also been removed from the audio path. CLAP
positive lo-fi probability is 98.18-99.61% across the current seeds. Audiobox
content-enjoyment scores are 7.11-7.63 and production-quality scores are
7.39-7.83. Seed 0 uses the original aligned stems from the CC0 Orion
construction kit; its stem sum correlates 0.9998 with the published full mix,
avoiding source-separation residue. An earlier cadence failed the final phrase
window and was revised; an underperforming harmony-only scene was removed from
automatic selection.

## Reproduction

```sh
tools/listen-qa/iterate.sh

# Include the workstation-only CLAP and Audiobox gates.
LISTEN_QA_MODELS=1 tools/listen-qa/iterate.sh
```

The loop rebuilds WASM, renders every selectable seed for the full 96-second
arc, renders each bass role in isolation, and runs the technical and bass
gates. The renderer executes the production AudioWorklet under a small Node
shim. It does not approximate the mesh, WASM, panning, gain ramps, or listener
mix.

## Decision Rules

A candidate fails when any hard audio check fails, CLAP positive lo-fi
probability is below 55%, or Meta Audiobox Aesthetics predicts content enjoyment
or production quality below 7/10. The technical gate also rejects a 96-second
render when its 24-second phrase windows show no meaningful change in loudness
or onset density. Automated PASS is still provisional until a human listens to
the rendered WAV and approves it.

CLAP is the LAION `clap-htsat-unfused` model. Listening-quality axes come from
Meta's Audiobox Aesthetics model. Both are workstation-only and never ship in
the app or firmware.
