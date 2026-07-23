# Overnight symbolic engine report

Sweep: 48 symbolic seeds -> **13 deck-worthy candidates**.

Every candidate below passed all symbolic property gates (backbeat,
chord-root bass, diatonic approaches, rests, register, evolution,
pocket bounds) and every audio craft window, measured through the
exact browser AudioWorklet/WASM path.

## Candidates vs the loop engine

| track | swing | rest | scale | 4-bar stripe | novelty | onsets/s | corpus dist |
|---|---|---|---|---|---|---|---|
| symbolic seed 6 (WINDOWLIGHT) | 0.579 | 0.026 | 0.722 | 0.113 | 0.288 | 3.312 | 0.57 |
| symbolic seed 10 (WINDOWLIGHT) | 0.587 | 0.027 | 0.653 | 0.140 | 0.273 | 3.812 | 0.92 |
| symbolic seed 34 (POLAROID) | 0.580 | 0.075 | 0.675 | 0.133 | 0.316 | 3.562 | 0.94 |
| symbolic seed 2 (POLAROID) | 0.580 | 0.060 | 0.673 | 0.175 | 0.432 | 3.239 | 0.99 |
| symbolic seed 42 (POLAROID) | 0.580 | 0.075 | 0.639 | 0.165 | 0.360 | 3.468 | 1.17 |
| symbolic seed 13 (WINDOWLIGHT) | 0.583 | 0.036 | 0.666 | 0.201 | 0.442 | 4.052 | 1.26 |
| symbolic seed 26 (WINDOWLIGHT) | 0.583 | 0.037 | 0.637 | 0.143 | 0.329 | 3.468 | 1.45 |
| symbolic seed 17 (POLAROID) | 0.588 | 0.041 | 0.709 | 0.200 | 0.451 | 3.552 | 1.57 |
| symbolic seed 45 (WINDOWLIGHT) | 0.582 | 0.209 | 0.672 | 0.148 | 0.271 | 2.781 | 2.07 |
| symbolic seed 39 (POLAROID) | 0.578 | 0.233 | 0.670 | 0.323 | 0.252 | 2.906 | 2.88 |
| symbolic seed 19 (FLOATING) | 0.570 | 0.236 | 0.698 | 0.318 | 0.281 | 2.677 | 2.91 |
| symbolic seed 31 (WINDOWLIGHT) | 0.581 | 0.204 | 0.656 | 0.411 | 0.386 | 2.771 | 3.22 |
| symbolic seed 20 (POLAROID) | 0.587 | 0.223 | 0.684 | 0.372 | 0.433 | 2.750 | 3.26 |
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

