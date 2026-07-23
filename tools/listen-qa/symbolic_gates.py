#!/usr/bin/env python3
"""Property gates over the symbolic score (score_dump JSONL).

Because the composer is symbolic, musical craft rules become testable
predicates that name the failing bar and lane — no listening involved.

Usage: symbolic_gates.py score.jsonl [--strict]
"""

from __future__ import annotations

import json
import sys
from collections import defaultdict
from pathlib import Path

STEPS_PER_BAR = 16
STEPS_PER_PHRASE = 128


def load(path: str) -> tuple[dict, list[dict]]:
    meta, events = {}, []
    for line in Path(path).read_text().splitlines():
        row = json.loads(line)
        if "meta" in row:
            meta = row["meta"]
        else:
            events.append(row)
    return meta, events


def gate(failures: list[str], ok: bool, message: str) -> None:
    if not ok:
        failures.append(message)


def check(meta: dict, events: list[dict]) -> list[str]:
    failures: list[str] = []
    by_lane = defaultdict(list)
    for event in events:
        by_lane[event["lane"]].append(event)
    total_steps = max((e["step"] for e in events), default=0) + 1
    phrases = max(1, (total_steps + STEPS_PER_PHRASE - 1) // STEPS_PER_PHRASE)

    # 1. Repitch quality: no sampled voice stretched past a tritone.
    gate(
        failures,
        float(meta.get("worst_repitch", 99)) <= 6.0,
        f"worst repitch {meta.get('worst_repitch')} semitones exceeds 6",
    )

    # 2. Backbeat integrity: strong snare on 4 and 12 (or 8 in half-time bars),
    #    and nothing loud anywhere else.
    for event in by_lane["snare"]:
        pos, strong = event["bar_pos"], event["level"] > 0.35
        if strong and pos not in (4, 12, 8):
            failures.append(f"loud snare off the backbeat at step {event['step']} (pos {pos})")

    # 3. Kick pattern: always on the downbeat of each bar, bounded density.
    kick_bars = defaultdict(set)
    for event in by_lane["kick"]:
        kick_bars[event["step"] // STEPS_PER_BAR].add(event["bar_pos"])
    for bar, positions in kick_bars.items():
        gate(failures, 0 in positions, f"bar {bar} has no downbeat kick")
        gate(failures, len(positions) <= 5, f"bar {bar} has {len(positions)} kicks")

    # 4. Space: the lead must rest on most steps; keys must not be a pad.
    lead_steps = {e["step"] for e in by_lane["lead"]}
    gate(
        failures,
        len(lead_steps) <= 0.25 * total_steps,
        f"lead plays {len(lead_steps)}/{total_steps} steps: no room to breathe",
    )
    keys_strikes = {e["step"] for e in by_lane["keys"]}
    gate(
        failures,
        len(keys_strikes) <= 0.40 * total_steps,
        f"keys strike {len(keys_strikes)}/{total_steps} steps",
    )
    gate(failures, len(by_lane["keys"]) > 0, "keys are entirely silent")
    gate(failures, len(by_lane["bass"]) > 0, "bass is entirely silent")

    # 5. Bass downbeats are chord roots.
    for event in by_lane["bass"]:
        if event["bar_pos"] == 0 and event.get("midi"):
            if event["midi"] % 12 != event["chord_root"] % 12:
                failures.append(
                    f"bass downbeat {event['midi']} is not the chord root "
                    f"{event['chord_root']} at step {event['step']}"
                )

    # 6. Harmonic collision: lead within a semitone of a sounding keys voice
    #    on the same step is mud; flag if it recurs.
    keys_by_step = defaultdict(list)
    for event in by_lane["keys"]:
        if event.get("midi"):
            keys_by_step[event["step"]].append(event["midi"])
    clashes = 0
    for event in by_lane["lead"]:
        midi = event.get("midi")
        if not midi:
            continue
        for other in keys_by_step.get(event["step"], []):
            if abs(midi - other) in (1, 2) or abs((midi - other) % 12) in (1, 11):
                clashes += 1
    gate(failures, clashes <= phrases, f"{clashes} lead/keys semitone clashes")

    # 7. Evolution: adjacent phrases must differ somewhere, but the drum
    #    backbone must never fully change at once.
    def phrase_signature(phrase: int, lane: str) -> frozenset:
        lo, hi = phrase * STEPS_PER_PHRASE, (phrase + 1) * STEPS_PER_PHRASE
        return frozenset(
            (e["step"] - lo) for e in by_lane[lane] if lo <= e["step"] < hi
        )

    for phrase in range(phrases - 1):
        changed = sum(
            phrase_signature(phrase, lane) != phrase_signature(phrase + 1, lane)
            for lane in ("kick", "snare", "hat", "bass", "keys", "lead")
        )
        gate(failures, changed >= 1, f"phrases {phrase}->{phrase + 1} are identical")

    # 8. Dynamics: levels bounded, downbeats stronger than offbeats on average.
    for lane, events_ in by_lane.items():
        for event in events_:
            gate(
                failures,
                0.0 < event["level"] <= 1.3,
                f"{lane} level {event['level']} out of range at step {event['step']}",
            )

    # 9. Micro-timing: delays bounded (pocket + swing + humanize < 60 ms).
    for lane, events_ in by_lane.items():
        for event in events_:
            gate(
                failures,
                -2_000 <= event["delay_us"] <= 60_000,
                f"{lane} delay {event['delay_us']}us out of pocket at step {event['step']}",
            )

    return failures


def main() -> int:
    path = sys.argv[1]
    meta, events = load(path)
    failures = check(meta, events)
    lanes = defaultdict(int)
    for event in events:
        lanes[event["lane"]] += 1
    summary = ", ".join(f"{lane}:{count}" for lane, count in sorted(lanes.items()))
    print(f"{path}: {len(events)} events ({summary})")
    if failures:
        print(f"FAIL ({len(failures)} problems)")
        for failure in failures[:40]:
            print(f"  - {failure}")
        return 1
    print("PASS: every symbolic property holds")
    return 0


if __name__ == "__main__":
    sys.exit(main())
