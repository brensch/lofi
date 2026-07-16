//! The mesh sync state machine.
//!
//! One per box. Firmware (and the simulator) drives it with three things:
//! timestamped received frames ([`SyncEngine::handle`]), periodic "what should
//! I send?" polls ([`SyncEngine::due_beacon`] / [`SyncEngine::due_probe`]), and
//! the local hardware clock for scheduling ([`SyncEngine::schedule_now`]).
//!
//! The engine itself owns the policy: a distributed lowest-id root election, an
//! NTP-style probe exchange to upstream peers, weighted discipline of the mesh
//! clock with outlier rejection, and a clean re-step when the root changes on a
//! cluster merge.

use crate::clock::DisciplineConfig;
use crate::mesh::clock::MeshClock;
use crate::mesh::peer::{PeerTable, MAX_PEERS};
use crate::mesh::wire::{Beacon, MeshMessage, ProbeRequest, ProbeResponse};
use crate::{Micros, NodeId};

pub const BEACON_INTERVAL_US: Micros = 300_000;
pub const PROBE_INTERVAL_US: Micros = 400_000;
/// Reject a probe whose one-way delay is far above the best seen: likely a
/// retransmit or a congested air slot, not a real path.
const OUTLIER_DELAY_FACTOR: Micros = 4;
const OUTLIER_DELAY_SLACK_US: Micros = 1_000;
const MAX_ROSTER: usize = MAX_PEERS + 1;

/// The mesh membership as one node sees it: sorted unique ids + this node's slot.
#[derive(Clone, Copy, Debug)]
pub struct RosterView {
    ids: [NodeId; MAX_ROSTER],
    len: usize,
    my_index: usize,
}

