use lofi_app::{ArpDirection, Device, DeviceVoice, DisplayState};
use lofi_core::transport::Transport;
use lofi_core::{Micros, NodeId};

use crate::rng::Lcg;

pub const SAMPLE_RATE: u32 = 48_000;
pub const GROUP_SIZE: usize = 4;

const DEVICE_BEEP_HZ: [u32; 8] = [1_320, 1_100, 880, 660, 440, 392, 330, 294];
const ARP_DIRS: [ArpDirection; 8] = [
    ArpDirection::Up,
    ArpDirection::Down,
    ArpDirection::Down,
    ArpDirection::Up,
    ArpDirection::Down,
    ArpDirection::Up,
    ArpDirection::Up,
    ArpDirection::Down,
];

/// Listener-side mixing for one box. These are room-placement and monitoring
/// controls in the simulator, not part of the device itself (a real box just
/// drives one mono speaker).
#[derive(Clone, Copy, Debug)]
pub struct MixParams {
    /// -1.0 hard left .. +1.0 hard right.
    pub pan: f32,
    pub gain: f32,
    pub mute: bool,
    pub solo: bool,
}

impl MixParams {
    /// Equal-power left/right gains for this pan position.
    pub fn lr_gains(&self) -> (f32, f32) {
        let angle = (self.pan.clamp(-1.0, 1.0) + 1.0) * (core::f32::consts::FRAC_PI_4);
        (angle.cos() * self.gain, angle.sin() * self.gain)
    }
}

/// One simulated box: the real device runtime, the "hardware truth" of how its
/// local oscillator drifts from the simulator's global clock, and listener-side
/// mix controls.
#[derive(Debug)]
pub struct NodeSim {
    pub device: Device,
    pub drift_ppb: i32,
    pub local_offset_us: Micros,
    pub mix: MixParams,
}

impl NodeSim {
    pub fn new(ix: usize, transport: Transport, seed: u64, rng: &mut Lcg) -> Self {
        let id = ix as NodeId + 1;
        let (drift_ppb, local_offset_us) = drift_and_offset(ix, id, rng);
        let voice = DeviceVoice::new(
            DEVICE_BEEP_HZ[ix % DEVICE_BEEP_HZ.len()],
            ARP_DIRS[ix % ARP_DIRS.len()],
        );
        Self {
            device: Device::new(id, voice, transport, seed).with_sample_rate(SAMPLE_RATE),
            drift_ppb,
            local_offset_us,
            mix: MixParams {
                pan: if pans_left(ix) { -1.0 } else { 1.0 },
                gain: 1.0,
                mute: false,
                solo: false,
            },
        }
    }

    /// Hardware-local microseconds at a given global time.
    pub fn local_time(&self, global_us: Micros) -> Micros {
        global_us
            + self.local_offset_us
            + ((global_us as i128 * self.drift_ppb as i128) / 1_000_000_000) as Micros
    }

    pub fn snapshot(&self, global_us: Micros) -> NodeSnapshot {
        let local = self.local_time(global_us);
        NodeSnapshot {
            display: self.device.display_state(local),
            mix: self.mix,
            drift_ppb: self.drift_ppb,
            local_offset_us: self.local_offset_us,
        }
    }
}

/// Per-node state the UI renders each frame.
#[derive(Clone, Copy, Debug)]
pub struct NodeSnapshot {
    pub display: DisplayState,
    pub mix: MixParams,
    pub drift_ppb: i32,
    pub local_offset_us: Micros,
}

pub fn drift_and_offset(ix: usize, id: NodeId, rng: &mut Lcg) -> (i32, Micros) {
    let drift_ppb = if id == 1 {
        0
    } else if id == 2 {
        120_000
    } else if ix == GROUP_SIZE {
        -40_000
    } else {
        rng.range_i32(-120_000, 120_000)
    };
    let local_offset_us = if id == 1 {
        0
    } else if id == 2 {
        140_000
    } else if ix == GROUP_SIZE {
        520_000
    } else {
        let group_bias = if ix < GROUP_SIZE { 0 } else { 520_000 };
        group_bias + rng.range_i64(-180_000, 180_000)
    };
    (drift_ppb, local_offset_us)
}

pub fn same_group(source: usize, target: usize) -> bool {
    source / GROUP_SIZE == target / GROUP_SIZE
}

pub fn pans_left(ix: usize) -> bool {
    (ix / GROUP_SIZE) % 2 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mix(pan: f32) -> MixParams {
        MixParams {
            pan,
            gain: 1.0,
            mute: false,
            solo: false,
        }
    }

    #[test]
    fn equal_power_pan() {
        let (l, r) = mix(-1.0).lr_gains();
        assert!(l > 0.99 && r < 0.01);
        let (l, r) = mix(1.0).lr_gains();
        assert!(l < 0.01 && r > 0.99);
        let (l, r) = mix(0.0).lr_gains();
        assert!((l - r).abs() < 0.001 && (l - 0.707).abs() < 0.01);
    }
}
