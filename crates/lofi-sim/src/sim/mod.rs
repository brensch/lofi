use lofi_core::event::{EventAction, ScheduledEvent, Section};
use lofi_core::mesh::wire::MeshMessage;
use lofi_core::transport::Transport;
use lofi_core::{Micros, NodeId};

use crate::node::{same_group, NodeSim, SAMPLE_RATE};
use crate::rng::Lcg;
use crate::wav::StereoSample;

mod control;

pub const DEFAULT_SYNC_START_US: Micros = 2_500_000;
pub const DEFAULT_GROUP_JOIN_US: Micros = 8_000_000;
const SONG_ZERO_US: Micros = 2_000_000;
const NET_STEP_US: Micros = 250;
const RENDER_BLOCK: usize = 64;
/// Ticks from "now" to the scheduled demo drop (8 bars at 96 ticks/beat, 4/4).
const DEMO_DROP_TICKS: i64 = 96 * 8;

#[derive(Debug)]
pub struct Simulation {
    global_us: Micros,
    us_acc: i64,
    sync_start_us: Micros,
    group_join_us: Micros,
    sync_enabled: bool,
    transport: Transport,
    rng: Lcg,
    nodes: Vec<NodeSim>,
    pending: Vec<PendingMsg>,
    seed: u64,
}

