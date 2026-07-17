# Listening Study

The browser route `/judge` runs a blinded binary preference study against the
real WASM music engine. Each candidate creates a fresh three-module mesh and
plays exactly eight bars at its source performance's native 72 or 80 BPM. Mesh
traffic is pre-rolled before a shared transport anchor, so the first audible
frame is the start of an arrangement phrase.

## Trial Flow

1. Start the candidate and listen to all eight bars.
2. Pause or resume without changing musical elapsed time.
3. After completion, optionally select positive and problem tags or add a note.
4. Submit the cross or tick. The next candidate starts immediately.
5. Replay is allowed and recorded; voting remains unavailable until a complete
   playback has finished.

Each trial owns exactly one AudioWorklet processor and three WASM instances.
After the output fade, the page sends an explicit dispose command; the processor
drops its instance and packet references and returns `false` from `process()` so
the browser removes it from the realtime thread. Replays, replacements, errors,
and page teardown use the same shutdown path.

The **Copy debug** action captures the active release and candidate, browser and
audio-context details, main-thread delay, lifecycle counts, packet statistics,
every module's mesh state, callback cadence, frame gaps, output discontinuities,
and clipping counters. It requests worklet state only when pressed and copies a
formatted JSON report suitable for attaching to a playback issue.

Candidates come from a shuffled, curated profile deck. Catalog analysis groups
the two perceptually similar 80 BPM sources together, so consecutive trials must
cross between that group and the distinct 72 BPM source as well as changing
rhythmic family. Across the deck, the engine compares half-time, double and
sparse hats, open-hat pockets, kick variants, fills, swing, tone profiles,
groove signatures, sampled bass-walk spotlights, and arrangement phases. The UI
hides seed and profile data before the vote to avoid expectation bias.

## Records

The browser writes every vote to local storage before attempting the network.
It POSTs the same record to `/api/judgements`, where the Vite development or
preview server validates and appends it to:

```text
target/listening-study/judgements.jsonl
```

Record IDs are idempotent. Retrying after a lost response cannot duplicate a
vote. Pending local records retry when the page next opens, and the Export
button provides a JSON backup.

Each record includes the build revision, seed, profile ID, source slot, starting
phrase, tempo, sample rate, sequence, module count, duration, verdict, tags,
note, replay count, decision time, and actual listening duration. Build revision,
seed, starting phrase, and tempo reproduce a candidate from the matching
catalogue and engine source.

## Analysis

Summarize the local dataset from the repository root:

```sh
node tools/judgement-report.mjs
```

The report deduplicates by record ID and shows like rate, tag signal, liked and
disliked seeds, replay volume, and notes. Automated genre and quality scores can
later be joined by seed, but they must not be shown before the human vote.
