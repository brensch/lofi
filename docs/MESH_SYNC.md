# Mesh Sync Design

This is **implemented** in `lofi-core::mesh` and validated by
`crates/lofi-core/tests/mesh_convergence.rs`. The simulator and (eventually) the
firmware drive the same `SyncEngine`.

## Goal

Keep a swarm of boxes on one musical timeline over a lossy, variable-latency
ESP-NOW link, with no infrastructure and no fixed leader. Target accuracy is
low-single-digit milliseconds — tighter than casual IoT, looser than sample
sync. Audio never blocks on the network; it reads a disciplined local clock.

## Why this shape (not pure averaging consensus)

An earlier sketch proposed weighted-average consensus (every node slews toward
the mean of its peers). That is elegant and leaderless, but it has two
properties that hurt *musical* sync: convergence is slow, and a cluster **merge
forces a step for everyone** (the mean of two timelines), which is an audible
glitch on every box at once.

Instead we use a **leaderless-emergent root** with **multi-parent NTP
discipline**:

- The root is whichever live node has the lowest id — an emergent, self-healing
  choice (no election messages, no infrastructure), discovered by gossiping
  `(root_id, stratum)` in beacons. Any node can be root; if it dies the next
  lowest id takes over.
- Every node measures *pairwise* offset/delay to the peers between it and the
  root (NTP four-timestamp exchange) and disciplines toward them, weighted by
  path quality. Multiple upstream peers give multi-path robustness without a
  single fragile parent.
- A merge is clean: the two clusters already agree *within* themselves; only the
  higher-id cluster re-parents and slews onto the lower-id root. One side moves,
  the winning side never glitches.

## Messages (`mesh::wire`)

All timestamps are local monotonic microseconds. Firmware stamps RX near the
radio interrupt and TX at send.

- `Beacon` (broadcast, ~300 ms): `sender, root_id, stratum, epoch, mesh_us,
  rate_ppb, root_dispersion, seq`. Topology + coarse time.
- `ProbeRequest` (unicast to an upstream peer, ~400 ms): `t1` = send time.
- `ProbeResponse`: echoes `t1`, adds `t2` (responder RX), `t3` (responder TX),
  and the responder's `mesh_us` at `t3`, plus its `root_id`/`stratum`.

From a completed exchange `t1..t4`:

```text
rtt   = (t4 - t1) - (t3 - t2)
delay = rtt / 2
reference_mesh_at_t4 = responder_mesh_at_t3 + delay
error = reference_mesh_at_t4 - our_mesh(t4)
```

## Peer table (`mesh::peer`)

Fixed capacity, no allocation. Per peer: smoothed one-way `delay` and `jitter`
(EWMA), last error, advertised `root_id`/`stratum`, age. Weight rewards low
delay, low jitter, and low stratum. Samples whose delay is far above the best
seen are rejected as outliers (retransmits / congested slots). Stale peers age
out of both the election and discipline.

## Disciplined clock (`mesh::clock`)

`mesh_time = local + offset + local·rate`, built on the tested affine
`ClockModel`, plus two things musical scheduling needs:

- **Cold-start step**: the first reference observation snaps onto the timeline
  instead of slewing in for minutes.
- **Continuous scheduling output** (`schedule_now`): never decreases and limits
  phase correction to 0.5% of elapsed local time. Discipline therefore changes
  playback rate briefly instead of holding or jumping the sample cursor.

On a root change (merge/heal) the measurement model re-anchors immediately, but
the scheduling output still approaches it through the bounded slew.

## Failure handling

- Peer disappears → its samples age out; election and discipline ignore it.
- Root disappears → next lowest id becomes root within the peer timeout.
- Split → each side keeps playing from its own emergent root.
- Merge → higher-id side re-parents and slews onto the lower-id root.

Validated by tests: convergence under drift+loss, monotonic mesh time across the
swarm, root failover, and split→merge.

## Open / future

- Firmware: ESP-NOW RX/TX tasks timestamp frames and hand bytes to `SyncEngine`;
  fill `t3`/`sender_mesh` at the real TX instant for best accuracy.
- Transport/groove-state catch-up for late joiners (separate from clock epoch).
- Call/response on the shared timeline (uses the scheduled-event system).
- Tuning the discipline gains against real ESP-NOW latency once boards exist.
