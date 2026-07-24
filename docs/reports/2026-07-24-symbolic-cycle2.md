# Overnight symbolic engine report

Sweep: 48 symbolic seeds -> **6 deck-worthy candidates**.

Every candidate below passed all symbolic property gates (backbeat,
chord-root bass, diatonic approaches, rests, register, evolution,
pocket bounds) and every audio craft window, measured through the
exact browser AudioWorklet/WASM path.

## Candidates vs the loop engine

| track | swing | rest | scale | 4-bar stripe | novelty | onsets/s | corpus dist |
|---|---|---|---|---|---|---|---|
| symbolic seed 6 (WINDOWLIGHT) | 0.579 | 0.026 | 0.700 | 0.121 | 0.258 | 3.250 | 0.72 |
| symbolic seed 10 (WINDOWLIGHT) | 0.587 | 0.036 | 0.641 | 0.152 | 0.253 | 3.812 | 0.99 |
| symbolic seed 45 (WINDOWLIGHT) | 0.584 | 0.181 | 0.665 | 0.163 | 0.284 | 2.771 | 1.98 |
| symbolic seed 19 (FLOATING) | 0.569 | 0.194 | 0.697 | 0.263 | 0.235 | 2.718 | 2.51 |
| symbolic seed 39 (POLAROID) | 0.579 | 0.201 | 0.661 | 0.311 | 0.266 | 2.968 | 2.78 |
| symbolic seed 24 (WINDOWLIGHT) | 0.577 | 0.177 | 0.684 | 0.537 | 0.697 | 2.635 | 3.96 |
| loop seed 0 (reference) | 0.550 | 0.000 | 0.838 | 0.016 | 0.026 | 4.197 | 0.00 |
| loop seed 1 (reference) | 0.572 | 0.055 | 0.745 | 0.055 | 0.354 | 3.218 | 0.00 |
| loop seed 2 (reference) | 0.556 | 0.062 | 0.753 | 0.141 | 0.448 | 2.948 | 0.00 |

Reference craft windows: swing 0.52-0.68, rest 0.02-0.30, scale >= 0.60,
4-bar stripe >= 0.10, novelty 0.25-0.75, onsets 1.8-4.5/s.

Note: loop seeds 0 and 1 fail the 4-bar stripe and novelty windows
themselves - the loop engine reaches its own structural bar only on
seed 2. Corpus distance is the rms z-distance to the loop-render
corpus, so the loop rows sit at ~0 by construction; for symbolic
candidates lower means closer to the approved production envelope.

## Failures and why

