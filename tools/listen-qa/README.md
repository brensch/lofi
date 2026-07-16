# Listen QA

This workstation-only loop renders the real browser AudioWorklet and WASM path
to a WAV, then checks level, dynamics, tempo, beat-interval jitter, tonal
consistency, stereo output, and clipping. An optional CLAP pass compares the
audio with positive lo-fi and negative failure descriptions. CLAP is advisory;
a human listening rejection always overrides it.

Run the complete technical iteration loop with:

```sh
LISTEN_QA_PYTHON=~/.cache/lofi-tools/audio-analysis/.venv/bin/python \
NUMBA_CACHE_DIR=/tmp/lofi-numba-cache \
  tools/listen-qa/iterate.sh
```

It rebuilds the production WASM, renders three complete 96-second phrase arcs,
renders each isolated bass role, checks phrase evolution, and verifies bass
level consistency and upper-band energy. Set `LISTEN_QA_MODELS=1` to add the
slower cached CLAP and Audiobox Aesthetics passes.

Build WASM and render one complete five-module evolution arc:

```sh
npm run build:web
node tools/listen-qa/render.mjs \
  --seed 2 --nodes 5 --duration 96 \
  --output target/listen-qa/seed-2.wav
```

Run deterministic measurements with the workstation analysis environment:

```sh
NUMBA_CACHE_DIR=/tmp/lofi-numba-cache \
  ~/.cache/lofi-tools/audio-analysis/.venv/bin/python \
  tools/listen-qa/evaluate.py target/listen-qa/seed-2.wav \
  --clap --aesthetics --output target/listen-qa/seed-2.json
```

The Python dependencies and downloaded CLAP/Audiobox weights are workstation
tooling. They are not linked into `lofi-core`, firmware, or the browser app.
