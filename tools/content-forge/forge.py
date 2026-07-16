#!/usr/bin/env python3
"""Generate, separate, harvest, and pack sample-only lo-fi content."""

from __future__ import annotations

import argparse
import json
import os
import random
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import asdict, dataclass
from pathlib import Path

import numpy as np
import soundfile as sf


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_ACE = Path.home() / ".cache/lofi-tools/ACE-Step-1.5"
DEFAULT_ANALYSIS_PYTHON = Path.home() / ".cache/lofi-tools/audio-analysis/.venv/bin/python"
DEFAULT_FFMPEG = Path.home() / ".cache/lofi-tools/ffmpeg"

KEYS = (("C", "Major"), ("D", "Dorian"), ("E", "Minor"), ("F", "Major"), ("G", "Major"), ("A", "Minor"))
PROGRESSIONS = {
    "Major": (
        "Imaj9 - IVmaj7 - vi7 - V9",
        "Imaj9 - vi7 - ii7 - V9",
        "Imaj9 - iii7 - IVmaj9 - iv6",
    ),
    "Minor": (
        "i9 - VImaj7 - IIImaj9 - VII9",
        "i9 - iv9 - i9 - V9",
        "i9 - IIImaj7 - VImaj9 - V7",
    ),
    "Dorian": (
        "i9 - IV9 - i9 - VIImaj7",
        "i9 - ii7 - IV9 - i9",
        "i9 - VIImaj7 - IV9 - i9",
    ),
}
PALETTES = (
    "muted jazz guitar, warm Rhodes, round electric bass, dry boom-bap drums",
    "soft felt piano, tape-worn guitar harmonics, upright bass, brushed hip-hop drums",
    "mellow Wurlitzer, nylon guitar fragments, subby bass, dusty acoustic drums",
    "dark Rhodes, palm-muted electric guitar, woody bass, restrained pocket drums",
)
MOODS = ("wistful", "rainy", "late-night", "tender", "sleepy", "quietly hopeful")


@dataclass
class Theme:
    id: str
    seed: int
    bpm: int
    key: str
    mode: str
    progression: str
    palette: str
    mood: str
    bars: int
    duration: float
    prompt: str
    status: str = "planned"