impl Simulation {
    pub fn new(node_count: usize, seed: u64, sync_start_us: Micros, group_join_us: Micros) -> Self {
        // 80 BPM matches the phase-aligned source loops without resampling.
        let transport = Transport::new(
            SONG_ZERO_US,
            80_000,
            lofi_core::transport::DEFAULT_TICKS_PER_BEAT,
        );
        let mut rng = Lcg::new(seed);
        let mut nodes = Vec::with_capacity(node_count);
        for ix in 0..node_count {
            nodes.push(NodeSim::new(ix, transport, seed, &mut rng));
        }

        Self {
            global_us: 0,
            us_acc: 0,
            sync_start_us,
            group_join_us,
            sync_enabled: true,
            transport,
            rng,
            nodes,
            pending: Vec::new(),
            seed,
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn global_us(&self) -> Micros {
        self.global_us
    }

    /// Schedule the demo drop + seed change on every device's shared timeline.
    pub fn schedule_demo_drop(&mut self) {
        let drop_tick = self.transport.tick_at(self.global_us) + DEMO_DROP_TICKS;
        let new_seed = self.seed ^ 0xd012_d012_d012_d012;
        for node in &mut self.nodes {
            node.device.push_event(ScheduledEvent {
                fire_at_tick: drop_tick,
                action: EventAction::SetSection(Section::Drop),
                id: 1,
            });
            node.device.push_event(ScheduledEvent {
                fire_at_tick: drop_tick,
                action: EventAction::SetSeed(new_seed),
                id: 2,
            });
        }
    }

    /// Advance only the network/clock model (no audio), e.g. to let sync settle.
    pub fn run(&mut self, duration_us: Micros) {
        let end_us = self.global_us + duration_us;
        while self.global_us < end_us {
            self.step_network();
            self.global_us = (self.global_us + NET_STEP_US).min(end_us);
        }
    }

    /// Render the whole duration into a fresh buffer (batch / WAV path).
    pub fn render(&mut self, duration_us: Micros) -> Vec<StereoSample> {
        let count = (duration_us as u128 * SAMPLE_RATE as u128 / 1_000_000) as usize;
        let mut samples = vec![StereoSample { left: 0, right: 0 }; count];
        self.fill(&mut samples);
        samples
    }

    /// Fill a stereo buffer in place, advancing audio and network in lockstep.
    /// The realtime engine calls this from its producer thread; the WAV path
    /// calls it once. Either way every device renders through the same code it
    /// runs on hardware.
    pub fn fill(&mut self, out: &mut [StereoSample]) {
        let mut scratch = [0i16; RENDER_BLOCK];
        let mut left = [0f32; RENDER_BLOCK];
        let mut right = [0f32; RENDER_BLOCK];
        let mut done = 0;
        while done < out.len() {
            let n = RENDER_BLOCK.min(out.len() - done);
            let block_start = self.global_us;
            left[..n].fill(0.0);
            right[..n].fill(0.0);

            let any_solo = self.nodes.iter().any(|node| node.mix.solo);
            let mut lnorm = 0.0f32;
            let mut rnorm = 0.0f32;
            for node in &mut self.nodes {
                let block_local = node.local_time(block_start);
                node.device.render_audio(&mut scratch[..n], block_local);
                if node.mix.mute || (any_solo && !node.mix.solo) {
                    continue;
                }
                let (lg, rg) = node.mix.lr_gains();
                lnorm += lg;
                rnorm += rg;
                for i in 0..n {
                    let s = scratch[i] as f32;
                    left[i] += s * lg;
                    right[i] += s * rg;
                }
            }

            let ln = lnorm.max(1.0);
            let rn = rnorm.max(1.0);
            for i in 0..n {
                out[done + i] = StereoSample {
                    left: clamp_f32(left[i] / ln),
                    right: clamp_f32(right[i] / rn),
                };
            }

            self.advance_network(n);
            done += n;
        }
    }

    /// Advance the network model across one rendered block's worth of time.
    fn advance_network(&mut self, frames: usize) {
        self.us_acc += frames as i64 * 1_000_000;
        let dur = self.us_acc / SAMPLE_RATE as i64;
        self.us_acc -= dur * SAMPLE_RATE as i64;
        let target = self.global_us + dur;
        while self.global_us < target {
            self.step_network();
            self.global_us = (self.global_us + NET_STEP_US).min(target);
        }
    }

    fn step_network(&mut self) {
        if self.sync_enabled && self.global_us >= self.sync_start_us {
            self.emit_sync_traffic();
        }
        self.deliver_due();
    }

    /// Pull each device's due beacons (broadcast) and probes (unicast) and put
    /// them on the wire. The devices schedule their own cadence; the sim only
    /// models the radio (reachability, loss, latency).
    fn emit_sync_traffic(&mut self) {
        for ix in 0..self.nodes.len() {
            let local = self.nodes[ix].local_time(self.global_us);
            if let Some(beacon) = self.nodes[ix].device.poll_beacon(local) {
                for target in 0..self.nodes.len() {
                    if target != ix {
                        self.send(ix, target, beacon);
                    }
                }
            }
            if let Some((dst_id, probe)) = self.nodes[ix].device.poll_probe(local) {
                if let Some(target) = self.index_of(dst_id) {
                    self.send(ix, target, probe);
                }
            }
        }
    }

    fn send(&mut self, source: usize, target: usize, msg: MeshMessage) {
        if !self.link_active(source, target)
            || self.rng.next_bounded(100) < self.loss_percent(source, target)
        {
            return;
        }
        let latency_us = self.rng.range_i64(700, 4_500);
        self.pending.push(PendingMsg {
            rx_global_us: self.global_us + latency_us,
            source,
            target,
            msg,
        });
    }

    fn index_of(&self, id: NodeId) -> Option<usize> {
        self.nodes.iter().position(|n| n.device.id() == id)
    }

    fn link_active(&self, source: usize, target: usize) -> bool {
        same_group(source, target) || self.global_us >= self.group_join_us
    }

    fn loss_percent(&self, source: usize, target: usize) -> u64 {
        if same_group(source, target) {
            4
        } else {
            8
        }
    }

    fn deliver_due(&mut self) {
        let mut ix = 0;
        while ix < self.pending.len() {
            if self.pending[ix].rx_global_us > self.global_us {
                ix += 1;
                continue;
            }
            let pending = self.pending.swap_remove(ix);
            if pending.target >= self.nodes.len()
                || !self.link_active(pending.source, pending.target)
            {
                continue;
            }
            let rx_global = pending.rx_global_us.max(self.global_us);
            let node = &mut self.nodes[pending.target];
            let rx_local = node.local_time(rx_global);
            if let Some(reply) = node.device.handle(pending.msg, rx_local) {
                if let Some(dst) = reply_target(&reply).and_then(|id| self.index_of(id)) {
                    self.send(pending.target, dst, reply);
                }
            }
        }
    }

    pub fn phase_stats(&self) -> PhaseStats {
        self.phase_stats_for(0..self.nodes.len())
    }

    pub fn phase_stats_for(&self, range: std::ops::Range<usize>) -> PhaseStats {
        let start = range.start.min(self.nodes.len());
        let end = range.end.min(self.nodes.len()).max(start);
        let times: Vec<Micros> = self.nodes[start..end]
            .iter()
            .map(|node| node.device.mesh_from_local(node.local_time(self.global_us)))
            .collect();
        let min = *times.iter().min().unwrap_or(&0);
        let max = *times.iter().max().unwrap_or(&0);
        let mean = times.iter().sum::<Micros>() / times.len().max(1) as Micros;
        let mean_abs_error_us =
            times.iter().map(|v| (v - mean).abs()).sum::<Micros>() / times.len().max(1) as Micros;
        PhaseStats {
            max_spread_us: max - min,
            mean_abs_error_us,
        }
    }
}

fn clamp_f32(v: f32) -> i16 {
    v.clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

#[derive(Clone, Debug)]
struct PendingMsg {
    rx_global_us: Micros,
    source: usize,
    target: usize,
    msg: MeshMessage,
}

/// Where a reply (probe response) should be routed back to.
fn reply_target(msg: &MeshMessage) -> Option<NodeId> {
    match msg {
        MeshMessage::ProbeResponse(r) => Some(r.target),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PhaseStats {
    pub max_spread_us: Micros,
    pub mean_abs_error_us: Micros,
}
