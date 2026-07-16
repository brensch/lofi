//! Per-peer measurement state and the leaderless root election.
//!
//! Fixed capacity, no allocation. Each peer entry tracks a smoothed one-way
//! delay and jitter (so we can weight low-latency neighbours more and reject
//! outliers) plus the peer's advertised place in the tree, which drives a
//! distributed "lowest id is the root" election.

use crate::{Micros, NodeId};

pub const MAX_PEERS: usize = 12;
/// A peer is ignored for election/discipline if unheard for this long.
pub const PEER_TIMEOUT_US: Micros = 3_000_000;
/// Cap on tree depth; beyond this a peer is treated as having no root, which
/// stops distance-vector count-to-infinity if the root vanishes.
pub const MAX_STRATUM: u8 = 16;

#[derive(Clone, Copy, Debug)]
pub struct Peer {
    pub id: NodeId,
    pub root_id: NodeId,
    pub stratum: u8,
    pub last_seen_us: Micros,
    pub delay_us: Micros,
    pub jitter_us: Micros,
    pub last_error_us: Micros,
    pub samples: u32,
    valid: bool,
}

impl Peer {
    const EMPTY: Self = Self {
        id: 0,
        root_id: 0,
        stratum: MAX_STRATUM,
        last_seen_us: Micros::MIN,
        delay_us: 0,
        jitter_us: 0,
        last_error_us: 0,
        samples: 0,
        valid: false,
    };

    fn fresh(&self, now_us: Micros) -> bool {
        self.valid && now_us.saturating_sub(self.last_seen_us) <= PEER_TIMEOUT_US
    }

    /// Discipline weight: lower delay and jitter, and closer to the root, count
    /// for more. Bigger is better. Zero once stale.
    pub fn weight(&self, now_us: Micros) -> u32 {
        if !self.fresh(now_us) || self.samples == 0 {
            return 0;
        }
        let cost = (self.delay_us + self.jitter_us).max(0) as u64 + 200;
        let base = (4_000_000u64 / cost) as u32;
        base / (self.stratum as u32 + 1)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PeerTable {
    peers: [Peer; MAX_PEERS],
}

impl Default for PeerTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerTable {
    pub const fn new() -> Self {
        Self {
            peers: [Peer::EMPTY; MAX_PEERS],
        }
    }

    fn slot(&mut self, id: NodeId, now_us: Micros) -> &mut Peer {
        if let Some(ix) = self.peers.iter().position(|p| p.valid && p.id == id) {
            return &mut self.peers[ix];
        }
        // Reuse a free slot, else evict the stalest.
        let mut target = 0;
        for ix in 1..MAX_PEERS {
            if !self.peers[ix].valid {
                target = ix;
                break;
            }
            if self.peers[ix].last_seen_us < self.peers[target].last_seen_us {
                target = ix;
            }
        }
        self.peers[target] = Peer {
            id,
            valid: true,
            last_seen_us: now_us,
            ..Peer::EMPTY
        };
        &mut self.peers[target]
    }

    /// Record a beacon: refresh topology and liveness only.
    pub fn observe_beacon(&mut self, id: NodeId, root_id: NodeId, stratum: u8, now_us: Micros) {
        let peer = self.slot(id, now_us);
        peer.root_id = root_id;
        peer.stratum = stratum;
        peer.last_seen_us = now_us;
    }

    /// Record a completed probe exchange: update smoothed delay/jitter and the
    /// last measured mesh-time error against this peer.
    pub fn observe_probe(
        &mut self,
        id: NodeId,
        root_id: NodeId,
        stratum: u8,
        delay_us: Micros,
        error_us: Micros,
        now_us: Micros,
    ) {
        let peer = self.slot(id, now_us);
        peer.root_id = root_id;
        peer.stratum = stratum;
        peer.last_seen_us = now_us;
        peer.last_error_us = error_us;
        if peer.samples == 0 {
            peer.delay_us = delay_us;
            peer.jitter_us = delay_us / 4;
        } else {
            let d_err = delay_us - peer.delay_us;
            peer.delay_us += d_err / 8; // EWMA, shift 3
            let dev = (delay_us - peer.delay_us).abs();
            peer.jitter_us += (dev - peer.jitter_us) / 4; // EWMA, shift 2
        }
        peer.samples = peer.samples.saturating_add(1);
    }

    pub fn get(&self, id: NodeId) -> Option<&Peer> {
        self.peers.iter().find(|p| p.valid && p.id == id)
    }

    pub fn fresh_peers(&self, now_us: Micros) -> impl Iterator<Item = &Peer> {
        self.peers.iter().filter(move |p| p.fresh(now_us))
    }

    pub fn fresh_count(&self, now_us: Micros) -> u8 {
        self.fresh_peers(now_us).count() as u8
    }

    /// Smallest fresh-peer delay, for outlier rejection. None if no samples.
    pub fn best_delay_us(&self, now_us: Micros) -> Option<Micros> {
        self.fresh_peers(now_us)
            .filter(|p| p.samples > 0)
            .map(|p| p.delay_us)
            .min()
    }

    /// Distributed root election: the root is the lowest node id anyone can
    /// reach. Returns the chosen `(root_id, our_stratum)`.
    pub fn elect_root(&self, self_id: NodeId, now_us: Micros) -> (NodeId, u8) {
        let mut root = self_id;
        let mut stratum = 0u8;
        for peer in self.fresh_peers(now_us) {
            if peer.stratum >= MAX_STRATUM {
                continue;
            }
            let cand_root = peer.root_id;
            let cand_stratum = peer.stratum.saturating_add(1);
            if cand_root < root || (cand_root == root && cand_stratum < stratum) {
                root = cand_root;
                stratum = cand_stratum;
            }
        }
        (root, stratum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowest_id_becomes_root() {
        let mut table = PeerTable::new();
        // We are node 5. Peer 2 is the root (stratum 0); peer 8 is a leaf.
        table.observe_beacon(2, 2, 0, 1_000);
        table.observe_beacon(8, 2, 1, 1_000);
        let (root, stratum) = table.elect_root(5, 1_000);
        assert_eq!(root, 2);
        assert_eq!(stratum, 1); // one hop from root via peer 2
    }

    #[test]
    fn we_are_root_when_lowest() {
        let mut table = PeerTable::new();
        table.observe_beacon(7, 7, 0, 1_000);
        let (root, stratum) = table.elect_root(3, 1_000);
        assert_eq!(root, 3);
        assert_eq!(stratum, 0);
    }

    #[test]
    fn stale_peers_drop_from_election() {
        let mut table = PeerTable::new();
        table.observe_beacon(1, 1, 0, 1_000);
        // Long after the timeout, peer 1 no longer counts; node 4 elects itself.
        let (root, _) = table.elect_root(4, 1_000 + PEER_TIMEOUT_US + 1);
        assert_eq!(root, 4);
    }

    #[test]
    fn lower_delay_weighs_more() {
        let mut table = PeerTable::new();
        table.observe_probe(1, 1, 0, 500, 0, 1_000);
        table.observe_probe(2, 1, 0, 5_000, 0, 1_000);
        let near = table.get(1).unwrap().weight(1_000);
        let far = table.get(2).unwrap().weight(1_000);
        assert!(near > far);
    }
}
