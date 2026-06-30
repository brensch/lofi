use lofi_core::clock::{ClockModel, DisciplineConfig};
use lofi_core::event::{EventQueue, ScheduledEvent, Section};
use lofi_core::groove::{sample_i16 as groove_sample, GrooveConfig, GroovePart};
use lofi_core::protocol::{Frame, MessageKind};
use lofi_core::sequencer::BeepVoice;
use lofi_core::transport::Transport;
use lofi_core::{Micros, NodeId};

use crate::display::DisplayState;
use crate::peers::PeerTable;

/// Timing-grid period for the shared sync beep.
pub const BEAT_PERIOD_US: Micros = 500_000;
/// How long the sync beep sounds each grid point.
pub const BEEP_DURATION_US: Micros = 45_000;
/// Sync beep amplitude. Sits under the groove so it reads as a click, not a tone.
pub const BEEP_AMPLITUDE: i16 = 650;
/// Default audio render rate. Firmware sets this to the real I2S DAC rate.
pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;

const EVENT_CAPACITY: usize = 16;

/// Direction of a device's arpeggio role on the shared chord.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArpDirection {
    Up,
    Down,
}

impl ArpDirection {
    fn part(self) -> GroovePart {
        match self {
            ArpDirection::Up => GroovePart::ArpUp,
            ArpDirection::Down => GroovePart::ArpDown,
        }
    }
}

/// A device's musical identity in the mesh: its sync-beep pitch and arp role.
/// Panning/placement is a listener concern and lives in the simulator, not here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceVoice {
    pub beep_hz: u32,
    pub arp: ArpDirection,
}

impl DeviceVoice {
    pub const fn new(beep_hz: u32, arp: ArpDirection) -> Self {
        Self { beep_hz, arp }
    }
}

/// The whole audible behavior of one physical box.
///
/// This is the code that runs identically in firmware and in the simulator.
/// Firmware feeds it a monotonic hardware clock and an I2S DMA buffer; the
/// simulator feeds it a drifting virtual clock and a host audio buffer. Both
/// call exactly the same methods, so what you hear in the lab is what the
/// hardware will do.
#[derive(Clone, Debug)]
pub struct Device {
    id: NodeId,
    voice: DeviceVoice,
    sample_rate: u32,
    clock: ClockModel,
    transport: Transport,
    section: Section,
    seed: u64,
    events: EventQueue<EVENT_CAPACITY>,
    running: bool,
    sequence: u32,
    last_sync_error_us: Micros,
    peers: PeerTable,
}

impl Device {
    pub fn new(id: NodeId, voice: DeviceVoice, transport: Transport, seed: u64) -> Self {
        Self {
            id,
            voice,
            sample_rate: DEFAULT_SAMPLE_RATE,
            clock: ClockModel::new(),
            transport,
            section: Section::Groove,
            seed,
            events: EventQueue::new(),
            running: true,
            sequence: 0,
            last_sync_error_us: 0,
            peers: PeerTable::new(),
        }
    }

    pub fn with_sample_rate(mut self, sample_rate: u32) -> Self {
        self.sample_rate = sample_rate.max(1);
        self
    }

    pub const fn id(&self) -> NodeId {
        self.id
    }

    pub const fn voice(&self) -> DeviceVoice {
        self.voice
    }

    pub const fn is_running(&self) -> bool {
        self.running
    }

    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    pub fn toggle_running(&mut self) {
        self.running = !self.running;
    }

    pub const fn transport(&self) -> Transport {
        self.transport
    }

