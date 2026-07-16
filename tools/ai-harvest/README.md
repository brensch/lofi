# AI Reference Harvester

This offline tool measures candidate musical and production signatures from an
AI-generated reference after six-stem Demucs separation. It does not make an AI
render redistributable and it does not automatically promote detected notes into
firmware. Reports are review inputs; committed content must be deliberately
quantized, simplified, and provenance-checked.

The analysis environment needs `numpy`, `scipy`, `soundfile`, and `librosa`.
Optional note reports are read from Basic Pitch `--save-note-events` CSV files.

```sh
python tools/ai-harvest/analyze.py \
  target/ai-reference/stems-6/htdemucs_6s/planned-theme-a \
  --notes target/ai-reference/transcription-6 \
  --tempo 80 --key-midi 55 \
  --output target/ai-reference/harvest/planned-theme-a.json
```

The report captures:

- stem RMS, peak, spectral centroid, and 85% rolloff;
- detected onset count and offsets from the nearest sixteenth-note grid;
- quantized note starts, durations, velocity, and tonic-relative pitch;
- exact pitch-class/onset return between the first two four-bar windows.

Raw generations, stems, MIDI, CSV, and generated reports stay under `target/`
and are not committed. Only compact, reviewed musical signatures belong in
`lofi-core`.