- seed 5 (WINDOWLIGHT): structure_cycle_stripe=0.063 outside [0.1, 1.0]; structure_novelty_mean=0.178 outside [0.25, 0.75]
- seed 3 (POLAROID): structure_cycle_stripe=0.033 outside [0.1, 1.0]; structure_novelty_mean=0.121 outside [0.25, 0.75]
- seed 40 (WINDOWLIGHT): structure_cycle_stripe=0.058 outside [0.1, 1.0]; structure_novelty_mean=0.097 outside [0.25, 0.75]
- seed 12 (WINDOWLIGHT): structure_cycle_stripe=0.097 outside [0.1, 1.0]; structure_novelty_mean=0.212 outside [0.25, 0.75]
- seed 47 (WINDOWLIGHT): structure_cycle_stripe=0.039 outside [0.1, 1.0]; structure_novelty_mean=0.158 outside [0.25, 0.75]
- seed 4 (WINDOWLIGHT): structure_novelty_mean=0.159 outside [0.25, 0.75]
- seed 30 (WINDOWLIGHT): structure_cycle_stripe=0.066 outside [0.1, 1.0]; structure_novelty_mean=0.145 outside [0.25, 0.75]
- seed 23 (POLAROID): structure_cycle_stripe=0.028 outside [0.1, 1.0]; structure_novelty_mean=0.093 outside [0.25, 0.75]
- seed 36 (POLAROID): structure_cycle_stripe=0.043 outside [0.1, 1.0]; structure_novelty_mean=0.087 outside [0.25, 0.75]
- seed 41 (WINDOWLIGHT): structure_novelty_mean=0.201 outside [0.25, 0.75]
- seed 21 (FLOATING): structure_novelty_mean=0.237 outside [0.25, 0.75]
- seed 38 (POLAROID): structure_novelty_mean=0.204 outside [0.25, 0.75]
- seed 25 (WINDOWLIGHT): structure_cycle_stripe=0.099 outside [0.1, 1.0]
- seed 8 (FLOATING): structure_cycle_stripe=0.077 outside [0.1, 1.0]
- seed 14 (FLOATING): structure_cycle_stripe=0.054 outside [0.1, 1.0]; structure_novelty_mean=0.136 outside [0.25, 0.75]
- seed 11 (FLOATING): structure_cycle_stripe=0.075 outside [0.1, 1.0]
- seed 28 (WINDOWLIGHT): rest_ratio=0.015 outside [0.02, 0.3]
- seed 18 (WINDOWLIGHT): rest_ratio=0.003 outside [0.02, 0.3]
- seed 27 (POLAROID): structure_cycle_stripe=0.029 outside [0.1, 1.0]; structure_novelty_mean=0.098 outside [0.25, 0.75]; onsets_per_second=4.604 outside [1.8, 4.5]
- seed 0 (FLOATING): structure_cycle_stripe=0.050 outside [0.1, 1.0]; structure_novelty_mean=0.074 outside [0.25, 0.75]
- seed 16 (FLOATING): structure_cycle_stripe=0.054 outside [0.1, 1.0]; structure_novelty_mean=0.150 outside [0.25, 0.75]
- seed 1 (FLOATING): structure_cycle_stripe=0.066 outside [0.1, 1.0]; structure_novelty_mean=0.154 outside [0.25, 0.75]
- seed 9 (WINDOWLIGHT): rest_ratio=0.003 outside [0.02, 0.3]
- seed 22 (WINDOWLIGHT): structure_cycle_stripe=0.091 outside [0.1, 1.0]; structure_novelty_mean=0.239 outside [0.25, 0.75]
- seed 46 (FLOATING): structure_cycle_stripe=0.080 outside [0.1, 1.0]; structure_novelty_mean=0.153 outside [0.25, 0.75]
- seed 7 (POLAROID): structure_novelty_mean=0.239 outside [0.25, 0.75]
- seed 15 (WINDOWLIGHT): rest_ratio=0.017 outside [0.02, 0.3]
- seed 33 (FLOATING): structure_novelty_mean=0.188 outside [0.25, 0.75]
- seed 29 (WINDOWLIGHT): rest_ratio=0.013 outside [0.02, 0.3]
- seed 35 (FLOATING): structure_novelty_mean=0.168 outside [0.25, 0.75]
- seed 32 (FLOATING): structure_cycle_stripe=0.020 outside [0.1, 1.0]; structure_novelty_mean=0.066 outside [0.25, 0.75]; onsets_per_second=4.583 outside [1.8, 4.5]
- seed 44 (POLAROID): structure_cycle_stripe=0.036 outside [0.1, 1.0]; structure_novelty_mean=0.117 outside [0.25, 0.75]
- seed 37 (POLAROID): structure_novelty_mean=0.213 outside [0.25, 0.75]
- seed 24 (WINDOWLIGHT): structure_novelty_mean=0.783 outside [0.25, 0.75]
- seed 43 (FLOATING):   - keys delay 70613us exceeds the 65461us pocket at step 480;   - keys delay 70326us exceeds the 65461us pocket at step 496

## Listen

- Blind deck: `npm run dev`, open <http://localhost:5173/judge>.
  Consecutive trials alternate engines; verdicts log per-engine.
- Direct WAVs: `target/candidates/seed-N/mix.wav` (96 s, 5 modules).
- Visual reports: `target/candidates/seed-N/report.png`.