- seed 5 (WINDOWLIGHT): structure_cycle_stripe=0.054 outside [0.1, 1.0]; structure_novelty_mean=0.145 outside [0.22, 0.75]
- seed 3 (POLAROID): structure_cycle_stripe=0.030 outside [0.1, 1.0]; structure_novelty_mean=0.104 outside [0.22, 0.75]; duck_depth=-0.027 outside [0.08, 0.8]
- seed 12 (WINDOWLIGHT): structure_cycle_stripe=0.080 outside [0.1, 1.0]; structure_novelty_mean=0.156 outside [0.22, 0.75]; band_low=0.860 outside [0.5, 0.84]
- seed 40 (WINDOWLIGHT): structure_cycle_stripe=0.059 outside [0.1, 1.0]; structure_novelty_mean=0.082 outside [0.22, 0.75]
- seed 47 (WINDOWLIGHT): structure_cycle_stripe=0.042 outside [0.1, 1.0]; structure_novelty_mean=0.146 outside [0.22, 0.75]; band_low=0.852 outside [0.5, 0.84]
- seed 25 (WINDOWLIGHT): band_low=0.871 outside [0.5, 0.84]
- seed 30 (WINDOWLIGHT): structure_cycle_stripe=0.063 outside [0.1, 1.0]; structure_novelty_mean=0.119 outside [0.22, 0.75]; duck_depth=-0.667 outside [0.08, 0.8]
- seed 34 (POLAROID): duck_depth=-0.208 outside [0.08, 0.8]; band_low=0.873 outside [0.5, 0.84]
- seed 41 (WINDOWLIGHT): structure_novelty_mean=0.175 outside [0.22, 0.75]; duck_depth=-0.160 outside [0.08, 0.8]; band_low=0.845 outside [0.5, 0.84]
- seed 4 (WINDOWLIGHT): structure_novelty_mean=0.135 outside [0.22, 0.75]; band_low=0.850 outside [0.5, 0.84]
- seed 23 (POLAROID): structure_cycle_stripe=0.029 outside [0.1, 1.0]; structure_novelty_mean=0.075 outside [0.22, 0.75]; band_low=0.846 outside [0.5, 0.84]
- seed 21 (FLOATING): structure_novelty_mean=0.203 outside [0.22, 0.75]; duck_depth=-0.473 outside [0.08, 0.8]
- seed 36 (POLAROID): structure_cycle_stripe=0.043 outside [0.1, 1.0]; structure_novelty_mean=0.068 outside [0.22, 0.75]
- seed 38 (POLAROID): structure_novelty_mean=0.162 outside [0.22, 0.75]
- seed 2 (POLAROID): duck_depth=-0.457 outside [0.08, 0.8]
- seed 8 (FLOATING): structure_cycle_stripe=0.076 outside [0.1, 1.0]
- seed 14 (FLOATING): structure_cycle_stripe=0.046 outside [0.1, 1.0]; structure_novelty_mean=0.113 outside [0.22, 0.75]; band_low=0.845 outside [0.5, 0.84]
- seed 42 (POLAROID): duck_depth=-0.031 outside [0.08, 0.8]; band_low=0.843 outside [0.5, 0.84]
- seed 11 (FLOATING): structure_cycle_stripe=0.072 outside [0.1, 1.0]; onsets_per_second=4.520 outside [1.8, 4.5]; band_low=0.850 outside [0.5, 0.84]
- seed 28 (WINDOWLIGHT): rest_ratio=0.014 outside [0.02, 0.3]; band_low=0.841 outside [0.5, 0.84]
- seed 18 (WINDOWLIGHT): rest_ratio=0.003 outside [0.02, 0.3]; band_low=0.879 outside [0.5, 0.84]
- seed 9 (WINDOWLIGHT): rest_ratio=0.009 outside [0.02, 0.3]; band_low=0.875 outside [0.5, 0.84]
- seed 0 (FLOATING): structure_cycle_stripe=0.056 outside [0.1, 1.0]; structure_novelty_mean=0.066 outside [0.22, 0.75]
- seed 1 (FLOATING): structure_cycle_stripe=0.069 outside [0.1, 1.0]; structure_novelty_mean=0.143 outside [0.22, 0.75]; duck_depth=0.077 outside [0.08, 0.8]; band_low=0.887 outside [0.5, 0.84]
- seed 26 (WINDOWLIGHT): duck_depth=-0.018 outside [0.08, 0.8]; band_low=0.889 outside [0.5, 0.84]
- seed 46 (FLOATING): structure_cycle_stripe=0.092 outside [0.1, 1.0]; structure_novelty_mean=0.133 outside [0.22, 0.75]; band_low=0.872 outside [0.5, 0.84]
- seed 22 (WINDOWLIGHT): structure_cycle_stripe=0.089 outside [0.1, 1.0]; structure_novelty_mean=0.191 outside [0.22, 0.75]; band_low=0.877 outside [0.5, 0.84]
- seed 27 (POLAROID): structure_cycle_stripe=0.028 outside [0.1, 1.0]; structure_novelty_mean=0.086 outside [0.22, 0.75]; onsets_per_second=4.624 outside [1.8, 4.5]
- seed 15 (WINDOWLIGHT): band_low=0.878 outside [0.5, 0.84]
- seed 13 (WINDOWLIGHT): band_low=0.843 outside [0.5, 0.84]
- seed 7 (POLAROID): structure_novelty_mean=0.203 outside [0.22, 0.75]; band_low=0.872 outside [0.5, 0.84]
- seed 29 (WINDOWLIGHT): rest_ratio=0.019 outside [0.02, 0.3]; band_low=0.866 outside [0.5, 0.84]
- seed 16 (FLOATING): structure_cycle_stripe=0.055 outside [0.1, 1.0]; structure_novelty_mean=0.116 outside [0.22, 0.75]
- seed 35 (FLOATING): structure_novelty_mean=0.157 outside [0.22, 0.75]; band_low=0.872 outside [0.5, 0.84]
- seed 17 (POLAROID): band_low=0.880 outside [0.5, 0.84]
- seed 33 (FLOATING): structure_novelty_mean=0.168 outside [0.22, 0.75]
- seed 32 (FLOATING): structure_cycle_stripe=0.016 outside [0.1, 1.0]; structure_novelty_mean=0.049 outside [0.22, 0.75]; band_low=0.843 outside [0.5, 0.84]
- seed 44 (POLAROID): structure_novelty_mean=0.199 outside [0.22, 0.75]
- seed 37 (POLAROID): structure_novelty_mean=0.207 outside [0.22, 0.75]; duck_depth=0.001 outside [0.08, 0.8]
- seed 31 (WINDOWLIGHT): band_low=0.860 outside [0.5, 0.84]
- seed 20 (POLAROID): band_low=0.877 outside [0.5, 0.84]
- seed 43 (FLOATING):   - keys delay 70613us exceeds the 65461us pocket at step 480;   - keys delay 70326us exceeds the 65461us pocket at step 496

## Listen

- Blind deck: `npm run dev`, open <http://localhost:5173/judge>.
  Consecutive trials alternate engines; verdicts log per-engine.
- Direct WAVs: `target/candidates/seed-N/mix.wav` (96 s, 5 modules).
- Visual reports: `target/candidates/seed-N/report.png`.