    pub const fn section(&self) -> Section {
        self.section
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn clock(&self) -> &ClockModel {
        &self.clock
    }

    /// Map this device's local hardware clock to shared mesh/root time.
    pub fn root_from_local(&self, local_us: Micros) -> Micros {
        self.clock.root_from_local(local_us)
    }

    /// Schedule an absolute, idempotent future event on the shared timeline.
    pub fn push_event(&mut self, event: ScheduledEvent) {
        let _ = self.events.push(event);
    }

    /// Build a sync beacon to broadcast. `local_us` is the send-time hardware clock.
    pub fn make_sync_frame(&mut self, local_us: Micros) -> Frame {
        self.sequence = self.sequence.wrapping_add(1);
        Frame {
            kind: MessageKind::Sync,
            node_id: self.id,
            root_id: 0,
            sequence: self.sequence,
            root_time_us: self.clock.root_from_local(local_us),
            beat_period_us: BEAT_PERIOD_US as u32,
            flags: 0,
        }
    }

    /// Consume a received frame: discipline the clock and note the peer.
    ///
    /// `local_rx_us` is the receive-time hardware clock; `observed_root_us` is
    /// the sender's root-time estimate corrected for the measured path delay.
    pub fn receive_frame(&mut self, frame: Frame, local_rx_us: Micros, observed_root_us: Micros) {
        if frame.kind != MessageKind::Sync || frame.node_id == self.id {
            return;
        }
        let cfg = DisciplineConfig {
            reject_offset_us: 750_000,
            ..DisciplineConfig::default()
        };
        let observation = self.clock.observe(local_rx_us, observed_root_us, cfg);
        if observation.accepted {
            self.last_sync_error_us = observation.error_us;
        }
        self.peers.note(frame.node_id, local_rx_us);
    }

    /// Render one mono audio block. `block_start_local_us` is the hardware clock
    /// at the first sample. Allocation-free and lock-free: safe for I2S DMA.
    pub fn render_audio(&mut self, out: &mut [i16], block_start_local_us: Micros) {
        let block_root = self.clock.root_from_local(block_start_local_us);
        self.apply_due_events(block_root);

        if !self.running {
            for sample in out.iter_mut() {
                *sample = 0;
            }
            return;
        }

        let sr = self.sample_rate as Micros;
        let beep = BeepVoice::new(BEAT_PERIOD_US, BEEP_DURATION_US, self.voice.beep_hz);
        for (ix, sample) in out.iter_mut().enumerate() {
            let local_us = block_start_local_us + (ix as Micros * 1_000_000 / sr);
            let root_us = self.clock.root_from_local(local_us);
            *sample = self.render_sample(root_us, beep);
        }
    }

    fn render_sample(&self, root_us: Micros, beep: BeepVoice) -> i16 {
        let timing = beep.sample_i16(root_us, self.sample_rate, BEEP_AMPLITUDE) as i32;
        let groove = self.groove(root_us, GroovePart::Drums) as i32
            + self.groove(root_us, GroovePart::Bass) as i32
            + self.groove(root_us, GroovePart::Harmony) as i32
            + self.groove(root_us, self.voice.arp.part()) as i32;
        (timing + groove).clamp(i16::MIN as i32, i16::MAX as i32) as i16
    }

    fn groove(&self, root_us: Micros, part: GroovePart) -> i16 {
        groove_sample(
            root_us,
            self.transport,
            self.section,
            GrooveConfig {
                sample_rate: self.sample_rate,
                seed: self.seed,
                part,
            },
        )
    }

    fn apply_due_events(&mut self, root_us: Micros) {
        let tick = self.transport.tick_at(root_us);
        while let Some(event) = self.events.pop_due(tick) {
            use lofi_core::event::EventAction;
            match event.action {
                EventAction::SetSection(section) => self.section = section,
                EventAction::SetSeed(seed) => self.seed = seed,
                EventAction::SetTempo { bpm_milli } => {
                    self.transport = self.transport.retimed(root_us, bpm_milli)
                }
            }
        }
    }

    pub fn peer_count(&self, now_local_us: Micros) -> u8 {
        self.peers.count_active(now_local_us)
    }

    /// Snapshot for the LCD. The same struct drives the real SSD1306 panel.
    pub fn display_state(&self, now_local_us: Micros) -> DisplayState {
        let root_us = self.clock.root_from_local(now_local_us);
        let tick = self.transport.tick_at(root_us);
        let ticks_per_bar = (self.transport.ticks_per_beat as i64) * 4;
        let beat_phase = tick.rem_euclid(ticks_per_bar.max(1));
        DisplayState {
            node_id: self.id as u32,
            playing: self.running,
            bpm_milli: self.transport.bpm_milli,
            section: self.section,
            peers: self.peer_count(now_local_us),
            sync_error_us: self.last_sync_error_us,
            beat_phase_milli: ((beat_phase * 1000) / ticks_per_bar.max(1)) as u16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device() -> Device {
        Device::new(
            1,
            DeviceVoice::new(880, ArpDirection::Up),
            Transport::default_at(0),
            1,
        )
    }

    #[test]
    fn stopped_device_is_silent() {
        let mut dev = device();
        dev.set_running(false);
        let mut buf = [123i16; 128];
        dev.render_audio(&mut buf, 0);
        assert!(buf.iter().all(|s| *s == 0));
    }

    #[test]
    fn running_device_is_audible() {
        let mut dev = device();
        let mut buf = [0i16; 256];
        dev.render_audio(&mut buf, 0);
        assert!(buf.iter().any(|s| *s != 0));
    }

    #[test]
    fn display_reflects_play_state() {
        let mut dev = device();
        assert!(dev.display_state(0).playing);
        dev.toggle_running();
        let state = dev.display_state(0);
        assert!(!state.playing);
        assert_eq!(state.node_id, 1);
        assert_eq!(state.peers, 0);
    }
}