def request_json(url: str, payload: dict[str, object], timeout: float = 60.0) -> dict[str, object]:
    request = urllib.request.Request(
        url,
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.loads(response.read())


def server_ready(url: str) -> bool:
    try:
        with urllib.request.urlopen(f"{url}/v1/models", timeout=2.0) as response:
            return response.status == 200
    except (urllib.error.URLError, TimeoutError):
        return False


def start_server(ace: Path, url: str) -> subprocess.Popen[bytes]:
    port = url.rsplit(":", 1)[-1]
    env = os.environ.copy()
    env.update(
        {
            "ACESTEP_CONFIG_PATH": "acestep-v15-turbo",
            "ACESTEP_LM_MODEL_PATH": "acestep-5Hz-lm-1.7B",
            "ACESTEP_LM_BACKEND": "pt",
            "ACESTEP_OFFLOAD_TO_CPU": "true",
            "ACESTEP_LM_OFFLOAD_TO_CPU": "true",
        }
    )
    process = subprocess.Popen(
        [str(ace / ".venv/bin/acestep-api"), "--port", port, "--init-llm"],
        cwd=ace,
        env=env,
    )
    deadline = time.time() + 240
    while time.time() < deadline:
        if process.poll() is not None:
            raise RuntimeError("ACE-Step server exited during startup")
        if server_ready(url):
            return process
        time.sleep(1.0)
    process.terminate()
    raise TimeoutError("ACE-Step server did not become ready")


def choose_theme(rng: random.Random, index: int, bars: int) -> Theme:
    bpm = rng.choice((72, 76, 80, 84))
    key, mode = rng.choice(KEYS)
    progression = rng.choice(PROGRESSIONS[mode])
    palette = rng.choice(PALETTES)
    mood = rng.choice(MOODS)
    seed = rng.randrange(1, 2**32)
    duration = bars * 4 * 60.0 / bpm
    identity = f"theme-{int(time.time())}-{index:03d}-{seed:08x}"
    prompt = (
        f"Instrumental {mood} lo-fi hip-hop at {bpm} BPM in {key} {mode}, 4/4. "
        f"Exact repeating four-bar harmony: {progression}. {palette}. "
        "Write one memorable sparse four-bar melody with a clear answer and long rests. "
        "Keep bass notes long and simple. Keep each instrument spectrally separate, dry, "
        "centered, and easy to isolate into stems. Repeat core phrases exactly before making "
        "small ending variations. Use section changes through subtraction and filtering. "
        "No voice, speech, vocal chops, singing, rapping, bright synths, or orchestral sounds."
    )
    return Theme(identity, seed, bpm, key, mode, progression, palette, mood, bars, duration, prompt)


def generate(theme: Theme, url: str, run: Path) -> None:
    payload = {
        "prompt": theme.prompt,
        "lyrics": "[Instrumental Intro]\n[Theme A]\n[Theme A Repeat]\n[Breakdown]\n[Theme A Return]\n[Instrumental Outro]",
        "thinking": True,
        "use_format": False,
        "use_cot_caption": False,
        "use_cot_language": False,
        "use_cot_metas": False,
        "bpm": theme.bpm,
        "key_scale": f"{theme.key} {theme.mode}",
        "time_signature": "4",
        "audio_duration": theme.duration,
        "audio_format": "wav",
        "inference_steps": 8,
        "use_random_seed": False,
        "seed": theme.seed,
        "batch_size": 1,
    }
    released = request_json(f"{url}/release_task", payload)
    task_id = released["data"]["task_id"] if isinstance(released["data"], dict) else released["data"]
    deadline = time.time() + 600
    while time.time() < deadline:
        queried = request_json(f"{url}/query_result", {"task_id_list": [task_id]})
        task = queried["data"][0]
        if task["status"] == 2:
            raise RuntimeError(f"generation failed: {task}")
        if task["status"] == 1:
            result = json.loads(task["result"])[0]
            audio_url = result["file"]
            if audio_url.startswith("/"):
                audio_url = url + audio_url
            with urllib.request.urlopen(audio_url, timeout=120) as response:
                (run / "mix.wav").write_bytes(response.read())
            return
        time.sleep(1.0)
    raise TimeoutError(f"generation task {task_id} timed out")


def separate(run: Path, analysis_python: Path) -> None:
    output = run / "demucs"
    env = os.environ.copy()
    env["PATH"] = f"{DEFAULT_FFMPEG}:{env.get('PATH', '')}"
    subprocess.run(
        [str(analysis_python), "-m", "demucs", "-n", "htdemucs_6s", "-o", str(output), str(run / "mix.wav")],
        check=True,
        env=env,
    )
    separated = output / "htdemucs_6s" / "mix"
    stems = run / "stems"
    stems.mkdir(exist_ok=True)
    for path in separated.glob("*.wav"):
        shutil.copy2(path, stems / path.name)


def validate(run: Path) -> dict[str, object]:
    measurements: dict[str, dict[str, float]] = {}
    for name in ("mix", "bass", "drums", "guitar", "piano", "other", "vocals"):
        path = run / "mix.wav" if name == "mix" else run / "stems" / f"{name}.wav"
        if not path.exists():
            continue
        audio, _ = sf.read(path, always_2d=True, dtype="float32")
        mono = np.mean(audio, axis=1)
        if not np.all(np.isfinite(mono)):
            return {"accepted": False, "reason": f"nonfinite:{name}"}
        rms = float(np.sqrt(np.mean(mono * mono) + 1e-12))
        measurements[name] = {
            "rms_db": round(20.0 * np.log10(max(rms, 1e-9)), 2),
            "peak": round(float(np.max(np.abs(mono))), 5),
        }

    required = ("mix", "bass", "drums")
    if any(name not in measurements for name in required):
        return {"accepted": False, "reason": "missing_required_stem", "measurements": measurements}
    if measurements["mix"]["peak"] >= 0.995:
        return {"accepted": False, "reason": "clipping", "measurements": measurements}
    if measurements["bass"]["rms_db"] < -44 or measurements["drums"]["rms_db"] < -44:
        return {"accepted": False, "reason": "weak_rhythm_section", "measurements": measurements}
    tonal = max(measurements.get(name, {"rms_db": -120})["rms_db"] for name in ("guitar", "piano", "other"))
    if tonal < -42:
        return {"accepted": False, "reason": "weak_tonal_stems", "measurements": measurements}
    vocal = measurements.get("vocals", {"rms_db": -120})["rms_db"]
    if vocal > measurements["mix"]["rms_db"] - 16:
        return {"accepted": False, "reason": "possible_vocals", "measurements": measurements}
    return {"accepted": True, "reason": "passed", "measurements": measurements}


def import_existing(runs: Path, source_wav: Path, stems: Path, metadata: dict[str, object]) -> None:
    run = runs / str(metadata["id"])
    run.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source_wav, run / "mix.wav")
    destination = run / "stems"
    destination.mkdir(exist_ok=True)
    for stem in ("bass", "drums", "guitar", "piano", "other", "vocals"):
        source = stems / f"{stem}.wav"
        if source.exists():
            shutil.copy2(source, destination / source.name)
    metadata["status"] = "harvestable"
    (run / "source.json").write_text(json.dumps(metadata, indent=2) + "\n")