impl RosterView {
    pub fn ids(&self) -> &[NodeId] {
        &self.ids[..self.len]
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn my_index(&self) -> usize {
        self.my_index
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncQuality {
    pub root_id: NodeId,
    pub stratum: u8,
    pub peers: u8,
    pub dispersion_us: i32,
    pub is_root: bool,
    pub synced: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct SyncEngine {
    node_id: NodeId,
    clock: MeshClock,
    peers: PeerTable,
    root_id: NodeId,
    stratum: u8,
    epoch: u32,
    seq: u32,
    last_beacon_us: Micros,
    last_probe_us: Micros,
    probe_cursor: usize,
}

impl SyncEngine {
    pub fn new(node_id: NodeId) -> Self {
        let mut clock = MeshClock::new();
        clock.anchor_as_root(); // a lone box is the root of its own timeline
        Self {
            node_id,
            clock,
            peers: PeerTable::new(),
            root_id: node_id,
            stratum: 0,
            epoch: node_id as u32,
            seq: 0,
            last_beacon_us: Micros::MIN,
            last_probe_us: Micros::MIN,
            probe_cursor: 0,
        }
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn is_root(&self) -> bool {
        self.root_id == self.node_id
    }

    /// The agreed mesh roster: this node plus its fresh peers, sorted by id and
    /// deduped, with this node's position. Drives role assignment and the
    /// turn-based arrangement (every box derives the same roster).
    pub fn roster(&self, now_us: Micros) -> RosterView {
        let mut ids = [0u64; MAX_ROSTER];
        let mut len = 1;
        ids[0] = self.node_id;
        for peer in self.peers.fresh_peers(now_us) {
            if len < MAX_ROSTER && !ids[..len].contains(&peer.id) {
                ids[len] = peer.id;
                len += 1;
            }
        }
        ids[..len].sort_unstable();
        let my_index = ids[..len]
            .iter()
            .position(|id| *id == self.node_id)
            .unwrap_or(0);
        RosterView { ids, len, my_index }
    }

    /// Monotonic mesh time for scheduling musical events.
    pub fn schedule_now(&mut self, local_us: Micros) -> Micros {
        self.clock.schedule_now(local_us)
    }

    /// Mesh time for measurement / display (not guaranteed monotonic).
    pub fn mesh_from_local(&self, local_us: Micros) -> Micros {
        self.clock.mesh_from_local(local_us)
    }

    pub fn quality(&self, now_us: Micros) -> SyncQuality {
        SyncQuality {
            root_id: self.root_id,
            stratum: self.stratum,
            peers: self.peers.fresh_count(now_us),
            dispersion_us: self.dispersion_us(now_us),
            is_root: self.is_root(),
            synced: self.is_root() || (self.stratum > 0 && self.clock.has_reference()),
        }
    }

    /// Ingest a received frame, returning an immediate reply to unicast back
    /// (only for probe requests addressed to us).
    pub fn handle(&mut self, msg: MeshMessage, rx_local_us: Micros) -> Option<MeshMessage> {
        match msg {
            MeshMessage::Beacon(b) => {
                if b.sender != self.node_id {
                    self.peers
                        .observe_beacon(b.sender, b.root_id, b.stratum, rx_local_us);
                    self.update_reference(rx_local_us);
                }
                None
            }
            MeshMessage::ProbeRequest(p) => {
                if p.target != self.node_id {
                    return None;
                }
                // t2 = receive, t3 = transmit. We approximate t3 == t2; firmware
                // may overwrite t3 and sender_mesh_us at the real send instant.
                Some(MeshMessage::ProbeResponse(ProbeResponse {
                    sender: self.node_id,
                    target: p.sender,
                    t1_local_us: p.t1_local_us,
                    t2_local_us: rx_local_us,
                    t3_local_us: rx_local_us,
                    sender_mesh_us: self.clock.mesh_from_local(rx_local_us),
                    root_id: self.root_id,
                    stratum: self.stratum,
                    seq: p.seq,
                }))
            }
            MeshMessage::ProbeResponse(r) => {
                if r.target == self.node_id {
                    self.ingest_response(r, rx_local_us);
                }
                None
            }
        }
    }

    fn ingest_response(&mut self, r: ProbeResponse, t4_local_us: Micros) {
        let rtt = (t4_local_us - r.t1_local_us) - (r.t3_local_us - r.t2_local_us);
        if rtt < 0 {
            return; // impossible timing, drop
        }
        let delay = rtt / 2;
        let ref_at_t4 = r.sender_mesh_us + delay;
        let error = ref_at_t4 - self.clock.mesh_from_local(t4_local_us);

        self.peers
            .observe_probe(r.sender, r.root_id, r.stratum, delay, error, t4_local_us);

        // Discipline only toward peers strictly closer to the root, and only on
        // clean low-delay samples.
        let upstream = r.stratum < self.stratum;
        let outlier = self
            .peers
            .best_delay_us(t4_local_us)
            .is_some_and(|best| delay > best * OUTLIER_DELAY_FACTOR + OUTLIER_DELAY_SLACK_US);
        if upstream && !outlier && !self.is_root() {
            let weight = self
                .peers
                .get(r.sender)
                .map(|p| p.weight(t4_local_us))
                .unwrap_or(0);
            self.clock
                .observe(t4_local_us, ref_at_t4, discipline_cfg(weight));
        }
    }

    fn update_reference(&mut self, now_us: Micros) {
        let (root, stratum) = self.peers.elect_root(self.node_id, now_us);
        if root == self.root_id {
            self.stratum = stratum;
            return;
        }
        // Root changed (join/merge/heal).
        self.root_id = root;
        self.epoch = root as u32;
        if root == self.node_id {
            self.stratum = 0;
            self.clock.anchor_as_root(); // freeze and free-run
        } else {
            self.stratum = stratum;
            self.clock.request_resync(); // step onto the new root's timeline
        }
    }

    /// A beacon to broadcast, if the interval has elapsed.
    pub fn due_beacon(&mut self, now_us: Micros) -> Option<MeshMessage> {
        if now_us < self.last_beacon_us {
            self.last_beacon_us = now_us; // local clock stepped back; don't stall
        }
        if now_us.saturating_sub(self.last_beacon_us) < BEACON_INTERVAL_US {
            return None;
        }
        // Re-run election on our own cadence so an isolated node recovers after
        // its last peer expires. Waiting for an incoming beacon leaves a fully
        // partitioned node stuck on a root it can no longer hear.
        self.update_reference(now_us);
        self.last_beacon_us = now_us;
        self.seq = self.seq.wrapping_add(1);
        Some(MeshMessage::Beacon(Beacon {
            sender: self.node_id,
            root_id: self.root_id,
            epoch: self.epoch,
            stratum: self.stratum,
            seq: self.seq,
            mesh_us: self.clock.mesh_from_local(now_us),
            rate_ppb: self.clock.rate_ppb(),
            root_dispersion_us: self.dispersion_us(now_us),
        }))
    }

    /// A probe to unicast to an upstream peer, if the interval has elapsed.
    /// Returns `(destination, message)`.
    pub fn due_probe(&mut self, now_us: Micros) -> Option<(NodeId, MeshMessage)> {
        if now_us < self.last_probe_us {
            self.last_probe_us = now_us; // local clock stepped back; don't stall
        }
        if now_us.saturating_sub(self.last_probe_us) < PROBE_INTERVAL_US {
            return None;
        }
        let target = self.next_upstream(now_us)?;
        self.last_probe_us = now_us;
        self.seq = self.seq.wrapping_add(1);
        Some((
            target,
            MeshMessage::ProbeRequest(ProbeRequest {
                sender: self.node_id,
                target,
                t1_local_us: now_us,
                seq: self.seq,
            }),
        ))
    }

    fn next_upstream(&mut self, now_us: Micros) -> Option<NodeId> {
        let mut ids = [0u64; MAX_PEERS];
        let mut count = 0;
        for peer in self.peers.fresh_peers(now_us) {
            if peer.stratum < self.stratum {
                ids[count] = peer.id;
                count += 1;
            }
        }
        if count == 0 {
            return None;
        }
        self.probe_cursor = (self.probe_cursor + 1) % count;
        Some(ids[self.probe_cursor])
    }

    fn dispersion_us(&self, now_us: Micros) -> i32 {
        if self.is_root() {
            return 0;
        }
        self.peers
            .fresh_peers(now_us)
            .filter(|p| p.stratum < self.stratum && p.samples > 0)
            .map(|p| (p.delay_us + p.jitter_us).clamp(0, i32::MAX as Micros) as i32)
            .min()
            .unwrap_or(i32::MAX)
    }
}

/// Higher-weight peers get a tighter smoothing shift (faster correction).
fn discipline_cfg(weight: u32) -> DisciplineConfig {
    let offset_smoothing_shift = if weight > 2_000 {
        3
    } else if weight > 500 {
        4
    } else {
        5
    };
    DisciplineConfig {
        offset_smoothing_shift,
        rate_smoothing_shift: 5,
        max_rate_ppb: 300_000,
        reject_offset_us: 40_000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::peer::PEER_TIMEOUT_US;

    #[test]
    fn isolated_node_re_elects_itself_after_peer_timeout() {
        let mut engine = SyncEngine::new(2);
        engine.handle(
            MeshMessage::Beacon(Beacon {
                sender: 1,
                root_id: 1,
                epoch: 1,
                stratum: 0,
                seq: 1,
                mesh_us: 0,
                rate_ppb: 0,
                root_dispersion_us: 0,
            }),
            0,
        );
        assert_eq!(engine.quality(0).root_id, 1);

        let now = PEER_TIMEOUT_US + BEACON_INTERVAL_US;
        engine.due_beacon(now);
        let quality = engine.quality(now);
        assert!(quality.is_root);
        assert_eq!(quality.root_id, 2);
        assert_eq!(quality.peers, 0);
    }
}
