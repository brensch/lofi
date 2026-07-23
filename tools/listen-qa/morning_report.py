#!/usr/bin/env python3
"""Compose the overnight results into one skimmable markdown report.

Reads the candidate sweep summary, the loop-engine reference features, and
the diversity numbers, and writes a report suitable for a first coffee.

Usage: morning_report.py --summary target/candidates/summary.json --out report.md
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

LOOP_REFS = {
    0: "target/scorecard/loop-seed0/features.json",
    1: "target/scorecard/loop-seed1/features.json",
    2: "target/scorecard/loop-seed2/features.json",
}

CRAFT_COLUMNS = [
    ("swing", "swing"),
    ("rest_ratio", "rest"),
    ("scale_consistency", "scale"),
    ("structure_cycle_stripe", "4-bar stripe"),
    ("structure_novelty_mean", "novelty"),
    ("onsets_per_second", "onsets/s"),
]


def feature_row(label: str, features: dict, distance: object = "") -> str:
    cells = [label]
    for key, _ in CRAFT_COLUMNS:
        cells.append(f"{float(features[key]):.3f}")
    cells.append(f"{distance:.2f}" if isinstance(distance, float) else str(distance))
    return "| " + " | ".join(cells) + " |"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--summary", default="target/candidates/summary.json")
    parser.add_argument("--out", default="target/candidates/MORNING_REPORT.md")
    args = parser.parse_args()

    results = json.loads(Path(args.summary).read_text())
    survivors = [
        r for r in results
        if r.get("symbolic_gates") == "PASS" and r.get("craft_windows") == "PASS"
    ]
    failed = [r for r in results if r not in survivors]

    lines = [
        "# Overnight symbolic engine report",
        "",
        f"Sweep: {len(results)} symbolic seeds -> **{len(survivors)} deck-worthy candidates**.",
        "",
        "Every candidate below passed all symbolic property gates (backbeat,",
        "chord-root bass, diatonic approaches, rests, register, evolution,",
        "pocket bounds) and every audio craft window, measured through the",
        "exact browser AudioWorklet/WASM path.",
        "",
        "## Candidates vs the loop engine",
        "",
        "| track | " + " | ".join(label for _, label in CRAFT_COLUMNS) + " | corpus dist |",
        "|" + "---|" * (len(CRAFT_COLUMNS) + 2),
    ]
    for r in survivors:
        lines.append(
            feature_row(
                f"symbolic seed {r['seed']} ({r['signature']})",
                r["features"],
                r.get("corpus_distance", ""),
            )
        )
    for seed, path in LOOP_REFS.items():
        ref = json.loads(Path(path).read_text())
        lines.append(feature_row(f"loop seed {seed} (reference)", ref, 0.0))
    lines += [
        "",
        "Reference craft windows: swing 0.52-0.68, rest 0.02-0.30, scale >= 0.60,",
        "4-bar stripe >= 0.10, novelty 0.25-0.75, onsets 1.8-4.5/s.",
        "",
        "Note: loop seeds 0 and 1 fail the 4-bar stripe and novelty windows",
        "themselves - the loop engine reaches its own structural bar only on",
        "seed 2. Corpus distance is the rms z-distance to the loop-render",
        "corpus, so the loop rows sit at ~0 by construction; for symbolic",
        "candidates lower means closer to the approved production envelope.",
        "",
        "## Failures and why",
        "",
    ]
    for r in failed:
        reason = []
        if r.get("symbolic_gates") != "PASS":
            reason.extend((r.get("gate_output") or [])[-2:])
        if isinstance(r.get("craft_windows"), list):
            reason.extend(r["craft_windows"])
        reason_text = "; ".join(str(x) for x in reason) or "unknown"
        lines.append(f"- seed {r['seed']} ({r.get('signature')}): {reason_text}")

    lines += [
        "",
        "## Listen",
        "",
        "- Blind deck: `npm run dev`, open <http://localhost:5173/judge>.",
        "  Consecutive trials alternate engines; verdicts log per-engine.",
        "- Direct WAVs: `target/candidates/seed-N/mix.wav` (96 s, 5 modules).",
        "- Visual reports: `target/candidates/seed-N/report.png`.",
        "",
    ]
    Path(args.out).write_text("\n".join(lines))
    print(args.out)
    return 0


if __name__ == "__main__":
    return_code = main()
    raise SystemExit(return_code)
