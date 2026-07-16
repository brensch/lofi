#!/usr/bin/env python3
"""Measure reusable musical signatures from an AI-generated reference.

This is an offline curation tool, not a firmware dependency. It expects a
Demucs six-stem directory and optional Basic Pitch CSV files, then writes a
small JSON report suitable for reviewing before material is transcribed into
the fixed-size no_std content catalogue.
"""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path

import librosa
import numpy as np
import soundfile as sf


STEM_NAMES = ("bass", "drums", "guitar", "piano", "other", "vocals")


def percentile(values: np.ndarray, value: float) -> float:
    return round(float(np.percentile(values, value)), 6)


def audio_signature(path: Path, tempo: float) -> dict[str, object]:
    audio, rate = sf.read(path, always_2d=True, dtype="float32")
    mono = np.mean(audio, axis=1)
    peak = float(np.max(np.abs(mono)))
    rms = librosa.feature.rms(y=mono, frame_length=2048, hop_length=512)[0]
    centroid = librosa.feature.spectral_centroid(y=mono, sr=rate)[0]
    rolloff = librosa.feature.spectral_rolloff(y=mono, sr=rate, roll_percent=0.85)[0]
    active = rms > max(float(np.max(rms)) * 0.05, 1e-5)
    active_centroid = centroid[: len(active)][active[: len(centroid)]]
    active_rolloff = rolloff[: len(active)][active[: len(rolloff)]]
    onset_envelope = librosa.onset.onset_strength(y=mono, sr=rate, hop_length=256)
    onset_frames = librosa.onset.onset_detect(
        onset_envelope=onset_envelope,
        sr=rate,
        hop_length=256,
        backtrack=True,
        units="frames",
    )
    onset_times = librosa.frames_to_time(onset_frames, sr=rate, hop_length=256)
    step_seconds = 60.0 / tempo / 4.0
    offsets_ms = []
    for onset in onset_times:
        nearest = round(onset / step_seconds) * step_seconds
        offset = (onset - nearest) * 1000.0
        if abs(offset) <= step_seconds * 500.0:
            offsets_ms.append(offset)

    return {
        "sample_rate": rate,
        "duration_s": round(len(mono) / rate, 3),
        "peak": round(peak, 6),
        "rms_db": round(float(librosa.amplitude_to_db([np.median(rms)])[0]), 2),
        "rms_db_p90": round(float(librosa.amplitude_to_db([np.percentile(rms, 90)])[0]), 2),
        "spectral_centroid_hz": round(float(np.median(active_centroid)), 1),
        "spectral_rolloff_85_hz": round(float(np.median(active_rolloff)), 1),
        "onset_count": len(onset_times),
        "grid_offset_ms": {
            "median": percentile(np.asarray(offsets_ms), 50) if offsets_ms else 0.0,
            "p10": percentile(np.asarray(offsets_ms), 10) if offsets_ms else 0.0,
            "p90": percentile(np.asarray(offsets_ms), 90) if offsets_ms else 0.0,
        },
    }


