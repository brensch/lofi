//! Host simulation kernel for the lofi mesh groovebox.
//!
//! The kernel drives real [`lofi_app::Device`] instances through a simulated
//! ESP-NOW network (drift, loss, latency) and mixes their mono output into a
//! stereo monitor bus. The same [`sim::Simulation`] powers both the batch WAV
//! renderer and the realtime/UI front end, so what you hear matches firmware.

pub mod node;
pub mod rng;
pub mod sim;
pub mod wav;

pub use node::{MixParams, NodeSnapshot, GROUP_SIZE, SAMPLE_RATE};
pub use sim::{PhaseStats, Simulation, DEFAULT_GROUP_JOIN_US, DEFAULT_SYNC_START_US};
