# Listen QA

## Verdict

**Automated lo-fi verdict: PASS. Human approval: pending.**

The prior one-shot compositor is explicitly rejected. User listening reported
that it sounded bad even though a genre classifier called it lo-fi. Human
rejection overrides every automated score.

The current candidate uses source-coherent, grid-conformed stem scenes and
transient-bounded drum hits. On 2026-07-16, seeds `0`, `1`, and `2` were each
rendered for 45 seconds with three modules through the real `mesh-worklet.js`
and `lofi_web.wasm` path.

| Seed | Tempo | Beat jitter | Onsets/s | Scale consistency | Stereo balance | Result |
| ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 0 | 80.36 BPM | 10.67 ms | 3.82 | 0.648 | 2.07 dB | PASS |
| 1 | 80.36 BPM | 10.67 ms | 2.91 | 0.643 | 2.84 dB | PASS |
| 2 | 80.36 BPM | 10.67 ms | 2.58 | 0.643 | 0.84 dB | PASS |

All three also pass level, headroom, crest factor, clipping, and active-stereo
checks. No render contains a clipped sample. The 10.67 ms figure is one
512-sample analysis hop, not measured transport drift.

An isolated five-module render placed all 30 strong kick attacks within one
64-sample analysis hop of their scheduled boundaries over 45 seconds. Before
the fix, a harvested 700 ms "kick" contained strong later attacks from the
source drum performance. Drum harvesting now ends a hit before the next source
onset. The matching Pocket render contained exactly the expected 120 attacks,
with a worst-case 7.83 ms onset-detector offset and no off-grid events.
Artificial vinyl impulses have also been removed from the audio path. Prior
CLAP/Audiobox scores describe the superseded candidate and are not carried
forward as evidence for this build.

## Reproduction

```sh
npm run build:web
node tools/listen-qa/render.mjs \
  --seed 2 --nodes 3 --duration 45 \
  --output target/listen-qa/seed-2.wav

NUMBA_CACHE_DIR=/tmp/lofi-numba-cache \
HF_HOME=~/.cache/huggingface \
  ~/.cache/lofi-tools/audio-analysis/.venv/bin/python \
  tools/listen-qa/evaluate.py target/listen-qa/seed-2.wav \
  --clap --aesthetics --output target/listen-qa/seed-2.json
```

The renderer executes the production AudioWorklet under a small Node shim. It
does not approximate the mesh, WASM, panning, gain ramps, or listener mix.

## Decision Rules

A candidate fails when any hard audio check fails, CLAP positive lo-fi
probability is below 55%, or Meta Audiobox Aesthetics predicts content enjoyment
or production quality below 7/10. Automated PASS is still provisional until a
human listens to the rendered WAV and approves it.

CLAP is the LAION `clap-htsat-unfused` model. Listening-quality axes come from
Meta's Audiobox Aesthetics model. Both are workstation-only and never ship in
the app or firmware.