def drum_signature(path: Path, tempo: float) -> dict[str, object]:
    audio, rate = sf.read(path, always_2d=True, dtype="float32")
    mono = np.mean(audio, axis=1)
    hop = 256
    envelope = librosa.onset.onset_strength(y=mono, sr=rate, hop_length=hop)
    frames = librosa.onset.onset_detect(
        onset_envelope=envelope,
        sr=rate,
        hop_length=hop,
        backtrack=True,
        units="frames",
    )
    times = librosa.frames_to_time(frames, sr=rate, hop_length=hop)
    step_seconds = 60.0 / tempo / 4.0
    patterns: dict[str, list[dict[str, object]]] = {"kick": [], "snare": [], "hat": []}

    for frame, onset in zip(frames, times):
        start = int(onset * rate)
        window = mono[start : start + int(rate * 0.09)]
        if len(window) < 32:
            continue
        spectrum = np.abs(np.fft.rfft(window * np.hanning(len(window)))) ** 2
        frequencies = np.fft.rfftfreq(len(window), 1.0 / rate)
        total = float(np.sum(spectrum)) + 1e-12
        low = float(np.sum(spectrum[frequencies < 180.0])) / total
        high = float(np.sum(spectrum[frequencies > 3500.0])) / total
        if low > 0.48:
            lane = "kick"
        elif high > 0.28:
            lane = "hat"
        else:
            lane = "snare"
        step_float = onset / step_seconds
        step = int(round(step_float))
        strength = float(envelope[min(int(frame), len(envelope) - 1)])
        patterns[lane].append(
            {
                "step": step,
                "step_in_4_bars": step % 64,
                "offset_ms": round((step_float - step) * step_seconds * 1000.0, 2),
                "strength": round(strength, 4),
            }
        )

    summary: dict[str, object] = {}
    for lane, events in patterns.items():
        offsets = np.asarray([event["offset_ms"] for event in events])
        histogram: dict[int, int] = {}
        for event in events:
            step = int(event["step_in_4_bars"])
            histogram[step] = histogram.get(step, 0) + 1
        summary[lane] = {
            "events": events,
            "common_steps": [
                {"step": step, "count": count}
                for step, count in sorted(histogram.items(), key=lambda item: (-item[1], item[0]))
            ],
            "offset_ms": {
                "median": percentile(offsets, 50) if len(offsets) else 0.0,
                "p10": percentile(offsets, 10) if len(offsets) else 0.0,
                "p90": percentile(offsets, 90) if len(offsets) else 0.0,
            },
        }
    return summary


def read_notes(path: Path, tempo: float, key_midi: int) -> dict[str, object]:
    notes = []
    with path.open(newline="") as source:
        for row in csv.DictReader(source):
            start = float(row["start_time_s"])
            end = float(row["end_time_s"])
            pitch = int(row["pitch_midi"])
            velocity = int(row["velocity"])
            if end <= start or pitch < 21 or pitch > 108:
                continue
            step_float = start * tempo * 4.0 / 60.0
            step = int(round(step_float))
            notes.append(
                {
                    "step": step,
                    "step_in_4_bars": step % 64,
                    "offset_ms": round((step_float - step) * 60_000.0 / tempo / 4.0, 2),
                    "duration_steps": round((end - start) * tempo * 4.0 / 60.0, 2),
                    "midi": pitch,
                    "degree_semitones": (pitch - key_midi) % 12,
                    "velocity": velocity,
                }
            )
    notes.sort(key=lambda note: (note["step"], -note["midi"]))
    first = {(n["step_in_4_bars"], n["midi"] % 12) for n in notes if n["step"] < 64}
    second = {
        (n["step_in_4_bars"], n["midi"] % 12)
        for n in notes
        if 64 <= n["step"] < 128
    }
    union = first | second
    return {
        "note_count": len(notes),
        "four_bar_return": round(len(first & second) / len(union), 3) if union else 0.0,
        "events": notes,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("stem_dir", type=Path)
    parser.add_argument("--notes", type=Path)
    parser.add_argument("--tempo", type=float, default=80.0)
    parser.add_argument("--key-midi", type=int, default=55, help="tonic MIDI pitch")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    report: dict[str, object] = {
        "source": str(args.stem_dir),
        "tempo": args.tempo,
        "key_midi": args.key_midi,
        "stems": {},
        "notes": {},
    }
    for name in STEM_NAMES:
        path = args.stem_dir / f"{name}.wav"
        if path.exists():
            report["stems"][name] = audio_signature(path, args.tempo)
            if name == "drums":
                report["drum_pattern"] = drum_signature(path, args.tempo)

    if args.notes:
        for path in sorted(args.notes.glob("*_basic_pitch.csv")):
            instrument = path.name.removesuffix("_basic_pitch.csv")
            report["notes"][instrument] = read_notes(path, args.tempo, args.key_midi)

    encoded = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded)
    else:
        print(encoded, end="")


if __name__ == "__main__":
    main()
