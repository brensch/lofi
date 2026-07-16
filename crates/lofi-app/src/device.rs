use lofi_core::event::{EventQueue, ScheduledEvent, Section};
use lofi_core::mesh::wire::MeshMessage;
use lofi_core::mesh::{SyncEngine, SyncQuality};
use lofi_core::music::arrangement::{Arrangement, Role, BARS_PER_PHRASE, ROLES};
use lofi_core::music::kit::kit_for;
use lofi_core::music::{
    color, render_role, signature_for, BeatCtx, BeatEvolution, LoopScene, Lowpass, PackedCatalog,
    AI_CATALOG,
};
use lofi_core::transport::Transport;
use lofi_core::{Micros, NodeId};

use crate::display::DisplayState;

/// Default audio render rate. Firmware sets this to the real I2S DAC rate.
pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;
/// Ticks per bar (16 sixteenth-steps × 24 ticks at 96 ticks/beat).
const TICKS_PER_BAR: i64 = 384;
/// Master lowpass cutoff — rolls off the highs for the lofi tone.
const LOWPASS_HZ: f32 = 3_600.0;
const BASS_LOWPASS_HZ: f32 = 420.0;
/// Peak headroom when converting the f32 mix to i16.
const OUTPUT_AMPLITUDE: f32 = 18_000.0;

const EVENT_CAPACITY: usize = 16;

/// Legacy arpeggio hint kept for simulator/device construction compatibility.
/// Procedural role assignment now comes from the mesh roster.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArpDirection {
    Up,
    Down,
}

/// A device's legacy musical identity. Panning/placement is a listener concern
/// and lives in the simulator, not here.
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
    catalog: &'static PackedCatalog,
    scene: LoopScene,
    engine: SyncEngine,
    transport: Transport,
    section: Section,
    seed: u64,
    events: EventQueue<EVENT_CAPACITY>,
    running: bool,
    lowpass: Lowpass,
    bass_lowpass: Lowpass,
    lowpass_cutoff_hz: f32,
}

impl Device {
    pub fn new(id: NodeId, voice: DeviceVoice, transport: Transport, seed: u64) -> Self {
        Self {
            id,
            voice,
            sample_rate: DEFAULT_SAMPLE_RATE,
            catalog: &AI_CATALOG,
            scene: AI_CATALOG
                .loop_scene(seed)
                .expect("catalog has no coherent loop scene"),
            engine: SyncEngine::new(id),
            transport,
            section: Section::Groove,
            seed,
            events: EventQueue::new(),
            running: true,
            lowpass: Lowpass::new(LOWPASS_HZ, DEFAULT_SAMPLE_RATE, 0.707),
            bass_lowpass: Lowpass::new(BASS_LOWPASS_HZ, DEFAULT_SAMPLE_RATE, 0.707),
            lowpass_cutoff_hz: LOWPASS_HZ,
        }
    }

    pub fn with_sample_rate(mut self, sample_rate: u32) -> Self {
        self.sample_rate = sample_rate.max(1);
        self.lowpass = Lowpass::new(self.lowpass_cutoff_hz, self.sample_rate, 0.707);
        self.bass_lowpass = Lowpass::new(BASS_LOWPASS_HZ, self.sample_rate, 0.707);
        self
    }

