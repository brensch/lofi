#!/usr/bin/env python3
"""Check isolated bass renders for balance and abrasive upper-band energy."""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import librosa
import numpy as np
import soundfile as sf


def decibels(value: float) -> float:
    return 20.0 * math.log10(max(value, 1e-9))


def analyze(path: Path) -> dict[str, object]:
    stereo, rate = sf.read(path, always_2d=True, dtype="float32")
    mono = stereo.mean(axis=1)
    spectrum = np.abs(librosa.stft(mono, n_fft=4096, hop_length=1024)) ** 2
    frequencies = librosa.fft_frequencies(sr=rate, n_fft=4096)
    total = float(spectrum.sum()) + 1e-12
    return {
        "file": str(path),
        "peak_dbfs": decibels(float(np.max(np.abs(mono)))),
        "rms_dbfs": decibels(float(np.sqrt(np.mean(mono * mono)))),
        "energy_above_800_hz": float(spectrum[frequencies >= 800].sum()) / total,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("audio", type=Path, nargs="+")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    renders = [analyze(path) for path in args.audio]
    levels = [float(render["rms_dbfs"]) for render in renders]
    report = {
        "renders": renders,
        "metrics": {
            "rms_spread_db": max(levels) - min(levels),
            "max_upper_band_energy": max(
                float(render["energy_above_800_hz"]) for render in renders
            ),
        },
    }
    report["checks"] = {
        "consistent_level": report["metrics"]["rms_spread_db"] <= 1.5,
        "soft_upper_band": report["metrics"]["max_upper_band_energy"] <= 0.001,
        "headroom": all(float(render["peak_dbfs"]) <= -10.0 for render in renders),
        "present": all(float(render["rms_dbfs"]) >= -36.0 for render in renders),
    }
    report["verdict"] = "PASS" if all(report["checks"].values()) else "FAIL"
    rendered = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered)
    print(rendered, end="")
    if report["verdict"] != "PASS":
        sys.exit(1)


if __name__ == "__main__":
    main()
