#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
PYTHON=${LISTEN_QA_PYTHON:-python3}
OUTPUT=${LISTEN_QA_OUTPUT:-"$ROOT/target/listen-qa/iteration"}
DURATION=${LISTEN_QA_DURATION:-96}
MODEL_FLAGS=()

if [[ ${LISTEN_QA_MODELS:-0} == 1 ]]; then
  MODEL_FLAGS=(--clap --aesthetics)
fi

cd "$ROOT"
mkdir -p "$OUTPUT"
npm run build:web

for seed in 0 1 2; do
  node tools/listen-qa/render.mjs \
    --seed "$seed" --nodes 5 --duration "$DURATION" \
    --output "$OUTPUT/seed-$seed.wav"
  node tools/listen-qa/render.mjs \
    --seed "$seed" --nodes 5 --solo 3 --duration 48 \
    --output "$OUTPUT/bass-$seed.wav"
  "$PYTHON" tools/listen-qa/evaluate.py "$OUTPUT/seed-$seed.wav" \
    "${MODEL_FLAGS[@]}" --output "$OUTPUT/seed-$seed.json"
done

"$PYTHON" tools/listen-qa/bass_qa.py \
  "$OUTPUT/bass-0.wav" "$OUTPUT/bass-1.wav" "$OUTPUT/bass-2.wav" \
  --output "$OUTPUT/bass.json"
