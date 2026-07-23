#!/usr/bin/env python3
"""Cross-candidate musical diversity: how different the catalogue sounds.

The loop engine can only rearrange three harvested scenes; the symbolic
composer claims unlimited harmonic ground. This measures that claim: mean
pairwise distance across candidates' chroma profiles (what keys/harmony the
listener actually hears) and tempo-independent craft features.

Usage: diversity.py label_a a1/features.json a2/... -- label_b b1/features.json ...
"""

from __future__ import annotations

import json
import sys
from itertools import combinations
from pathlib import Path

import numpy as np


def profile(path: str) -> np.ndarray:
    features = json.loads(Path(path).read_text())
    chroma = np.array(features["chroma_profile"], dtype=float)
    # Rotation-sensitive on purpose: a track in a different key IS variety.
    return chroma / max(chroma.sum(), 1e-9)


def group_diversity(paths: list[str]) -> float:
    profiles = [profile(p) for p in paths]
    if len(profiles) < 2:
        return 0.0
    distances = [
        float(np.linalg.norm(a - b)) for a, b in combinations(profiles, 2)
    ]
    return float(np.mean(distances))


def main() -> int:
    groups: list[tuple[str, list[str]]] = []
    label, bucket = None, []
    for arg in sys.argv[1:]:
        if arg == "--":
            if label:
                groups.append((label, bucket))
            label, bucket = None, []
        elif label is None:
            label = arg
        else:
            bucket.append(arg)
    if label:
        groups.append((label, bucket))

    for name, paths in groups:
        print(f"{name}: {group_diversity(paths):.4f} mean pairwise chroma distance "
              f"({len(paths)} tracks)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
