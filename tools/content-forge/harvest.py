#!/usr/bin/env python3
"""Build a compact sample-only catalogue from completed content-forge runs."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
import zlib
from dataclasses import asdict, dataclass
from pathlib import Path

import librosa
import numpy as np
import soundfile as sf


TARGET_RATE = 22_050
SILENCE_RMS_DB = -44.0
MAX_ONE_SHOTS_PER_STEM = 24
KIND_ORDER = (
    "kick",
    "snare",
    "hat",
    "drum_loop",
    "bass_note",
    "bass_loop",
    "lead_note",
    "melody_loop",
    "keys_note",
    "harmony_loop",
    "texture_loop",
)
ROOT_KINDS = ("bass_note", "lead_note", "keys_note")
ROOT_VARIANTS = 8
DRUM_KINDS = ("kick", "snare", "hat")


@dataclass
class Element:
    id: str
    kind: str
    source_id: str
    offset: int
    length: int
    sample_rate: int
    gain: float
    looped: bool
    bars: int
    phase: int
    bpm: int
    key: str
    mode: str
    progression: str
    root_semitone: int | None
    energy: int
    tags: list[str]


def dbfs_rms(audio: np.ndarray) -> float:
    rms = math.sqrt(float(np.mean(audio * audio)) + 1e-12)
    return 20.0 * math.log10(max(rms, 1e-9))


def load_mono(path: Path) -> np.ndarray:
    audio, rate = sf.read(path, always_2d=True, dtype="float32")
    mono = np.mean(audio, axis=1)
    if rate != TARGET_RATE:
        mono = librosa.resample(mono, orig_sr=rate, target_sr=TARGET_RATE)
    return np.asarray(mono, dtype=np.float32)


def normalize(audio: np.ndarray, peak_target: float = 0.78) -> tuple[np.ndarray, float]:
    peak = float(np.max(np.abs(audio))) if len(audio) else 0.0
    if peak < 1e-5:
        return audio, 1.0
    gain = min(peak_target / peak, 4.0)
    return np.clip(audio * gain, -0.98, 0.98), gain


def align_one_shot(audio: np.ndarray, kind: str) -> np.ndarray:
    """Place every attack at a predictable offset from the sample start."""
    if not len(audio):
        return audio
    peak = float(np.max(np.abs(audio)))
    if peak < 1e-5:
        return audio
    ratio = 0.08 if kind in DRUM_KINDS else 0.04
    threshold = max(0.005, peak * ratio)
    search_end = min(len(audio), int(TARGET_RATE * 0.25))
    crossings = np.flatnonzero(np.abs(audio[:search_end]) >= threshold)
    if not len(crossings):
        return audio
    pre_roll_seconds = 0.001 if kind in DRUM_KINDS else 0.003
    start = max(0, int(crossings[0]) - int(TARGET_RATE * pre_roll_seconds))
    return audio[start:]


def fade_one_shot(audio: np.ndarray, kind: str) -> np.ndarray:
    result = audio.copy()
    fade_seconds = 0.0005 if kind in DRUM_KINDS else 0.002
    fade_in = min(len(result), max(2, int(TARGET_RATE * fade_seconds)))
    fade_out = min(len(result), int(TARGET_RATE * 0.025))
    if fade_in:
        result[:fade_in] *= np.linspace(0.0, 1.0, fade_in, dtype=np.float32)
    if fade_out:
        result[-fade_out:] *= np.linspace(1.0, 0.0, fade_out, dtype=np.float32)
    return result


def smooth_loop(audio: np.ndarray) -> np.ndarray:
    result = audio.copy()
    count = min(len(result) // 8, int(TARGET_RATE * 0.001))
    if count < 2:
        return result
    phase = np.linspace(0.0, np.pi / 2.0, count, dtype=np.float32)
    fade = np.sin(phase) ** 2
    result[:count] *= fade
    result[-count:] *= fade[::-1]
    return result


def encode_mulaw(audio: np.ndarray) -> bytes:
    pcm = np.asarray(np.clip(audio, -1.0, 1.0) * 32767.0, dtype=np.int32)
    sign = np.where(pcm < 0, 0x80, 0)
    magnitude = np.minimum(np.abs(pcm), 32635) + 0x84
    exponent = np.zeros_like(magnitude)
    mask = np.full_like(magnitude, 0x4000)
    for level in range(7, 0, -1):
        selected = (exponent == 0) & ((magnitude & mask) != 0)
        exponent[selected] = level
        mask >>= 1
    mantissa = (magnitude >> (exponent + 3)) & 0x0F
    encoded = ~(sign | (exponent << 4) | mantissa) & 0xFF
    return encoded.astype(np.uint8).tobytes()


def fingerprint(audio: np.ndarray) -> str:
    if len(audio) > TARGET_RATE * 2:
        audio = audio[: TARGET_RATE * 2]
    mel = librosa.feature.melspectrogram(y=audio, sr=TARGET_RATE, n_mels=24, n_fft=1024)
    vector = librosa.power_to_db(mel + 1e-9).mean(axis=1)
    quantized = np.round(vector / 3.0).astype(np.int8)
    return hashlib.sha1(quantized.tobytes()).hexdigest()[:12]


def estimate_root(audio: np.ndarray, kind: str) -> tuple[int | None, float]:
    duration = min(len(audio), int(TARGET_RATE * 0.8))
    if duration < 2048:
        return None, 0.0
    low = "bass" in kind
    fmin = librosa.note_to_hz("C1" if low else "C3")
    fmax = librosa.note_to_hz("C4" if low else "C7")
    f0, voiced, probability = librosa.pyin(
        audio[:duration], fmin=fmin, fmax=fmax, sr=TARGET_RATE, frame_length=2048
    )
    valid = np.isfinite(f0) & voiced & (probability >= 0.55)
    if np.count_nonzero(valid) < 3:
        return None, float(np.nanmean(probability)) if len(probability) else 0.0
    frequency = float(np.median(f0[valid]))
    semitone = int(round(12.0 * math.log2(frequency / 16.351597831287414)))
    return semitone, float(np.mean(probability[valid]))


def classify_drum(window: np.ndarray) -> str:
    spectrum = np.abs(np.fft.rfft(window * np.hanning(len(window)))) ** 2
    frequencies = np.fft.rfftfreq(len(window), 1.0 / TARGET_RATE)
    total = float(np.sum(spectrum)) + 1e-12
    low = float(np.sum(spectrum[frequencies < 180])) / total
    high = float(np.sum(spectrum[frequencies > 3500])) / total
    if low > 0.48:
        return "kick"
    if high > 0.30:
        return "hat"
    return "snare"


class PackBuilder:
    def __init__(self, output: Path, max_bytes: int, source_count: int):
        self.output = output
        self.max_bytes = max_bytes
        self.source_budget = max_bytes // max(source_count, 1)
        self.loop_budget = int(self.source_budget * 0.45)
        self.pack = bytearray()
        self.elements: list[Element] = []
        self.fingerprints: set[tuple[str, str]] = set()
        self.rejections: dict[str, int] = {}
        self.preview: list[np.ndarray] = []
        self.source_bytes: dict[str, int] = {}
        self.source_loop_bytes: dict[str, int] = {}

    def reject(self, reason: str) -> None:
        self.rejections[reason] = self.rejections.get(reason, 0) + 1

    def add(
        self,
        audio: np.ndarray,
        *,
        kind: str,
        source: dict[str, object],
        looped: bool,
        bars: int = 0,
        phase: int = 0,
        root: int | None = None,
        tags: list[str] | None = None,
    ) -> None:
        if dbfs_rms(audio) < SILENCE_RMS_DB:
            self.reject("quiet")
            return
        if looped:
            audio = smooth_loop(audio)
        else:
            audio = fade_one_shot(align_one_shot(audio, kind), kind)
        audio, applied_gain = normalize(audio)
        identity = fingerprint(audio)
        dedupe_key = (kind, identity)
        if dedupe_key in self.fingerprints:
            self.reject("duplicate")
            return
        encoded = encode_mulaw(audio)
        source_id = str(source["id"])
        if self.source_bytes.get(source_id, 0) + len(encoded) > self.source_budget:
            self.reject("source_budget")
            return
        if looped and self.source_loop_bytes.get(source_id, 0) + len(encoded) > self.loop_budget:
            self.reject("loop_budget")
            return
        if len(self.pack) + len(encoded) > self.max_bytes:
            self.reject("pack_full")
            return
        element_id = f"{source['id']}-{kind}-{len(self.elements):04d}"
        offset = len(self.pack)
        self.pack.extend(encoded)
        self.source_bytes[source_id] = self.source_bytes.get(source_id, 0) + len(encoded)
        if looped:
            self.source_loop_bytes[source_id] = self.source_loop_bytes.get(source_id, 0) + len(encoded)
        rms = dbfs_rms(audio)
        energy = int(np.clip(round((rms + 42.0) / 30.0 * 255.0), 0, 255))
        self.elements.append(
            Element(
                id=element_id,
                kind=kind,
                source_id=source_id,
                offset=offset,
                length=len(encoded),
                sample_rate=TARGET_RATE,
                gain=round(1.0 / max(applied_gain, 1e-6), 4),
                looped=looped,
                bars=bars,
                phase=phase,
                bpm=int(source["bpm"]),
                key=str(source["key"]),
                mode=str(source["mode"]),
                progression=str(source["progression"]),
                root_semitone=root,
                energy=energy,
                tags=list(tags or []),
            )
        )
        self.fingerprints.add(dedupe_key)
        excerpt = audio[: min(len(audio), TARGET_RATE * 2)]
        self.preview.extend([excerpt, np.zeros(TARGET_RATE // 6, dtype=np.float32)])

    def finish(self, sources: list[dict[str, object]]) -> None:
        self.output.mkdir(parents=True, exist_ok=True)
        elements = sorted(self.elements, key=lambda element: KIND_ORDER.index(element.kind))
        header_size = 16
        kind_table_size = len(KIND_ORDER) * 4
        root_table_size = len(ROOT_KINDS) * 128 * ROOT_VARIANTS * 2
        record_size = 28
        data_offset = header_size + kind_table_size + root_table_size + len(elements) * record_size

        kind_table = bytearray()
        for kind in KIND_ORDER:
            indexes = [index for index, element in enumerate(elements) if element.kind == kind]
            start = indexes[0] if indexes else 0
            kind_table.extend(struct.pack("<HH", start, len(indexes)))

        root_table = bytearray()
        for kind in ROOT_KINDS:
            candidates = [
                (index, element)
                for index, element in enumerate(elements)
                if element.kind == kind and element.root_semitone is not None
            ]
            for target in range(128):
                if not candidates:
                    root_table.extend(struct.pack("<H", 0xFFFF) * ROOT_VARIANTS)
                    continue
                ranked = sorted(
                    candidates,
                    key=lambda candidate: (
                        abs(int(candidate[1].root_semitone) - target),
                        -candidate[1].energy,
                        candidate[1].source_id,
                    ),
                )
                for variant in range(ROOT_VARIANTS):
                    root_table.extend(struct.pack("<H", ranked[variant % len(ranked)][0]))

        records = bytearray()
        key_classes = {"C": 0, "C#": 1, "Db": 1, "D": 2, "Eb": 3, "E": 4, "F": 5, "F#": 6, "Gb": 6, "G": 7, "Ab": 8, "A": 9, "Bb": 10, "B": 11}
        mode_codes = {"Major": 0, "Minor": 1, "Dorian": 2}
        for element in elements:
            root = element.root_semitone if element.root_semitone is not None else -1
            records.extend(
                struct.pack(
                    "<BBBBHBBIIbBHII",
                    KIND_ORDER.index(element.kind),
                    1 if element.looped else 0,
                    element.bars,
                    element.phase,
                    element.bpm,
                    key_classes.get(element.key, 0),
                    mode_codes.get(element.mode, 0),
                    zlib.crc32(element.source_id.encode()),
                    zlib.crc32(element.progression.encode()),
                    root,
                    element.energy,
                    int(np.clip(round(element.gain * 32768.0), 0, 65535)),
                    data_offset + element.offset,
                    element.length,
                )
            )

        header = struct.pack(
            "<4sBBHII", b"LFPK", 2, 1, len(elements), TARGET_RATE, data_offset
        )
        packed = header + kind_table + root_table + records + self.pack
        (self.output / "catalog.pack").write_bytes(packed)
        manifest = {
            "version": 2,
            "codec": "mulaw",
            "sample_rate": TARGET_RATE,
            "pack_bytes": len(packed),
            "audio_bytes": len(self.pack),
            "element_count": len(elements),
            "sources": sources,
            "rejections": self.rejections,
            "elements": [asdict(element) for element in elements],
        }
        (self.output / "catalog.json").write_text(json.dumps(manifest, indent=2) + "\n")
        if self.preview:
            sf.write(self.output / "catalog-preview.wav", np.concatenate(self.preview), TARGET_RATE)


def harvest_run(builder: PackBuilder, run: Path, source: dict[str, object]) -> None:
    stems = run / "stems"
    beats_per_bar = 4
    bar_frames = int(TARGET_RATE * 60.0 / int(source["bpm"]) * beats_per_bar)
    stem_kinds = {
        "drums": "drum_loop",
        "bass": "bass_loop",
        "guitar": "melody_loop",
        "piano": "harmony_loop",
        "other": "texture_loop",
    }
    for stem, loop_kind in stem_kinds.items():
        path = stems / f"{stem}.wav"
        if not path.exists():
            builder.reject("missing_stem")
            continue
        audio = load_mono(path)
        loop_bars = 1 if stem == "drums" else 4
        frames = bar_frames * loop_bars
        max_loops = 4 if stem == "drums" else 1
        for loop_index, start in enumerate(range(0, len(audio) - frames + 1, frames)):
            if loop_index >= max_loops:
                break
            phase = (start // bar_frames) % 4
            builder.add(
                audio[start : start + frames],
                kind=loop_kind,
                source=source,
                looped=True,
                bars=loop_bars,
                phase=phase,
                tags=[stem, "ai-harvest"],
            )

        envelope = librosa.onset.onset_strength(y=audio, sr=TARGET_RATE, hop_length=256)
        onset_frames = librosa.onset.onset_detect(
            onset_envelope=envelope,
            sr=TARGET_RATE,
            hop_length=256,
            backtrack=True,
            units="samples",
        )
        for onset in onset_frames[:MAX_ONE_SHOTS_PER_STEM]:
            onset = max(0, int(onset) - int(TARGET_RATE * 0.008))
            if stem == "drums":
                probe = audio[onset : onset + int(TARGET_RATE * 0.09)]
                if len(probe) < 128:
                    continue
                kind = classify_drum(probe)
                duration = {"kick": 0.7, "snare": 0.65, "hat": 0.35}[kind]
                root = None
                confidence = 1.0
            elif stem in ("bass", "guitar", "piano", "other"):
                kind = {
                    "bass": "bass_note",
                    "guitar": "lead_note",
                    "piano": "keys_note",
                    "other": "keys_note",
                }[stem]
                duration = {"bass": 1.4, "guitar": 1.1, "piano": 1.6, "other": 1.4}[stem]
                candidate = audio[onset : onset + int(TARGET_RATE * duration)]
                root, confidence = estimate_root(candidate, kind)
                threshold = 0.6 if stem in ("bass", "guitar") else 0.72
                if root is None or confidence < threshold:
                    builder.reject("unpitched_note")
                    continue
            candidate = audio[onset : onset + int(TARGET_RATE * duration)]
            builder.add(
                candidate,
                kind=kind,
                source=source,
                looped=False,
                root=root,
                tags=[stem, "one-shot", f"confidence:{confidence:.2f}"],
            )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("runs", type=Path, help="directory containing completed forge runs")
    parser.add_argument("output", type=Path)
    parser.add_argument("--max-pack-mb", type=float, default=12.0)
    args = parser.parse_args()

    harvestable = []
    for metadata in sorted(args.runs.glob("*/source.json")):
        source = json.loads(metadata.read_text())
        if source.get("status") == "harvestable":
            harvestable.append((metadata, source))
    builder = PackBuilder(
        args.output, int(args.max_pack_mb * 1024 * 1024), len(harvestable)
    )
    sources = []
    for metadata, source in harvestable:
        sources.append(source)
        harvest_run(builder, metadata.parent, source)
    builder.finish(sources)
    packed_size = (args.output / "catalog.pack").stat().st_size
    print(
        f"catalog: {len(builder.elements)} elements, "
        f"{packed_size / (1024 * 1024):.2f} MiB, rejections={builder.rejections}"
    )


if __name__ == "__main__":
    main()
