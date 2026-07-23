#!/usr/bin/env python3
"""Instrumented listening: turn a render into features, images, and distances.

GenAI cannot reliably hear, so this tool converts audio into artifacts it can
reason about: a feature vector (rhythm, spectrum, harmony, structure, space),
a visual report (mel spectrogram, self-similarity matrix, chromagram, onset
grid), and z-score distances against a reference corpus of approved lo-fi.

Usage:
  scorecard.py analyze mix.wav --bpm 78 --out target/scorecard/name
  scorecard.py corpus features_a.json features_b.json ... --out corpus.json
  scorecard.py compare features.json corpus.json
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import librosa
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
import soundfile as sf

MAJOR = np.array([1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 1], dtype=np.float32)


def decibels(value: float) -> float:
    return 20.0 * math.log10(max(value, 1e-9))


def load_mono(path: str) -> tuple[np.ndarray, int]:
    audio, rate = sf.read(path, always_2d=True)
    return audio.mean(axis=1).astype(np.float32), rate


def scale_consistency(chroma: np.ndarray) -> float:
    energy = chroma.sum(axis=1)
    total = float(energy.sum())
    if total < 1e-6:
        return 0.0
    return max(float(np.dot(energy, np.roll(MAJOR, tonic))) / total for tonic in range(12))


def swing_ratio(onset_env: np.ndarray, rate: int, hop: int, bpm: float) -> float:
    """Median off-beat eighth placement as a fraction of the eighth interval.

    0.5 is straight; lo-fi pockets sit around 0.55-0.65.
    """
    beat_frames = 60.0 * rate / bpm / hop
    if beat_frames < 4:
        return 0.5
    peaks = librosa.util.peak_pick(
        onset_env, pre_max=3, post_max=3, pre_avg=6, post_avg=6, delta=0.35, wait=2
    )
    if len(peaks) < 8:
        return 0.5
    positions = (peaks % beat_frames) / beat_frames
    offbeats = positions[(positions > 0.30) & (positions < 0.85)]
    if len(offbeats) < 4:
        return 0.5
    return float(np.median(offbeats))


def chroma_entropy(chroma: np.ndarray) -> float:
    weights = chroma.sum(axis=1)
    weights = weights / max(float(weights.sum()), 1e-9)
    return float(-sum(x * math.log(x + 1e-12) for x in weights))


def rest_ratio(audio: np.ndarray, rate: int) -> float:
    frame = int(rate * 0.1)
    frames = audio[: len(audio) // frame * frame].reshape(-1, frame)
    rms = np.sqrt((frames**2).mean(axis=1))
    track = np.sqrt((audio**2).mean())
    return float((rms < track * 10 ** (-18 / 20)).mean())


def band_energy(audio: np.ndarray, rate: int) -> dict[str, float]:
    spectrum = np.abs(np.fft.rfft(audio)) ** 2
    freqs = np.fft.rfftfreq(len(audio), 1 / rate)
    total = spectrum.sum() + 1e-12
    bands = {"low": (20, 150), "lowmid": (150, 800), "mid": (800, 3000), "high": (3000, 12000)}
    return {
        name: float(spectrum[(freqs >= lo) & (freqs < hi)].sum() / total)
        for name, (lo, hi) in bands.items()
    }


def phrase_windows(audio: np.ndarray, rate: int, seconds: float) -> dict[str, object]:
    frames = int(seconds * rate)
    windows = []
    for index in range(len(audio) // frames):
        chunk = audio[index * frames : (index + 1) * frames]
        env = librosa.onset.onset_strength(y=chunk, sr=rate, hop_length=512)
        onsets = librosa.onset.onset_detect(onset_envelope=env, sr=rate, hop_length=512)
        windows.append(
            {
                "rms_dbfs": decibels(float(np.sqrt((chunk**2).mean()))),
                "onsets_per_second": len(onsets) / seconds,
            }
        )
    if len(windows) < 2:
        return {"windows": windows, "loudness_range_db": 0.0, "onset_density_range": 0.0}
    loud = [w["rms_dbfs"] for w in windows]
    dens = [w["onsets_per_second"] for w in windows]
    return {
        "windows": windows,
        "loudness_range_db": max(loud) - min(loud),
        "onset_density_range": max(dens) - min(dens),
    }


def structure(
    audio: np.ndarray, harmonic: np.ndarray, rate: int, bpm: float
) -> tuple[dict[str, float], np.ndarray]:
    hop = 512
    chroma = librosa.feature.chroma_cqt(y=harmonic, sr=rate, hop_length=hop, fmin=110.0)
    mfcc = librosa.feature.mfcc(y=audio, sr=rate, hop_length=hop, n_mfcc=13)[1:]
    _, beats = librosa.beat.beat_track(y=audio, sr=rate, hop_length=hop, start_bpm=bpm)
    if len(beats) < 8:
        return {"repetition": 0.0, "novelty_mean": 0.0, "diagonal_contrast": 0.0}, np.zeros((2, 2))
    sync = np.vstack(
        [
            librosa.util.sync(chroma, beats, aggregate=np.median),
            librosa.util.sync(mfcc, beats, aggregate=np.median),
        ]
    )
    sync = librosa.util.normalize(sync, axis=0)
    ssm = np.dot(sync.T, sync)
    n = ssm.shape[0]
    # Repetition: strongest off-diagonal stripe at lags of 4..64 beats.
    lags = {}
    for lag in range(4, min(64, n - 4)):
        lags[lag] = float(np.mean(np.diag(ssm, k=lag)))
    best_lag = max(lags, key=lags.get) if lags else 0
    off_diag = ssm[np.triu_indices(n, k=4)]
    mean_off = float(np.mean(off_diag))
    features = {
        "repetition": float(lags.get(best_lag, 0.0)),
        "repetition_lag_beats": float(best_lag),
        "diagonal_contrast": float(lags.get(best_lag, 0.0) - mean_off),
        # The musically meaningful stripes: the 4-bar cycle and 8-bar phrase.
        "cycle_stripe": float(lags.get(16, 0.0) - mean_off),
        "phrase_stripe": float(lags.get(32, 0.0) - mean_off),
        "novelty_mean": float(np.mean(np.abs(np.diff(np.diag(ssm, k=1))))),
    }
    return features, ssm


def analyze(path: str, bpm: float, out: Path) -> dict[str, object]:
    audio, rate = load_mono(path)
    hop = 512
    onset_env = librosa.onset.onset_strength(y=audio, sr=rate, hop_length=hop)
    tempo, _ = librosa.beat.beat_track(onset_envelope=onset_env, sr=rate, hop_length=hop, start_bpm=bpm)
    # Harmony metrics read the harmonic residue only: percussion and the
    # dominant sub-bass otherwise smear every pitch-class bin.
    harmonic = librosa.effects.harmonic(audio, margin=3.0)
    chroma = librosa.feature.chroma_cqt(y=harmonic, sr=rate, hop_length=hop, fmin=110.0)
    centroid = librosa.feature.spectral_centroid(y=audio, sr=rate, hop_length=hop)
    onsets = librosa.onset.onset_detect(onset_envelope=onset_env, sr=rate, hop_length=hop)
    struct, ssm = structure(audio, harmonic, rate, bpm)

    features: dict[str, object] = {
        "path": str(path),
        "duration_s": len(audio) / rate,
        "rms_dbfs": decibels(float(np.sqrt((audio**2).mean()))),
        "peak_dbfs": decibels(float(np.abs(audio).max())),
        "tempo_bpm": float(np.atleast_1d(tempo)[0]),
        "onsets_per_second": len(onsets) * rate / hop / max(len(onset_env), 1),
        "swing": swing_ratio(onset_env, rate, hop, bpm),
        "rest_ratio": rest_ratio(audio, rate),
        "scale_consistency": scale_consistency(chroma),
        "chroma_entropy": chroma_entropy(chroma),
        "spectral_centroid_hz": float(centroid.mean()),
        **{f"band_{k}": v for k, v in band_energy(audio, rate).items()},
        **{f"structure_{k}": v for k, v in struct.items()},
        "phrases": phrase_windows(audio, rate, 24.0),
    }

    out.mkdir(parents=True, exist_ok=True)
    render_report(audio, rate, hop, chroma, ssm, onset_env, features, out)
    (out / "features.json").write_text(json.dumps(features, indent=2))
    return features


def render_report(audio, rate, hop, chroma, ssm, onset_env, features, out: Path) -> None:
    fig, axes = plt.subplots(4, 1, figsize=(14, 16))
    mel = librosa.power_to_db(
        librosa.feature.melspectrogram(y=audio, sr=rate, hop_length=hop, n_mels=96), ref=np.max
    )
    librosa.display.specshow(mel, sr=rate, hop_length=hop, x_axis="time", y_axis="mel", ax=axes[0])
    axes[0].set_title(
        f"mel | rms {features['rms_dbfs']:.1f} dBFS | tempo {features['tempo_bpm']:.1f} "
        f"| swing {features['swing']:.2f} | rest {features['rest_ratio']:.2f}"
    )
    librosa.display.specshow(chroma, x_axis="time", y_axis="chroma", hop_length=hop, sr=rate, ax=axes[1])
    axes[1].set_title(f"chroma | scale consistency {features['scale_consistency']:.3f}")
    axes[2].imshow(ssm, origin="lower", aspect="auto", cmap="magma")
    axes[2].set_title(
        f"beat-synced self-similarity | repetition {features['structure_repetition']:.2f} "
        f"@ {features.get('structure_repetition_lag_beats', 0):.0f} beats"
    )
    times = np.arange(len(onset_env)) * hop / rate
    axes[3].plot(times, onset_env, linewidth=0.6)
    axes[3].set_xlim(0, times[-1] if len(times) else 1)
    axes[3].set_title(f"onset envelope | {features['onsets_per_second']:.2f} onsets/s")
    fig.tight_layout()
    fig.savefig(out / "report.png", dpi=110)
    plt.close(fig)


NUMERIC_KEYS = [
    "rms_dbfs",
    "onsets_per_second",
    "swing",
    "rest_ratio",
    "scale_consistency",
    "spectral_centroid_hz",
    "band_low",
    "band_lowmid",
    "band_mid",
    "band_high",
    "structure_repetition",
    "structure_diagonal_contrast",
    "structure_cycle_stripe",
    "structure_novelty_mean",
]


def build_corpus(feature_files: list[str], out: str) -> None:
    rows = [json.loads(Path(f).read_text()) for f in feature_files]
    stats = {}
    for key in NUMERIC_KEYS:
        values = [float(row[key]) for row in rows if key in row]
        spread = float(np.std(values))
        stats[key] = {
            "mean": float(np.mean(values)),
            # Floor the spread so a tiny corpus does not produce absurd z-scores.
            "std": max(spread, abs(float(np.mean(values))) * 0.08 + 1e-3),
        }
    Path(out).write_text(json.dumps({"tracks": len(rows), "stats": stats}, indent=2))
    print(f"{out}: corpus over {len(rows)} tracks")


def compare(features_file: str, corpus_file: str) -> int:
    features = json.loads(Path(features_file).read_text())
    corpus = json.loads(Path(corpus_file).read_text())["stats"]
    rows = []
    for key in NUMERIC_KEYS:
        if key not in features or key not in corpus:
            continue
        value = float(features[key])
        z = (value - corpus[key]["mean"]) / corpus[key]["std"]
        rows.append((key, value, corpus[key]["mean"], z))
    rows.sort(key=lambda r: -abs(r[3]))
    distance = float(np.sqrt(np.mean([r[3] ** 2 for r in rows])))
    print(f"corpus distance (rms z): {distance:.2f}")
    for key, value, mean, z in rows:
        flag = " <-- off" if abs(z) > 2.0 else ""
        print(f"  {key:32s} {value:10.3f}  ref {mean:10.3f}  z {z:+5.2f}{flag}")
    result = {"distance": distance, "rows": [list(r) for r in rows]}
    Path(features_file).with_suffix(".compare.json").write_text(json.dumps(result, indent=2))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    p_analyze = sub.add_parser("analyze")
    p_analyze.add_argument("wav")
    p_analyze.add_argument("--bpm", type=float, default=78.0)
    p_analyze.add_argument("--out", required=True)
    p_corpus = sub.add_parser("corpus")
    p_corpus.add_argument("features", nargs="+")
    p_corpus.add_argument("--out", required=True)
    p_compare = sub.add_parser("compare")
    p_compare.add_argument("features")
    p_compare.add_argument("corpus")
    args = parser.parse_args()

    if args.command == "analyze":
        features = analyze(args.wav, args.bpm, Path(args.out))
        print(json.dumps({k: v for k, v in features.items() if k != "phrases"}, indent=2))
    elif args.command == "corpus":
        build_corpus(args.features, args.out)
    elif args.command == "compare":
        return compare(args.features, args.corpus)
    return 0


if __name__ == "__main__":
    sys.exit(main())
