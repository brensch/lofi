# Mesh Sync Design

## Target

Use a true mesh clock, not only a leader clock. There can still be a temporary coordinator for transport decisions, but the time base should be estimated from peer relationships and degrade gracefully when a node disappears.

The musical target is tighter than casual IoT timing but looser than sample sync:

- sub-millisecond to low-single-digit millisecond timing is useful
- future scheduled events matter more than "play this packet now"
- audio engines never block on mesh traffic

## Clock Representation

Each node maintains:

```text
mesh_time = local_time + offset + local_time * rate
```

The raw local timer is monotonic. `mesh_time` is a slewed estimate and should not jump backward.

## Pairwise Measurement

Every node periodically runs NTP-style probes with peers:

```text
A sends probe at A:t1
B receives at B:t2
B sends response at B:t3
A receives at A:t4

offset_ab = ((t2 - t1) + (t3 - t4)) / 2
delay_ab  = ((t4 - t1) - (t3 - t2)) / 2
```

Samples with high delay are low quality and should be rejected or heavily down-weighted. The best sample is often the lowest-delay sample in a short window.

## Mesh Estimate

For 3-10 boxes, use weighted offset averaging:

1. Maintain a peer table with the latest pairwise offset, delay, jitter, age, and quality.
2. Convert each peer's reported mesh-time estimate into a local correction candidate.
3. Weight candidates by quality:
   - lower delay is better
   - lower jitter is better
   - fresher is better
   - peers with more stable clocks are better
4. Compute a trimmed weighted average, discarding obvious outliers.
5. Slew local offset/rate toward that consensus.

This gives the "true mesh" behavior: no single permanent root. The group clock is the consensus of the group.

## Epochs

The mesh still needs an epoch id so devices know which shared time universe they are in. A simple initial rule:

- each isolated device starts an epoch from its own node id and boot counter
- when groups meet, the larger group wins
- tie-break by lowest epoch id
- losing group slews into the winning epoch, never hard-jumps the audio clock

Transport state is separate from clock epoch. A node can join the clock epoch first, then receive the current transport/groove state.

## Broadcast Cadence

Suggested starting values:

- beacon every 250 ms
- pairwise probe each visible peer every 1-3 seconds, jittered
- transport/groove state every 1 second while playing
- scheduled events rebroadcast until their fire tick is safely in the past

Avoid synchronized broadcast storms. Every periodic job needs deterministic jitter from node id and sequence.

## Call/Response

Local action happens immediately on the initiating device. The initiating device also broadcasts a scheduled response:

```text
call happened at tick C
response fires at next_bar(C) + 4 bars
action = CallResponse(call_id, source_node, phrase ids)
```

Other devices prepare local variations from the shared seed, role, and call id. They do not need streamed notes.

## Failure Handling

- If a peer disappears, age out its samples gradually.
- If the mesh splits, both sides keep playing from their local consensus.
- If groups rejoin, merge epochs by size/tie-break and slew.
- Scheduled events carry ids and dedupe keys so rebroadcasts are safe.

## Implementation Phases

0. Current simulator baseline: all-to-all mesh beacons where every reachable node slews toward peer mesh-time estimates. The CLI demo can start two isolated four-device clusters, sync each cluster internally, then open cross-cluster links so both sides merge into one consensus.
1. Replace beacon-only correction with all-to-all pairwise probes and weighted averaging.
2. Add packet loss, jitter, clock drift, and split/merge scenarios.
3. Add monotonic slew constraints and tests for no backward mesh time.
4. Move the same state machine into `lofi-core`.
5. Wire firmware ESP-NOW receive/send tasks to the mesh state machine.