    /// Point playback at a catalogue in memory-mapped read-only flash.
    pub fn with_catalog(mut self, catalog: &'static PackedCatalog) -> Self {
        assert!(catalog.is_valid(), "invalid sample catalogue");
        self.catalog = catalog;
        self.scene = catalog
            .loop_scene(self.seed)
            .expect("catalog has no coherent loop scene");
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

    /// The mesh sync state machine. Firmware/sim drive its network I/O.
    pub fn engine(&mut self) -> &mut SyncEngine {
        &mut self.engine
    }

    /// Map this device's local hardware clock to shared mesh time (measurement).
    pub fn mesh_from_local(&self, local_us: Micros) -> Micros {
        self.engine.mesh_from_local(local_us)
    }

    /// Schedule an absolute, idempotent future event on the shared timeline.
    pub fn push_event(&mut self, event: ScheduledEvent) {
        let _ = self.events.push(event);
    }

    /// A beacon to broadcast now, if due. `local_us` is the hardware clock.
    pub fn poll_beacon(&mut self, local_us: Micros) -> Option<MeshMessage> {
        self.engine.due_beacon(local_us)
    }

    /// A probe to unicast to an upstream peer now, if due: `(destination, msg)`.
    pub fn poll_probe(&mut self, local_us: Micros) -> Option<(NodeId, MeshMessage)> {
        self.engine.due_probe(local_us)
    }

    /// Ingest a received mesh frame, returning a reply to unicast back if any.
    /// `rx_local_us` is the receive-time hardware clock.
    pub fn handle(&mut self, msg: MeshMessage, rx_local_us: Micros) -> Option<MeshMessage> {
        self.engine.handle(msg, rx_local_us)
    }

    /// Render one mono audio block. `block_start_local_us` is the hardware clock
    /// at the first sample. Allocation-free and lock-free: safe for I2S DMA.
    pub fn render_audio(&mut self, out: &mut [i16], block_start_local_us: Micros) {
        let block_mesh = self.engine.schedule_now(block_start_local_us);
        self.apply_due_events(block_mesh);

        if !self.running {
            for sample in out.iter_mut() {
                *sample = 0;
            }
            self.lowpass.reset();
            self.bass_lowpass.reset();
            return;
        }

        // Resolve the shared arrangement, this box's roles, and the vibe (kit)
        // once per block. The kit is chosen deterministically from the seed, so
        // every box in the mesh selects the same content and tone profile.
        let roster = self.engine.roster(block_start_local_us);
        let phrase = self
            .transport
            .tick_at(block_mesh)
            .div_euclid(TICKS_PER_BAR * BARS_PER_PHRASE);
        let arrangement = Arrangement::at(self.seed, roster.ids(), phrase);
        let previous_arrangement = Arrangement::at(self.seed, roster.ids(), phrase - 1);
        let kit = kit_for(self.seed);
        let signature = signature_for(self.seed);
        let mut roles = [false; ROLES.len()];
        for (j, role) in ROLES.iter().enumerate() {
            roles[j] = role.assigned_to(roster.my_index(), roster.len());
        }
        let ctx = BeatCtx::new(self.transport, self.scene).with_evolution(BeatEvolution {
            previous: previous_arrangement.params,
            current: arrangement.params,
            phrase,
        });

        // The vibe's master lowpass; retune the biquad only when the vibe changes.
        let mut tone = signature.blend_tone(kit.tone);
        self.retune_lowpass(tone.cutoff_hz);
        // Blend the kit's baseline air with the arrangement's `dust` feature so
        // "Dusty" audibly lifts the noise bed without swamping quieter vibes.
        tone.air *= arrangement.params.dust as f32 * 0.5 + 0.4;

        let sr = self.sample_rate as Micros;
        for (ix, sample) in out.iter_mut().enumerate() {
            // Discipline the mesh clock once at the DMA/block boundary. Its rate
            // cannot change meaningfully within tens of samples, and calling the
            // affine clock model per sample wastes i128 math on embedded targets.
            let mesh_us = block_mesh + (ix as Micros * 1_000_000 / sr);
            let mut dry = 0.0;
            for (j, role) in ROLES.iter().enumerate() {
                if roles[j] {
                    let contribution = render_role(*role, mesh_us, ctx);
                    dry += if *role == Role::Low {
                        self.bass_lowpass.process(contribution)
                    } else {
                        contribution
                    };
                }
            }
            let colored = color(dry, mesh_us, self.sample_rate, tone);
            let wet = self.lowpass.process(colored);
            *sample = (wet * OUTPUT_AMPLITUDE).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        }
    }

    /// Retune the master lowpass to the active vibe's `cutoff_hz`. A no-op on the
    /// common path (the cutoff only moves when the seed picks a new kit), so the
    /// rare filter re-init is inaudible against the seed change that triggered it.
    fn retune_lowpass(&mut self, cutoff_hz: f32) {
        if (cutoff_hz - self.lowpass_cutoff_hz).abs() > 0.5 {
            self.lowpass = Lowpass::new(cutoff_hz, self.sample_rate, 0.707);
            self.lowpass_cutoff_hz = cutoff_hz;
        }
    }

    fn apply_due_events(&mut self, root_us: Micros) {
        let tick = self.transport.tick_at(root_us);
        while let Some(event) = self.events.pop_due(tick) {
            use lofi_core::event::EventAction;
            match event.action {
                EventAction::SetSection(section) => self.section = section,
                EventAction::SetSeed(seed) => {
                    self.seed = seed;
                    self.scene = self
                        .catalog
                        .loop_scene(seed)
                        .expect("catalog has no coherent loop scene");
                }
                EventAction::SetTempo { bpm_milli } => {
                    self.transport = self.transport.retimed(root_us, bpm_milli)
                }
            }
        }
    }

    /// Current mesh sync quality (root, stratum, peer count, dispersion).
    pub fn quality(&self, now_local_us: Micros) -> SyncQuality {
        self.engine.quality(now_local_us)
    }

    /// Snapshot for the LCD. The same struct drives the real SSD1306 panel.
    pub fn display_state(&self, now_local_us: Micros) -> DisplayState {
        let mesh_us = self.engine.mesh_from_local(now_local_us);
        let tick = self.transport.tick_at(mesh_us);
        let ticks_per_bar = (self.transport.ticks_per_beat as i64) * 4;
        let beat_phase = tick.rem_euclid(ticks_per_bar.max(1));
        let quality = self.engine.quality(now_local_us);

        let roster = self.engine.roster(now_local_us);
        let bar = tick.div_euclid(TICKS_PER_BAR);
        let phrase = bar.div_euclid(BARS_PER_PHRASE);
        let phrase_ticks = TICKS_PER_BAR * BARS_PER_PHRASE;
        let next_phrase_tick = (tick.div_euclid(phrase_ticks) + 1) * phrase_ticks;
        let change_in_millis = self
            .transport
            .root_time_for_tick(next_phrase_tick)
            .saturating_sub(mesh_us)
            .div_euclid(1_000)
            .clamp(0, u32::MAX as i64) as u32;
        let arrangement = Arrangement::at(self.seed, roster.ids(), phrase);

        DisplayState {
            node_id: self.id as u32,
            playing: self.running,
            bpm_milli: self.transport.bpm_milli,
            role: Role::primary(roster.my_index(), roster.len()),
            codename: arrangement.codename(),
            next_codename: Arrangement::next_codename(self.seed, roster.ids(), phrase),
            bars_to_next: (BARS_PER_PHRASE - bar.rem_euclid(BARS_PER_PHRASE)) as u8,
            change_in_millis,
            peers: quality.peers,
            sync_error_us: quality.dispersion_us as Micros,
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

    #[test]
    fn display_counts_down_to_the_shared_phrase_boundary() {
        let dev = Device::new(
            1,
            DeviceVoice::new(880, ArpDirection::Up),
            Transport::new(0, 80_000, 96),
            1,
        );
        assert_eq!(dev.display_state(0).change_in_millis, 24_000);
        assert_eq!(dev.display_state(3_000_000).change_in_millis, 21_000);
        assert_eq!(dev.display_state(23_500_000).change_in_millis, 500);
        assert_eq!(dev.display_state(24_000_000).change_in_millis, 24_000);
    }
}
