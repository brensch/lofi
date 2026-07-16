//! Single-engine behaviour: root identity, the probe reply, and adopting a
//! lower-id root from a beacon. (Multi-node dynamics live in `mesh_convergence`.)

use lofi_core::mesh::wire::{Beacon, MeshMessage, ProbeRequest};
use lofi_core::mesh::SyncEngine;

#[test]
fn lone_node_is_its_own_root_and_schedules() {
    let mut engine = SyncEngine::new(5);
    assert!(engine.is_root());
    assert_eq!(engine.schedule_now(1_000_000), 1_000_000);
}

#[test]
fn answers_probe_addressed_to_us() {
    let mut engine = SyncEngine::new(1);
    let req = MeshMessage::ProbeRequest(ProbeRequest {
        sender: 2,
        target: 1,
        t1_local_us: 500,
        seq: 9,
    });
    match engine.handle(req, 1_200) {
        Some(MeshMessage::ProbeResponse(r)) => {
            assert_eq!(r.target, 2);
            assert_eq!(r.t1_local_us, 500);
            assert_eq!(r.t2_local_us, 1_200);
            assert_eq!(r.seq, 9);
        }
        other => panic!("expected probe response, got {other:?}"),
    }
}

#[test]
fn adopts_lower_id_root_from_beacon() {
    let mut engine = SyncEngine::new(5);
    let beacon = MeshMessage::Beacon(Beacon {
        sender: 2,
        root_id: 2,
        epoch: 2,
        stratum: 0,
        seq: 1,
        mesh_us: 9_000_000,
        rate_ppb: 0,
        root_dispersion_us: 0,
    });
    engine.handle(beacon, 1_000);
    assert!(!engine.is_root());
    assert_eq!(engine.quality(1_000).root_id, 2);
    assert_eq!(engine.quality(1_000).stratum, 1);
}
