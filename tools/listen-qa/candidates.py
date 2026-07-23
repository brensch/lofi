#!/usr/bin/env python3
"""The overnight iteration driver for symbolic candidates.

For each seed: dump the symbolic score, run the property gates, render the
exact browser path, run the scorecard, and compare against the reference
corpus. Emits a ranked summary so a human (or the next iteration) can see at
a glance which candidates are deck-worthy and why the rest failed.

Usage:
  candidates.py --seeds 1,2,3,7,11 --bpm 76 --corpus target/scorecard/corpus.json
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PYTHON = sys.executable

# Absolute craft windows measured from approved references and production
# practice. A candidate must land inside every window; corpus z-scores rank
# the survivors but cannot rescue a window failure.
CRAFT_WINDOWS = {
    "swing": (0.52, 0.68),
    "rest_ratio": (0.02, 0.30),
    "scale_consistency": (0.60, 1.01),
    "structure_cycle_stripe": (0.10, 1.0),
    "structure_novelty_mean": (0.25, 0.75),
    "onsets_per_second": (1.8, 4.5),
}


def run(command: list[str], **kwargs) -> subprocess.CompletedProcess:
    return subprocess.run(command, cwd=ROOT, capture_output=True, text=True, **kwargs)


def evaluate_seed(seed: int, bpm: int, corpus: str | None, phrases: int, duration: int) -> dict:
    out = ROOT / "target" / "candidates" / f"seed-{seed}"
    out.mkdir(parents=True, exist_ok=True)
    result: dict[str, object] = {"seed": seed, "bpm": bpm}

    score_path = out / "score.jsonl"
    dump = run(
        ["cargo", "run", "-q", "-p", "lofi-core", "--example", "score_dump", "--",
         str(seed), str(phrases), str(bpm * 1000)],
    )
    score_path.write_text(dump.stdout)
    meta = json.loads(dump.stdout.splitlines()[0])["meta"]
    result["signature"] = meta["signature"]
    result["worst_repitch"] = meta["worst_repitch"]

    gates = run([PYTHON, "tools/listen-qa/symbolic_gates.py", str(score_path)])
    result["symbolic_gates"] = "PASS" if gates.returncode == 0 else "FAIL"
    result["gate_output"] = gates.stdout.strip().splitlines()[-8:]
    if gates.returncode != 0:
        return result

    wav = out / "mix.wav"
    render = run(
        ["node", "tools/listen-qa/render.mjs", "--seed", str(seed), "--nodes", "5",
         "--duration", str(duration), "--bpm", str(bpm), "--engine", "symbolic",
         "--output", str(wav)],
    )
    if render.returncode != 0:
        result["render"] = "FAIL"
        result["render_error"] = render.stderr[-400:]
        return result

    analyze = run(
        [PYTHON, "tools/listen-qa/scorecard.py", "analyze", str(wav),
         "--bpm", str(bpm), "--out", str(out)],
    )
    if analyze.returncode != 0:
        result["scorecard"] = "FAIL"
        result["scorecard_error"] = analyze.stderr[-400:]
        return result
    features = json.loads((out / "features.json").read_text())
    result["features"] = {
        key: features[key]
        for key in (
            "rms_dbfs", "swing", "rest_ratio", "scale_consistency", "onsets_per_second",
            "structure_cycle_stripe", "structure_novelty_mean", "structure_repetition_lag_beats",
            "band_low", "spectral_centroid_hz",
        )
    }

    window_failures = []
    for key, (lo, hi) in CRAFT_WINDOWS.items():
        value = float(features[key])
        if not lo <= value <= hi:
            window_failures.append(f"{key}={value:.3f} outside [{lo}, {hi}]")
    result["craft_windows"] = window_failures or "PASS"

    if corpus:
        compare = run(
            [PYTHON, "tools/listen-qa/scorecard.py", "compare",
             str(out / "features.json"), corpus],
        )
        first = compare.stdout.splitlines()[0] if compare.stdout else ""
        result["corpus"] = first
        compare_path = out / "features.compare.json"
        if compare_path.exists():
            result["corpus_distance"] = json.loads(compare_path.read_text())["distance"]
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seeds", required=True, help="comma-separated seed list")
    parser.add_argument("--bpm", type=int, default=78)
    parser.add_argument("--phrases", type=int, default=4)
    parser.add_argument("--duration", type=int, default=96)
    parser.add_argument("--corpus", default=None)
    parser.add_argument("--out", default="target/candidates/summary.json")
    args = parser.parse_args()

    results = []
    for seed in [int(s) for s in args.seeds.split(",")]:
        result = evaluate_seed(seed, args.bpm, args.corpus, args.phrases, args.duration)
        results.append(result)
        passed = (
            result.get("symbolic_gates") == "PASS"
            and result.get("craft_windows") == "PASS"
        )
        verdict = "PASS" if passed else "fail"
        distance = result.get("corpus_distance")
        distance_text = f" corpus={distance:.2f}" if isinstance(distance, float) else ""
        print(f"seed {seed}: {verdict} ({result.get('signature')}){distance_text}", flush=True)
        if not passed:
            for line in (result.get("gate_output") or [])[:4]:
                print(f"    {line}")
            if isinstance(result.get("craft_windows"), list):
                for line in result["craft_windows"]:
                    print(f"    {line}")

    ranked = sorted(
        results,
        key=lambda r: (
            r.get("symbolic_gates") != "PASS",
            r.get("craft_windows") != "PASS",
            r.get("corpus_distance", 99.0),
        ),
    )
    Path(ROOT / args.out).parent.mkdir(parents=True, exist_ok=True)
    Path(ROOT / args.out).write_text(json.dumps(ranked, indent=2))
    survivors = [
        r["seed"] for r in ranked
        if r.get("symbolic_gates") == "PASS" and r.get("craft_windows") == "PASS"
    ]
    print(f"\nsurvivors: {survivors}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
