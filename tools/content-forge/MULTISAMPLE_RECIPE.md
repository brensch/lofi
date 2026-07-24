# Multisample Recipe (next forge session)

## Why

The symbolic engine's remaining timbre ceiling is the tonal one-shots: they
were sliced out of full-mix Demucs stems, so each note carries separation
residue, room inconsistency, and only 10 keys pitches exist across 3 sources.
Chords assembled from them read as ringtones. The fix is not more engine DSP —
it is **clean, purpose-generated multisamples**: solo single notes at known
pitches, no separation step at all.

## Generation prompts (ACE-Step, workstation)

Generate *note ladders*: one instrument, one note every 2 seconds, climbing a
known scale. The existing onset slicer in `harvest.py` can cut these, and root
tagging becomes trivial because the pitch order is known in advance.

| Instrument | Prompt sketch | Ladder |
| --- | --- | --- |
| Keys | "solo Rhodes electric piano, single sustained notes, one note every two seconds, chromatic scale ascending from E2 to E4, dry recording, no drums, no bass, no melody, soft velvet touch, warm, slight tape saturation" | E2-E4 chromatic (25 notes) |
| Keys (soft layer) | same, "played very softly, felt-muted" | E2-E4 in minor thirds (9) |
| Bass | "solo warm fingered electric bass, single sustained notes, one note every two seconds, chromatic ascending from B0 to E2, dry, round, no drums, no chords" | B0-E2 chromatic (18) |
| Lead | "solo muted jazz guitar, single plucked notes, one note every two seconds, chromatic ascending from A2 to A4, dry, intimate, warm" | A2-A4 in whole tones (13) |
| Lead (alt voice) | "solo mellow synth flute, single soft notes..." | A3-A4 chromatic (13) |

Also worth one run: **tuned kicks** — "solo boom bap kick drum, one hit every
two seconds, twelve hits" at 2-3 different tunings, so kick tuning can select
instead of repitch.

## Slicing and tagging

1. Run each render through the existing onset slicer with `--no-separation`
   (the render *is* the stem — skip Demucs entirely).
2. Assign roots positionally from the ladder definition; verify each with the
   existing pYIN estimate and reject any slice where they disagree.
3. Trim to ≤1.8 s with a 60 ms fade-out; RMS-normalize per kind as today.
4. Pack as the existing `KeysNote`/`BassNote`/`LeadNote` kinds with one new
   synthetic `source_hash` per instrument voice, so session casting keeps one
   consistent instrument per lane automatically.

## Acceptance

- ≥20 keys roots spanning E2-E4, ≥12 bass, ≥12 lead: the scene binder's
  worst-repitch metric should drop from ~4 semitones to ≤2.
- No slice with audible bleed, clicks, or double attacks (existing pack
  transient tests apply unchanged).
- Re-run `tools/listen-qa/candidates.py` afterwards: expect scale consistency
  and the mid-band presence to rise with zero engine changes.

The engine needs no modification for any of this — the catalogue format,
session casting, and scene binding already handle richer inventories.
