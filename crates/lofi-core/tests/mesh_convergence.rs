//! System-level validation of the mesh sync protocol: independent `SyncEngine`s
//! exchanging real wire messages over a simulated lossy, latent, drifting link.
//! Asserts the properties the music depends on — convergence, monotonic mesh
//! time, self-healing root failover, and clean cluster merge.

use lofi_core::mesh::wire::MeshMessage;
use lofi_core::mesh::SyncEngine;
use lofi_core::{Micros, NodeId};

struct Node {
    id: NodeId,
    drift_ppb: i64,
    offset_us: Micros,
    engine: SyncEngine,
    alive: bool,
}

impl Node {
    fn local(&self, global_us: Micros) -> Micros {
        global_us
            + self.offset_us
            + ((global_us as i128 * self.drift_ppb as i128) / 1_000_000_000) as Micros
    }
}

struct Pending {
    rx_global_us: Micros,
    dst: NodeId,
    src: NodeId,
    msg: MeshMessage,
}

struct Harness {
    nodes: Vec<Node>,
    pending: Vec<Pending>,
    reach: Vec<Vec<bool>>,
    global_us: Micros,
    rng: u64,
    loss_pct: u64,
}

impl Harness {
    fn new(specs: &[(NodeId, i64, Micros)], loss_pct: u64) -> Self {
        let nodes: Vec<Node> = specs
            .iter()
            .map(|&(id, drift_ppb, offset_us)| Node {
                id,
                drift_ppb,
                offset_us,
                engine: SyncEngine::new(id),
                alive: true,
            })
            .collect();
        let n = nodes.len();
        Self {
            nodes,
            pending: Vec::new(),
            reach: vec![vec![true; n]; n],
            global_us: 0,
            rng: 0x1234_5678,
            loss_pct,
        }
    }

    fn rand(&mut self, upper: u64) -> u64 {
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.rng >> 33) % upper.max(1)
    }

    fn index(&self, id: NodeId) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    fn reachable(&self, src: NodeId, dst: NodeId) -> bool {
        let (Some(s), Some(d)) = (self.index(src), self.index(dst)) else {
            return false;
        };
        self.nodes[s].alive && self.nodes[d].alive && self.reach[s][d]
    }

    fn set_partition(&mut self, group_a: &[NodeId]) {
        for s in 0..self.nodes.len() {
            for d in 0..self.nodes.len() {
                let a_has_s = group_a.contains(&self.nodes[s].id);
                let a_has_d = group_a.contains(&self.nodes[d].id);
                self.reach[s][d] = a_has_s == a_has_d;
            }
        }
    }

    fn heal_partition(&mut self) {
        let n = self.nodes.len();
        self.reach = vec![vec![true; n]; n];
    }

    fn enqueue(&mut self, src: NodeId, dst: NodeId, msg: MeshMessage) {
        if !self.reachable(src, dst) || self.rand(100) < self.loss_pct {
            return;
        }
        let latency = 500 + self.rand(2_000) as Micros;
        self.pending.push(Pending {
            rx_global_us: self.global_us + latency,
            dst,
            src,
            msg,
        });
    }

    fn run(&mut self, duration_us: Micros) {
        let end = self.global_us + duration_us;
        while self.global_us < end {
            self.tick();
            self.global_us += 1_000;
        }
    }

    fn tick(&mut self) {
        // Outgoing beacons + probes.
        for ix in 0..self.nodes.len() {
            if !self.nodes[ix].alive {
                continue;
            }
            let id = self.nodes[ix].id;
            let local = self.nodes[ix].local(self.global_us);
            if let Some(beacon) = self.nodes[ix].engine.due_beacon(local) {
                let others: Vec<NodeId> = self
                    .nodes
                    .iter()
                    .map(|n| n.id)
                    .filter(|o| *o != id)
                    .collect();
                for dst in others {
                    self.enqueue(id, dst, beacon);
                }
            }
            if let Some((dst, probe)) = self.nodes[ix].engine.due_probe(local) {
                self.enqueue(id, dst, probe);
            }
        }

        // Deliver due messages.
        let mut ix = 0;
        while ix < self.pending.len() {
            if self.pending[ix].rx_global_us > self.global_us {
                ix += 1;
                continue;
            }
            let p = self.pending.swap_remove(ix);
            let Some(d) = self.index(p.dst) else { continue };
            if !self.nodes[d].alive || !self.reachable(p.src, p.dst) {
                continue;
            }
            let rx_local = self.nodes[d].local(p.rx_global_us);
            if let Some(reply) = self.nodes[d].engine.handle(p.msg, rx_local) {
                let dst = reply_target(&reply);
                if let Some(dst) = dst {
                    self.enqueue(p.dst, dst, reply);
                }
            }
        }
    }

    fn mesh_spread(&self, ids: &[NodeId]) -> Micros {
        let times: Vec<Micros> = self
            .nodes
            .iter()
            .filter(|n| n.alive && ids.contains(&n.id))
            .map(|n| n.engine.mesh_from_local(n.local(self.global_us)))
            .collect();
        let min = *times.iter().min().unwrap();
        let max = *times.iter().max().unwrap();
        max - min
    }

    fn root_of(&self, id: NodeId) -> NodeId {
        let ix = self.index(id).unwrap();
        self.nodes[ix]
            .engine
            .quality(self.nodes[ix].local(self.global_us))
            .root_id
    }
}

