#!/usr/bin/env python3
"""Measure a rendered mix and optionally score its audible character with CLAP."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

import librosa
import numpy as np
import soundfile as sf


POSITIVE_LABELS = (
    "a professionally produced lo-fi hip hop instrumental",
    "a coherent mellow boom bap beat",
    "warm dusty study music with a stable groove",
)
NEGATIVE_LABELS = (
    "out of tune music with clashing notes",
    "random disconnected musical samples",
    "an amateur unfinished music demo",
    "isolated sound effects without a song",
    "harsh noisy distorted audio",
    "silence",
)


def decibels(value: float) -> float:
    return 20.0 * math.log10(max(value, 1e-9))


def scale_consistency(chroma: np.ndarray, rate: int, hop: int) -> float:
    frames_per_phrase = max(1, round(12.8 * rate / hop))
    scale = np.zeros(12, dtype=np.float32)
    scale[[0, 2, 4, 5, 7, 9, 11]] = 1.0
    ratios = []
    for start in range(0, chroma.shape[1], frames_per_phrase):
        energy = chroma[:, start : start + frames_per_phrase].sum(axis=1)
        total = float(energy.sum())
        if total < 1e-6:
            continue
        best = max(float(np.dot(energy, np.roll(scale, tonic))) / total for tonic in range(12))
        ratios.append(best)
    return float(np.mean(ratios)) if ratios else 0.0


def clap_scores(audio: np.ndarray) -> dict[str, object]:
    try:
        from transformers import pipeline
    except ImportError:
        return {"available": False, "reason": "transformers is not installed"}
    classifier = pipeline(
        "zero-shot-audio-classification", model="laion/clap-htsat-unfused", device="cpu"
    )
    labels = list(POSITIVE_LABELS + NEGATIVE_LABELS)
    results = classifier(audio, candidate_labels=labels)
    scores = {str(item["label"]): float(item["score"]) for item in results}
    positive = sum(scores[label] for label in POSITIVE_LABELS)
    negative = sum(scores[label] for label in NEGATIVE_LABELS)
    return {
        "available": True,
        "positive_probability": positive,
        "negative_probability": negative,
        "labels": results,
    }


def aesthetics_scores(audio: np.ndarray, rate: int) -> dict[str, object]:
    try:
        import torch
        from audiobox_aesthetics.infer import initialize_predictor
    except ImportError:
        return {"available": False, "reason": "audiobox_aesthetics is not installed"}
    duration = len(audio) / rate
    starts = sorted({min(5.0, max(0.0, duration - 8.0)), max(0.0, duration / 2.0 - 4.0), max(0.0, duration - 10.0)})
    items = []
    for start in starts:
        segment = audio[int(start * rate) : int(min(duration, start + 8.0) * rate)]
        items.append({"path": torch.from_numpy(segment.copy()).unsqueeze(0), "sample_rate": rate})
    scores = initialize_predictor().forward(items)
    mean = {key: sum(float(score[key]) for score in scores) / len(scores) for key in scores[0]}
    return {"available": True, "windows": scores, "mean": mean}


def evaluate(path: Path, use_clap: bool, use_aesthetics: bool) -> dict[str, object]:
    stereo, rate = sf.read(path, always_2d=True, dtype="float32")
    if stereo.shape[1] == 1:
        stereo = np.repeat(stereo, 2, axis=1)
    mono = stereo.mean(axis=1)
    peak = float(np.max(np.abs(stereo)))
    rms = float(np.sqrt(np.mean(mono * mono)))
    hop = 512
    harmonic, _ = librosa.effects.hpss(mono)
    chroma = librosa.feature.chroma_cqt(y=harmonic, sr=rate, hop_length=hop)
    tempo, beats = librosa.beat.beat_track(y=mono, sr=rate, start_bpm=80.0)
    beat_times = librosa.frames_to_time(beats, sr=rate, hop_length=hop)
    beat_intervals = np.diff(beat_times)
    beat_interval_jitter_ms = (
        float(np.percentile(np.abs(beat_intervals - np.median(beat_intervals)), 90) * 1_000.0)
        if len(beat_intervals)
        else float("inf")
    )
    onsets = librosa.onset.onset_detect(y=mono, sr=rate, units="time", backtrack=True)
    left_rms = float(np.sqrt(np.mean(stereo[:, 0] ** 2)))
    right_rms = float(np.sqrt(np.mean(stereo[:, 1] ** 2)))
    correlation = float(np.corrcoef(stereo[:, 0], stereo[:, 1])[0, 1])
    metrics = {
        "duration_seconds": len(mono) / rate,
        "sample_rate": rate,
        "peak_dbfs": decibels(peak),
        "rms_dbfs": decibels(rms),
        "crest_db": decibels(peak / max(rms, 1e-9)),
        "tempo_bpm": float(np.asarray(tempo).item()),
        "beat_count": len(beats),
        "beat_interval_jitter_ms": beat_interval_jitter_ms,
        "onsets_per_second": len(onsets) / max(len(mono) / rate, 1e-9),
        "scale_consistency": scale_consistency(chroma, rate, hop),
        "stereo_correlation": correlation,
        "channel_balance_db": abs(decibels(left_rms) - decibels(right_rms)),
        "clipped_samples": int(np.count_nonzero(np.abs(stereo) >= 0.999)),
    }
    checks = {
        "audible_level": -30.0 <= metrics["rms_dbfs"] <= -10.0,
        "headroom": -18.0 <= metrics["peak_dbfs"] <= -1.0,
        "dynamic_range": 8.0 <= metrics["crest_db"] <= 24.0,
        "tempo": 68.0 <= metrics["tempo_bpm"] <= 86.0
        or 34.0 <= metrics["tempo_bpm"] <= 43.0,
        "rhythm_consistency": metrics["beat_interval_jitter_ms"] <= 20.0,
        "tonal_center": metrics["scale_consistency"] >= 0.61,
        "stereo_output": metrics["channel_balance_db"] <= 3.0,
        "no_clipping": metrics["clipped_samples"] == 0,
    }
    report: dict[str, object] = {"file": str(path), "metrics": metrics, "checks": checks}
    if use_clap:
        report["clap"] = clap_scores(mono)
    if use_aesthetics:
        report["aesthetics"] = aesthetics_scores(mono, rate)
    clap = report.get("clap", {})
    semantic_pass = not use_clap or (
        bool(clap.get("available")) and float(clap.get("positive_probability", 0.0)) >= 0.55
    )
    aesthetics = report.get("aesthetics", {})
    means = aesthetics.get("mean", {}) if isinstance(aesthetics, dict) else {}
    aesthetics_pass = not use_aesthetics or (
        bool(aesthetics.get("available"))
        and float(means.get("CE", 0.0)) >= 7.0
        and float(means.get("PQ", 0.0)) >= 7.0
    )
    report["verdict"] = (
        "PASS" if all(checks.values()) and semantic_pass and aesthetics_pass else "FAIL"
    )
    return report


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("audio", type=Path)
    parser.add_argument("--clap", action="store_true")
    parser.add_argument("--aesthetics", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = evaluate(args.audio, args.clap, args.aesthetics)
    rendered = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered)
    print(rendered, end="")


if __name__ == "__main__":
    main()
