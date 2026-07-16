# Listen QA

## Verdict

**Automated lo-fi verdict: PASS. Human approval: pending.**

The prior one-shot compositor is explicitly rejected. User listening reported
that it sounded bad even though a genre classifier called it lo-fi. Human
rejection overrides every automated score.

The current candidate uses source-coherent loop scenes. On 2026-07-16, seeds
`0`, `1`, and `2` were each rendered for 45 seconds with three modules through
the real `mesh-worklet.js` and `lofi_web.wasm` path.

| Seed | Tempo | Scale consistency | Stereo balance | CLAP lo-fi | Enjoyment | Production | Result |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 0 | 39.9 BPM half-time | 0.638 | 0.15 dB | 97.7% | 7.80/10 | 7.62/10 | PASS |
| 1 | 80.4 BPM | 0.622 | 2.76 dB | 99.0% | 7.54/10 | 7.76/10 | PASS |
| 2 | 80.4 BPM | 0.630 | 0.08 dB | 99.1% | 7.50/10 | 7.64/10 | PASS |

All three also pass level, headroom, crest factor, clipping, and active-stereo
checks. No render contains a clipped sample. The rejected baseline had lower
scale consistency (`0.601`), excessive onset density (`5.58/s`), and failed the
stereo-balance gate.

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
