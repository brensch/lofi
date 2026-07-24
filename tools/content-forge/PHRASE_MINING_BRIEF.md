# Phrase Mining Brief (GPU box session)

## Mission

Mine the lofi generation model for **symbolic performance phrases**: one-bar
and two-bar patterns per lane (keys comping, basslines, melodies, drum bars),
captured as note events, tagged with the chord and context they were played
over. These become the corpus for the phrase-sequencing composer being built
on the `claude/procedural-lofi-generation-76ykkn` branch. The note-level
composer is being retired: the human verdict is that note-by-note layouts
cannot sound good regardless of timbre. Real performances, symbolically
captured, are the replacement.

## Why symbolic capture (not audio phrases)

- Note events are tiny (flash-friendly for the eventual device) and fully
  inspectable — every phrase can be property-gated, diffed, transposed, and
  re-voiced by the composer.
- Transcription errors are survivable: we mine for *patterns*, and the ingest
  gates reject or snap anything that disagrees with the known chord chart.

## The loop

For each batch (repeat with varied prompts/seeds until yields plateau):

1. **Generate with known ground truth.** Prompt ACE-Step with a fixed chord
   chart, key, and tempo you choose and record in the metadata. Favor simple
   two- and four-chord charts from this vocabulary (matches the runtime
   engine): min9, min7, maj7, maj9, dom9, half-dim; roots anywhere. 70-84 BPM.
   Prompts should request sparse, distinct lanes ("sparse Rhodes comping, warm
   fingered bass, dry boom-bap drums, one quiet guitar melody, clear rests").
2. **Separate** with Demucs six-stem as today.
3. **Transcribe per stem, constrained:**
   - bass → pYIN f0 + onset segmentation (monophonic assumption);
   - keys/piano-ish → ByteDance piano-transcription (fallback: Basic Pitch);
   - melody/guitar → Basic Pitch, keep only confident monophonic lines;
   - drums → onset detection + band classification (kick/snare/hat) as in
     the existing forge heuristics.
4. **Canonicalize**: quantize onsets to a 16th grid at the known tempo
   (record the pre-quantization offset in microseconds — that is the groove,
   keep it!), transpose so the chart's tonic is pitch class 0, and slice into
   one-bar and two-bar phrases aligned to the known chart.
5. **Gate before writing** (reject silently, log counts):
   - every pitch within the known key's scale or the bar's chord tones;
   - keys: 2-6 distinct pitches per bar, ≤6 strikes per bar;
   - bass: monophonic, register ≤ MIDI 48, downbeat is root/fifth/third;
   - melody: monophonic, ≤8 notes/bar, at least one rest ≥ a beat per 2 bars;
   - drums: snare only on backbeat ± ghosts below 40% velocity;
   - dedupe by (lane, chord quality, quantized onset set, pitch-class set).
6. **Write JSONL**, one phrase per line, to `assets/phrases/mined.jsonl`
   (git-tracked; append across batches; keep total < 2 MB for now).

## Output schema (one line per phrase)

```json
{
  "id": "sha1-of-content",
  "lane": "keys|bass|melody|kick|snare|hat",
  "bars": 1,
  "chord": {"root_pc": 0, "quality": "min9"},
  "next_chord": {"root_pc": 5, "quality": "min7"},
  "key_mode": "dorian|aeolian|major|harmonic_minor",
  "tempo_bpm": 76,
  "energy": 0.42,
  "events": [
    {"tick": 0, "midi": 62, "vel": 64, "dur_ticks": 96, "offset_us": -4200}
  ],
  "provenance": {"prompt_hash": "…", "seed": 123, "model": "ace-step-1.5"}
}
```

Ticks: 96 per beat, 384 per bar (matches the runtime). `midi` is in the
transposed frame (tonic pc = 0); drums use `midi: 0` and the lane carries the
voice. `offset_us` is the captured micro-timing relative to the quantized
grid. `energy` = normalized RMS of the source bar. `next_chord` matters:
comping and bass phrases that lead into a change are the good ones.

## Targets

- ≥120 keys phrases, ≥80 bass, ≥60 melody, ≥60 drum bars, spread over ≥4
  chord qualities and ≥3 energy tiers. Yields below ~20%/batch usually mean
  the prompt is too dense — simplify the requested arrangement.

## Optional paid tooling

If transcription quality on keys stems is the bottleneck, AnthemScore has a
CLI and Klangio an API; both are batchable. Melodyne is better than either
but GUI-only — use it (if at all) for hand-curating a "golden 20" set, not in
the loop. Start with the free stack; the ingest gates do the quality control.

## Contract with the consuming side

The repo session is building, against this exact schema:
- `tools/phrase-lab/ingest.py` — validates, gates, dedupes, and packs
  `mined.jsonl` into the fixed binary phrase table the engine reads;
- the phrase-sequencing composer (`music::score` v2) that selects mined
  phrases by (chord, energy, variation) on the shared mesh timeline;
- the browser-path reference renderer for A/B listening.

Do not change the schema unilaterally — if a field is missing, add it as a
new optional key and note it in this file in your branch's commit.
