use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use lofi_sim::wav::StereoSample;
use lofi_sim::{NodeSnapshot, Simulation, SAMPLE_RATE};

/// Control messages from the UI to the simulation producer thread.
#[derive(Clone, Copy, Debug)]
pub enum Command {
    TogglePlay(usize),
    SetAllPlay(bool),
    AddNode,
    RemoveNode(usize),
    SetSync(bool),
    SetPan(usize, f32),
    SetGain(usize, f32),
    SetMute(usize, bool),
    SetSolo(usize, bool),
    SetDrift(usize, i32),
    SetOffset(usize, i64),
}

/// A consistent view of the sim for one UI frame.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub nodes: Vec<NodeSnapshot>,
    pub global_us: i64,
    pub spread_us: i64,
    pub sync_enabled: bool,
}

/// Stereo frames kept buffered ahead of playback (~200ms).
const RING_TARGET_FRAMES: usize = SAMPLE_RATE as usize / 5;
const PRODUCER_BLOCK: usize = 512;

/// Owns the audio output stream and the producer thread. Dropping it stops both.
pub struct Engine {
    tx: Sender<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    running: Arc<AtomicBool>,
    producer: Option<JoinHandle<()>>,
    #[cfg(feature = "audio")]
    _stream: Option<cpal::Stream>,
}

impl Engine {
    pub fn new(mut sim: Simulation) -> Self {
        sim.set_sync_enabled(true);
        let ring: Arc<Mutex<VecDeque<f32>>> =
            Arc::new(Mutex::new(VecDeque::with_capacity(RING_TARGET_FRAMES * 4)));
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let running = Arc::new(AtomicBool::new(true));
        let (tx, rx) = mpsc::channel();

        #[cfg(feature = "audio")]
        let stream = audio::build_stream(ring.clone());
        // cpal drains the ring via its callback. If it found no device (e.g. WSL
        // has no ALSA card), the producer falls back to PulseAudio or silence.
        #[cfg(feature = "audio")]
        let cpal_active = stream.is_some();
        #[cfg(not(feature = "audio"))]
        let cpal_active = false;
        let producer = spawn_producer(
            sim,
            rx,
            ring,
            snapshot.clone(),
            running.clone(),
            cpal_active,
        );

        Self {
            tx,
            snapshot,
            running,
            producer: Some(producer),
            #[cfg(feature = "audio")]
            _stream: stream,
        }
    }

    pub fn send(&self, command: Command) {
        let _ = self.tx.send(command);
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.lock().unwrap().clone()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.producer.take() {
            let _ = handle.join();
        }
    }
}

/// Which audio path the producer is driving.
enum Sink {
    #[cfg(feature = "audio")]
    Cpal,
    Pulse(crate::pulse::PulseSink),
    None,
}

impl Sink {
    fn select(cpal_active: bool) -> Self {
        if cpal_active {
            eprintln!("lofi-ui: audio via system default (cpal)");
            #[cfg(feature = "audio")]
            return Sink::Cpal;
        }
        if let Some(pulse) = crate::pulse::PulseSink::try_open(SAMPLE_RATE) {
            eprintln!("lofi-ui: audio via PulseAudio");
            return Sink::Pulse(pulse);
        }
        eprintln!("lofi-ui: no audio output device; running silently");
        Sink::None
    }
}

fn spawn_producer(
    mut sim: Simulation,
    rx: Receiver<Command>,
    ring: Arc<Mutex<VecDeque<f32>>>,
    snapshot: Arc<Mutex<Snapshot>>,
    running: Arc<AtomicBool>,
    cpal_active: bool,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let _ = &ring; // only read by the cpal path
        let mut sink = Sink::select(cpal_active);
        let mut block = [StereoSample { left: 0, right: 0 }; PRODUCER_BLOCK];
        let mut interleaved = [0i16; PRODUCER_BLOCK * 2];
        while running.load(Ordering::Relaxed) {
            while let Ok(command) = rx.try_recv() {
                apply_command(&mut sim, command);
            }
            let mut paced_by_sink = false;
            match &mut sink {
                #[cfg(feature = "audio")]
                Sink::Cpal => {
                    push_to_ring(&mut sim, &mut block, &ring);
                    paced_by_sink = true; // ring backpressure paces us
                }
                Sink::Pulse(pulse) => {
                    sim.fill(&mut block);
                    interleave(&block, &mut interleaved);
                    pulse.write(&interleaved); // blocking write paces us
                    paced_by_sink = true;
                }
                Sink::None => {
                    // No sink: advance ~one tick of audio so the UI still moves.
                    let blocks = (SAMPLE_RATE as usize * 3 / 1000) / PRODUCER_BLOCK + 1;
                    for _ in 0..blocks {
                        sim.fill(&mut block);
                    }
                }
            }
            publish(&mut sim, &snapshot);
            if !paced_by_sink {
                thread::sleep(Duration::from_millis(3));
            }
        }
    })
}

fn interleave(block: &[StereoSample], out: &mut [i16]) {
    for (frame, sample) in block.iter().enumerate() {
        out[frame * 2] = sample.left;
        out[frame * 2 + 1] = sample.right;
    }
}

#[cfg(feature = "audio")]
fn push_to_ring(
    sim: &mut Simulation,
    block: &mut [StereoSample],
    ring: &Arc<Mutex<VecDeque<f32>>>,
) {
    loop {
        if ring.lock().unwrap().len() >= RING_TARGET_FRAMES * 2 {
            break;
        }
        sim.fill(block);
        let mut guard = ring.lock().unwrap();
        for sample in block.iter() {
            guard.push_back(sample.left as f32 / 32768.0);
            guard.push_back(sample.right as f32 / 32768.0);
        }
    }
}

fn publish(sim: &mut Simulation, snapshot: &Arc<Mutex<Snapshot>>) {
    let stats = sim.phase_stats();
    *snapshot.lock().unwrap() = Snapshot {
        nodes: sim.snapshot(),
        global_us: sim.global_us(),
        spread_us: stats.max_spread_us,
        sync_enabled: sim.sync_enabled(),
    };
}

fn apply_command(sim: &mut Simulation, command: Command) {
    match command {
        Command::TogglePlay(ix) => sim.toggle_running(ix),
        Command::SetAllPlay(on) => sim.set_all_running(on),
        Command::AddNode => sim.add_node(),
        Command::RemoveNode(ix) => sim.remove_node(ix),
        Command::SetSync(on) => sim.set_sync_enabled(on),
        Command::SetPan(ix, v) => sim.with_node_mix(ix, |m| m.pan = v),
        Command::SetGain(ix, v) => sim.with_node_mix(ix, |m| m.gain = v),
        Command::SetMute(ix, v) => sim.with_node_mix(ix, |m| m.mute = v),
        Command::SetSolo(ix, v) => sim.with_node_mix(ix, |m| m.solo = v),
        Command::SetDrift(ix, v) => sim.set_drift_ppb(ix, v),
        Command::SetOffset(ix, v) => sim.set_offset_us(ix, v),
    }
}

#[cfg(feature = "audio")]
mod audio {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use lofi_sim::SAMPLE_RATE;

    pub fn build_stream(ring: Arc<Mutex<VecDeque<f32>>>) -> Option<cpal::Stream> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };
        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut guard = ring.lock().unwrap();
                    for sample in data.iter_mut() {
                        *sample = guard.pop_front().unwrap_or(0.0);
                    }
                },
                |err| eprintln!("audio stream error: {err}"),
                None,
            )
            .ok()?;
        stream.play().ok()?;
        Some(stream)
    }
}
