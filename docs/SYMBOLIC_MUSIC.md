# Symbolic Music

## Why symbolic

The loop engine's musicality is trapped inside harvested audio: it cannot be
inspected, diffed, property-tested, or improved without ears. Generative AI
cannot reliably *listen*, but it can reason about symbols. So the audible path
becomes a **symbolic composer**: every note and hit exists as data — pitch,
step, velocity, duration, lane — before it touches a sample. Audio samples
supply *timbre only*. The music itself lives in a domain where quality is
measurable: harmony rules, rhythm grids, density curves, voice leading, rests.

This replaces the source-coherent *loop* scene as the audible default. The
invariant changes from "never mix stems from different performances" to
"never emit a symbol the theory layer cannot justify."

## The representation

There is no stored score. A score is a pure function, in layers, all
deterministic from `(seed, mesh tick, roster)` — the same contract the mesh
already guarantees, so distributed boxes agree note-for-note without traffic.

```text
Session   = f(seed)                  tempo, key, mode, progression, groove
Harmony   = f(session, bar)          chord, voicing (voice-led), bass root
Pattern   = f(session, phrase)       per-lane step grids + comping shapes
Event     = f(pattern, harmony, step) concrete notes: pitch, velocity, length
Voice     = f(event, catalog)        a root-tagged one-shot + playback rate
```

### Session (per seed)

- **Tempo**: 72–86 BPM, seeded. No longer tied to a source performance.
- **Key**: any pitch class; **mode** comes from the progression template
  (major, dorian, aeolian, harmonic minor).
- **Progression**: one of the `music::progression` templates (ii–V–i,
  i–VI–III–VII, modal vamps…), voiced with `theory::voicing_near` so the four
  colour-tone voices move smoothly chord to chord.
- **Groove signature**: one of the measured `content::SIGNATURES` — step grids,
  swing percent, per-lane micro-delays and humanization harvested from the
  human-approved references.
- **Drum kit**: one source family (kick/snare/hat one-shots from the same
  performance) so the drums keep a coherent timbre.
- **Tonal voices**: one bass source, one keys source, one lead source, fixed
  for the session, so each lane sounds like one instrument.

### The grid

16 steps per bar (24 ticks each at 96 ticks/beat), 64-step four-bar cycle,
8-bar phrases, 4-phrase macro arc — unchanged from the mesh timeline. Swing
delays off-beat steps by the signature's swing percent of a step; each lane
adds its measured fixed delay (the "pocket") plus a bounded deterministic
per-step humanization (≤1 ms mesh-wide, identical on every box).

### Lanes (mapped onto the existing mesh roles)

- **Pulse — kick**: signature kick grid, thinned or thickened per phrase by
  the arrangement (half-time drops all but step 0; variations mask specific
  hits). Velocity accents beat 1.
- **Pocket — snare + hats**: backbeat on steps 4/12 always; ghost notes and
  phrase-end fills only when the arrangement says so. Hats play the signature
  grid with seeded gaps — never a metronomic full grid — with open hats and
  double-time as features, velocity dipping on off-beats.
- **Low — bass**: events on the signature bass grid. Pitch: chord root on the
  downbeat; fifth or octave on secondary hits; on `bass_approach` steps a
  diatonic approach tone walks into the *next* bar's root. Every pitch snaps
  to the session scale. One octave, register 24–36 to match sampled roots.
- **Color — keys**: the chord voicing played with a *comping pattern*, not a
  pad: sustained downbeat, push (anticipation on the "and of 4"), or broken
  (voices staggered a step apart). Velocities soft (≤72), top voice slightly
  louder. Sparse phrases drop to half density; rich phrases add the second
  strike. Voicing register sits near the sampled keys roots to keep repitch
  ratios small.
- **Motif — lead**: `content::MOTIFS` scale-degree contours over the session
  scale, arranged call/answer: bars 1–2 state the motif, bars 3–4 answer with
  a seeded transform (transpose within scale, tail inversion, thinning). The
  lead rests entirely in low-energy phrases. Register 55–67 near sampled lead
  roots.

### Arrangement

The distributed feature-card system is unchanged (phrase selector, two-card
window, spotlight, codenames), but cards now steer symbolic parameters —
comping shape, motif variation, grid masks, ghost/fill enables — instead of
loop gains. The 4-phrase arc shapes energy by *subtraction*: phrase 0 thins
hats and rests the lead, phrase 2 is the full groove, phrase 3 breaks down
(e.g. keys tacet or drums thin) before the cycle turns over.

## Rendering

`SymbolicScene` is resolved once at init/seed-change (never in the render
loop): for every pitch the session can emit, it binds the nearest root-tagged
one-shot from the lane's source family and stores the playback-rate ratio.
Per block, each lane collects its active events into a bounded voice list
(fixed capacity, no allocation), then per sample sums
`render_sample_pitched(sample, age, rate * warble)`. Tails ring through
chord changes naturally because samples decay in ~1.5 s.

The device master chain (bass lowpass, saturation, vinyl air, kit lowpass)
is unchanged.

## The feedback loop

Because the composer is symbolic and pure, the exact score is available
without listening:

- `cargo run -p lofi-core --example score_dump -- --seed N --phrases 4`
  emits every event as JSONL (lane, step, tick, midi, velocity, duration,
  chord, phrase) — the ground truth for property gates and diffs.
- `tools/listen-qa/scorecard.py` renders the WAV through the real worklet
  path and produces a visual report (mel spectrogram, self-similarity matrix,
  chromagram, onset grid) plus feature distances against the reference
  corpus. See [Listen QA](LISTEN_QA.md).

Property gates run on the symbolic log, so failures name the bar and lane:

1. Every pitch is in the session scale; bass downbeats are chord roots.
2. Backbeat integrity: snare on 4/12, never elsewhere except ghosts/fills.
3. Swing within 54–62 %; per-lane pocket delays within measured bounds.
4. Rest ratio: lead active on ≤25 % of steps; keys ≤40 %; mandatory space.
5. Phrase evolution: adjacent phrases differ in ≥1 lane pattern, but never
   change more than 2 lanes at once.
6. Voice leading: adjacent voicings move ≤3 semitones per voice, no crossing.
7. Register separation: bass ≤ B2 < keys ≤ lead; no unison collisions on
   the same step between lead and keys top voice.

## What stays true

- `no_std`, allocation-free, lock-free render path; fixed-capacity tables.
- Deterministic from `(seed, roster, mesh tick)`; boxes never stream notes.
- Loops remain in the pack and the loop engine remains selectable for A/B
  listening studies, but the symbolic engine is the product default.
- Workstation ML (CLAP/Audiobox) stays advisory; human judgement and the
  symbolic/statistical gates decide.
