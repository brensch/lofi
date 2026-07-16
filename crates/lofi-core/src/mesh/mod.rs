//! Production mesh time synchronization.
//!
//! A leaderless, self-healing protocol that keeps a swarm of boxes on one
//! musical timeline over a lossy, variable-latency ESP-NOW link, with no
//! infrastructure and no fixed leader.
//!
//! Shape:
//! - [`wire`]: the on-air messages (beacon + NTP-style probe exchange).
//! - [`clock::MeshClock`]: the disciplined, monotonic scheduling clock.
//! - [`peer::PeerTable`]: per-peer delay/jitter/weight with outlier rejection.
//! - [`SyncEngine`]: ties them together — root selection, weighted discipline,
//!   and message scheduling. Firmware feeds it timestamped frames and asks it
//!   for mesh time; the simulator does the same.

pub mod clock;
pub mod peer;
pub mod wire;

mod engine;

pub use engine::{RosterView, SyncEngine, SyncQuality};