fn reply_target(msg: &MeshMessage) -> Option<NodeId> {
    match msg {
        MeshMessage::ProbeResponse(r) => Some(r.target),
        _ => None,
    }
}

const ALL: &[NodeId] = &[1, 2, 3, 4, 5];

fn drifting_swarm(loss_pct: u64) -> Harness {
    Harness::new(
        &[
            (1, 0, 0),
            (2, 90_000, 140_000),
            (3, -60_000, -90_000),
            (4, 45_000, 220_000),
            (5, -110_000, 60_000),
        ],
        loss_pct,
    )
}

#[test]
fn converges_under_drift_and_loss() {
    let mut h = drifting_swarm(10);
    h.run(45_000_000);
    let spread = h.mesh_spread(ALL);
    assert!(spread < 3_000, "mesh spread too wide: {spread}us");
    for id in ALL {
        assert_eq!(h.root_of(*id), 1, "node {id} should follow root 1");
    }
}

#[test]
fn scheduling_time_is_continuous_across_the_swarm() {
    let mut h = drifting_swarm(15);
    let mut last = vec![Micros::MIN; h.nodes.len()];
    let mut last_local = vec![Micros::MIN; h.nodes.len()];
    for _ in 0..600 {
        h.run(50_000);
        for ((node, previous), previous_local) in
            h.nodes.iter_mut().zip(&mut last).zip(&mut last_local)
        {
            let local = node.local(h.global_us);
            let now = node.engine.schedule_now(local);
            assert!(now >= *previous, "node {} mesh went backwards", node.id);
            if *previous != Micros::MIN {
                let local_advance = local - *previous_local;
                let mesh_advance = now - *previous;
                let tolerance = (local_advance / 100).max(2);
                assert!(
                    (mesh_advance - local_advance).abs() <= tolerance,
                    "node {} scheduling jump: local +{local_advance}us, mesh +{mesh_advance}us",
                    node.id,
                );
            }
            *previous = now;
            *previous_local = local;
        }
    }
}

#[test]
fn relocks_after_offset_step() {
    let mut h = drifting_swarm(8);
    h.run(40_000_000);
    assert!(h.mesh_spread(ALL) < 3_000);

    // Teleport node 3's hardware clock +250ms (a UI offset-slider drag). The
    // step exceeds the per-sample reject threshold, so it must trip the stepout
    // re-lock rather than rejecting every probe forever.
    let ix = h.index(3).unwrap();
    h.nodes[ix].offset_us += 250_000;
    h.run(20_000_000);
    assert!(
        h.mesh_spread(ALL) < 4_000,
        "did not re-lock after offset step"
    );
}

#[test]
fn heals_after_root_failure() {
    let mut h = drifting_swarm(8);
    h.run(40_000_000);
    assert_eq!(h.root_of(2), 1);
    // Kill the root; node 2 (next lowest id) should take over and the rest
    // re-converge onto it.
    h.nodes[0].alive = false;
    h.run(40_000_000);
    let survivors = &[2, 3, 4, 5];
    for id in survivors {
        assert_eq!(h.root_of(*id), 2, "node {id} should adopt new root 2");
    }
    assert!(h.mesh_spread(survivors) < 4_000);
}

#[test]
fn splits_and_merges_cleanly() {
    let mut h = drifting_swarm(8);
    h.set_partition(&[1, 2]);
    h.run(35_000_000);
    // Each side has converged internally onto its own lowest-id root.
    assert_eq!(h.root_of(2), 1);
    assert_eq!(h.root_of(4), 3);
    assert!(h.mesh_spread(&[1, 2]) < 3_000);
    assert!(h.mesh_spread(&[3, 4, 5]) < 3_000);

    // Merge: the whole swarm must agree on root 1.
    h.heal_partition();
    h.run(45_000_000);
    for id in ALL {
        assert_eq!(
            h.root_of(*id),
            1,
            "post-merge node {id} should follow root 1"
        );
    }
    assert!(h.mesh_spread(ALL) < 4_000, "post-merge spread too wide");
}