def rebuild_catalog(python: Path, runs: Path, catalog: Path, max_pack_mb: float) -> None:
    subprocess.run(
        [str(python), str(Path(__file__).with_name("harvest.py")), str(runs), str(catalog), "--max-pack-mb", str(max_pack_mb)],
        check=True,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=ROOT / "target/content-forge")
    parser.add_argument("--count", type=int, default=1)
    parser.add_argument("--forever", action="store_true")
    parser.add_argument("--bars", type=int, default=12)
    parser.add_argument("--random-seed", type=int)
    parser.add_argument("--ace-url", default="http://127.0.0.1:8001")
    parser.add_argument("--ace-path", type=Path, default=DEFAULT_ACE)
    parser.add_argument("--analysis-python", type=Path, default=DEFAULT_ANALYSIS_PYTHON)
    parser.add_argument("--start-server", action="store_true")
    parser.add_argument("--max-pack-mb", type=float, default=12.0)
    parser.add_argument("--rebuild-only", action="store_true")
    args = parser.parse_args()

    runs = args.output / "runs"
    catalog = args.output / "catalog"
    runs.mkdir(parents=True, exist_ok=True)
    if args.rebuild_only:
        rebuild_catalog(args.analysis_python, runs, catalog, args.max_pack_mb)
        return

    process = None
    if not server_ready(args.ace_url):
        if not args.start_server:
            raise RuntimeError("ACE-Step API is unavailable; pass --start-server")
        process = start_server(args.ace_path, args.ace_url)

    rng = random.Random(args.random_seed)
    completed = 0
    try:
        while args.forever or completed < args.count:
            theme = choose_theme(rng, completed, args.bars)
            run = runs / theme.id
            run.mkdir(parents=True)
            (run / "source.json").write_text(json.dumps(asdict(theme), indent=2) + "\n")
            try:
                generate(theme, args.ace_url, run)
                separate(run, args.analysis_python)
                quality = validate(run)
                if not quality["accepted"]:
                    theme.status = "rejected"
                    rejected = asdict(theme) | {"quality": quality}
                    (run / "source.json").write_text(json.dumps(rejected, indent=2) + "\n")
                    print(f"{theme.id}: rejected ({quality['reason']})", file=sys.stderr)
                    if not args.forever:
                        raise RuntimeError(f"quality gate rejected theme: {quality['reason']}")
                    continue
                theme.status = "harvestable"
                accepted = asdict(theme) | {"quality": quality}
                (run / "source.json").write_text(json.dumps(accepted, indent=2) + "\n")
                rebuild_catalog(args.analysis_python, runs, catalog, args.max_pack_mb)
                completed += 1
            except Exception as error:
                theme.status = "failed"
                failed = asdict(theme) | {"error": str(error)}
                (run / "source.json").write_text(json.dumps(failed, indent=2) + "\n")
                print(f"{theme.id}: {error}", file=sys.stderr)
                if not args.forever:
                    raise
            if args.forever:
                time.sleep(2.0)
    finally:
        if process is not None:
            process.terminate()
            process.wait(timeout=30)


if __name__ == "__main__":
    main()
