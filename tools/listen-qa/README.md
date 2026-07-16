# Listen QA

This workstation-only loop renders the real browser AudioWorklet and WASM path
to a WAV, then checks level, dynamics, tempo, tonal consistency, stereo output,
and clipping. An optional CLAP pass compares the audio with positive lo-fi and
negative failure descriptions. CLAP is advisory; a human listening rejection
always overrides it.

Build WASM and render the default three-module composition:

```sh
npm run build:web
node tools/listen-qa/render.mjs \
  --seed 2 --nodes 3 --duration 45 \
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
