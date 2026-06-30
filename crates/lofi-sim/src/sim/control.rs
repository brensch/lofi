//! Live control surface for the simulation, driven by the UI. Split out from
//! the kernel so `mod.rs` stays focused on the network/audio model. As a child
//! module it can reach `Simulation`'s private fields directly.

use lofi_core::event::{EventAction, ScheduledEvent, Section};
use lofi_core::Micros;

use super::{Simulation, DEMO_DROP_TICKS};
use crate::node::{MixParams, NodeSim, NodeSnapshot};

impl Simulation {
    pub fn add_node(&mut self) {
        let ix = self.nodes.len();
        let mut node = NodeSim::new(ix, self.transport, self.seed, &mut self.rng);
        // A late joiner should still get the scheduled drop on the shared grid.
        let drop_tick = self.transport.tick_at(self.global_us) + DEMO_DROP_TICKS;
        node.device.push_event(ScheduledEvent {
            fire_at_tick: drop_tick,
            action: EventAction::SetSection(Section::Drop),
            id: 1,
        });
        self.nodes.push(node);
    }

    pub fn remove_node(&mut self, ix: usize) {
        if ix < self.nodes.len() {
            self.nodes.remove(ix);
            self.pending.retain(|p| p.target != ix);
            for p in &mut self.pending {
                if p.target > ix {
                    p.target -= 1;
                }
            }
        }
    }

    pub fn toggle_running(&mut self, ix: usize) {
        if let Some(node) = self.nodes.get_mut(ix) {
            node.device.toggle_running();
        }
    }

    pub fn set_all_running(&mut self, running: bool) {
        for node in &mut self.nodes {
            node.device.set_running(running);
        }
    }

    pub fn set_sync_enabled(&mut self, enabled: bool) {
        self.sync_enabled = enabled;
    }

    pub fn sync_enabled(&self) -> bool {
        self.sync_enabled
    }

    pub fn with_node_mix(&mut self, ix: usize, f: impl FnOnce(&mut MixParams)) {
        if let Some(node) = self.nodes.get_mut(ix) {
            f(&mut node.mix);
        }
    }

    pub fn set_drift_ppb(&mut self, ix: usize, ppb: i32) {
        if let Some(node) = self.nodes.get_mut(ix) {
            node.drift_ppb = ppb;
        }
    }

    pub fn set_offset_us(&mut self, ix: usize, offset_us: Micros) {
        if let Some(node) = self.nodes.get_mut(ix) {
            node.local_offset_us = offset_us;
        }
    }

    pub fn snapshot(&self) -> Vec<NodeSnapshot> {
        self.nodes
            .iter()
            .map(|node| node.snapshot(self.global_us))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::Simulation;

    #[test]
    fn add_and_remove_change_node_count() {
        let mut sim = Simulation::new(8, 1, 0, 0);
        assert_eq!(sim.node_count(), 8);
        sim.add_node();
        assert_eq!(sim.node_count(), 9);
        sim.remove_node(0);
        assert_eq!(sim.node_count(), 8);
    }

    #[test]
    fn muting_every_node_renders_silence() {
        let mut sim = Simulation::new(8, 1, 0, 8_000_000);
        for ix in 0..sim.node_count() {
            sim.with_node_mix(ix, |m| m.mute = true);
        }
        let buf = sim.render(200_000);
        assert!(!buf.is_empty());
        assert!(buf.iter().all(|s| s.left == 0 && s.right == 0));
    }

    #[test]
    fn solo_isolates_one_node() {
        let mut sim = Simulation::new(8, 1, 0, 8_000_000);
        // Solo a node that is silent (stopped): output must be silent too.
        sim.set_all_running(false);
        sim.with_node_mix(0, |m| m.solo = true);
        sim.toggle_running(1); // a non-soloed node is running but must be excluded
        let buf = sim.render(200_000);
        assert!(buf.iter().all(|s| s.left == 0 && s.right == 0));
    }
}
