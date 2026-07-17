# Listen QA

This workstation-only loop renders the real browser AudioWorklet and WASM path
to a WAV, then checks level, dynamics, tempo, beat-interval jitter, tonal
consistency, stereo output, and clipping. An optional CLAP pass compares the
audio with positive lo-fi and negative failure descriptions. CLAP is advisory;
a human listening rejection always overrides it.

Every render also disposes the processor and asserts that its next `process()`
call returns `false`. This guards against accumulating hidden WASM meshes across
listening-study candidates.

Run the complete technical iteration loop with:

```sh
LISTEN_QA_PYTHON=~/.cache/lofi-tools/audio-analysis/.venv/bin/python \
NUMBA_CACHE_DIR=/tmp/lofi-numba-cache \
  tools/listen-qa/iterate.sh
```

It rebuilds the production WASM, renders three complete 96-second phrase arcs,
renders each bass-bearing module, checks phrase evolution, and verifies bass
level consistency and upper-band energy. A physical module always carries one
rhythm and one tonal lane, so this intentionally measures the shipped
`Pulse + Low` output rather than a nonexistent stem-solo mode. Set
`LISTEN_QA_MODELS=1` to add the
slower cached CLAP and Audiobox Aesthetics passes.

Build WASM and render one complete five-module evolution arc:

```sh
npm run build:web
node tools/listen-qa/render.mjs \
  --seed 2 --nodes 5 --bpm 80 --start-phrase 7 --duration 24 \
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

Measure the real-time worklet hot path with reusable output buffers and telemetry
disabled, matching the listening study:

```sh
node --expose-gc tools/listen-qa/benchmark-worklet.mjs
```

The report includes the 2.67 ms render budget, latency percentiles, and retained
heap growth across 20,000 audio quanta. Optional arguments select the iteration
count, seed, and starting phrase.

Audit browser lifecycle cleanup across three complete replays:

```sh
node tools/listen-qa/audit-browser-teardown.mjs
```

The audit instruments worklet construction, explicit disposal, disconnection,
and audio-context suspension in headless Chrome.
