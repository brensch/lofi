use lofi_core::{Micros, NodeId};

const PEER_SLOTS: usize = 12;
const PEER_ACTIVE_WINDOW_US: Micros = 2_000_000;

/// Bounded, fixed-capacity record of recently heard peers. No allocation, so it
/// runs the same on hardware as in the simulator.
#[derive(Clone, Copy, Debug)]
pub struct PeerTable {
    slots: [PeerSlot; PEER_SLOTS],
}

impl Default for PeerTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerTable {
    pub const fn new() -> Self {
        Self {
            slots: [PeerSlot::EMPTY; PEER_SLOTS],
        }
    }

    /// Record that `id` was heard at local time `now_us`.
    pub fn note(&mut self, id: NodeId, now_us: Micros) {
        for slot in self.slots.iter_mut() {
            if slot.id == Some(id) {
                slot.last_local_us = now_us;
                return;
            }
        }
        // Reuse a free slot, else evict the stalest.
        let mut target = 0;
        for ix in 1..PEER_SLOTS {
            if self.slots[ix].id.is_none() {
                target = ix;
                break;
            }
            if self.slots[ix].last_local_us < self.slots[target].last_local_us {
                target = ix;
            }
        }
        self.slots[target] = PeerSlot {
            id: Some(id),
            last_local_us: now_us,
        };
    }

    /// Peers heard within the activity window as of `now_us`.
    pub fn count_active(&self, now_us: Micros) -> u8 {
        self.slots
            .iter()
            .filter(|slot| match slot.id {
                Some(_) => now_us - slot.last_local_us <= PEER_ACTIVE_WINDOW_US,
                None => false,
            })
            .count() as u8
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PeerSlot {
    id: Option<NodeId>,
    last_local_us: Micros,
}

impl PeerSlot {
    const EMPTY: Self = Self {
        id: None,
        last_local_us: 0,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_distinct_peers_in_window() {
        let mut peers = PeerTable::new();
        peers.note(2, 1_000);
        peers.note(3, 1_000);
        peers.note(2, 1_500); // refreshes, not a new peer
        assert_eq!(peers.count_active(1_500), 2);
    }

    #[test]
    fn stale_peers_drop_out() {
        let mut peers = PeerTable::new();
        peers.note(2, 1_000);
        assert_eq!(peers.count_active(1_000 + PEER_ACTIVE_WINDOW_US), 1);
        assert_eq!(peers.count_active(1_000 + PEER_ACTIVE_WINDOW_US + 1), 0);
    }
}
