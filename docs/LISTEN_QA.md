# Listen QA

## Verdict

**Automated lo-fi verdict: PASS. Human approval: pending.**

The prior one-shot compositor is explicitly rejected. User listening reported
that it sounded bad even though a genre classifier called it lo-fi. Human
rejection overrides every automated score.

That failure reshaped the QA philosophy: classifier probability is no longer
treated as evidence of quality. The symbolic engine is instead gated by
instruments that measure craft directly — property gates over the exact
symbolic score, and audio features compared against approved references. See
[Symbolic engine QA](#symbolic-engine-qa) below.

**Symbolic engine status (2026-07-23 overnight sweep): 48 seeds, 13 pass
every property gate and craft window through the exact browser path. Human
approval: pending.** The best candidate (seed 6) measures *closer to the
approved production envelope than any loop-engine render* (corpus distance
0.57 vs 0.62/0.80/1.16), every survivor locks the four-bar repetition
stripe at 16 beats, and the loop engine itself fails the structure windows
on two of its three scenes. Render cost is ~4 % of the realtime budget for
a three-module worklet. The `/judge` deck now alternates eight gated
symbolic candidates against the twelve loop profiles, blind. Full sweep
detail: [docs/reports/2026-07-23-symbolic-overnight.md](reports/2026-07-23-symbolic-overnight.md).

The current candidate uses source-coherent, grid-conformed stem scenes,
transient-bounded drum hits, and deterministic eight-bar phrase evolution. On
2026-07-16, seeds `0`, `1`, and `2` were each rendered for 96 seconds with five
modules through the real `mesh-worklet.js` and `lofi_web.wasm` path. This covers
the complete four-phrase macro arc.

| Seed | RMS | Peak | Tempo | Beat jitter | Phrase range | Stereo balance | Result |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 0 | -28.16 dBFS | -9.08 dBFS | 80.36 BPM | 10.67 ms | 1.33 dB | 1.11 dB | PASS |
| 1 | -28.87 dBFS | -10.69 dBFS | 80.36 BPM | 10.67 ms | 1.69 dB | 0.59 dB | PASS |
| 2 | -29.26 dBFS | -9.33 dBFS | 80.36 BPM | 10.67 ms | 2.56 dB | 0.56 dB | PASS |

All three pass level, headroom, crest factor, clipping, active-stereo, scale,
rhythm, and musical-evolution checks. No render contains a clipped sample. The
10.67 ms figure is one 512-sample analysis hop, not measured transport drift.

Isolated bass renders also pass a dedicated gate. Their RMS levels span only
0.74 dB across the three selectable seeds, peaks remain below -15.7 dBFS, and
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
positive lo-fi probability is 97.10-99.09% across the current seeds. Audiobox
content-enjoyment scores are 7.11-7.66 and production-quality scores are
7.34-7.92. Seed 0 uses the original aligned stems from the CC0 Orion
construction kit; its stem sum correlates 0.9998 with the published full mix,
avoiding source-separation residue. An earlier cadence failed the final phrase
window and was revised; an underperforming harmony-only scene was removed from
automatic selection.

## Symbolic engine QA

The symbolic composer is inspectable without ears, so its gates run in the
symbolic domain first and the audio domain second:

```sh
# 1. The exact score as data: every note, velocity, and micro-delay.
cargo run -p lofi-core --example score_dump -- <seed> 4 78000 > score.jsonl

# 2. Property gates that name the failing bar and lane: backbeat integrity,
#    chord-root basslines, diatonic approach tones, rest ratios, register
#    separation, phrase evolution, pocket bounds, repitch limits.
tools/listen-qa/symbolic_gates.py score.jsonl

# 3. The browser-path render, measured: swing, rest ratio, scale consistency,
#    band balance, four-bar repetition stripe, beat novelty — plus a visual
#    report (mel spectrogram, self-similarity matrix, chromagram, onsets).
tools/listen-qa/scorecard.py analyze mix.wav --bpm 78 --out report/

# 4. Distance from the approved-reference corpus, and craft windows.
tools/listen-qa/scorecard.py compare report/features.json corpus.json

# The overnight driver runs all of the above per seed and ranks survivors.
tools/listen-qa/candidates.py --seeds 0,1,2,... --corpus corpus.json
```

A symbolic candidate reaches the judge deck only when the property gates all
hold, every craft window is met, and its corpus distance is comparable to the
loop engine's own renders. Human listening on `/judge` remains the final
authority; the deck now runs both engines blind, alternating between them.

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
