#![cfg_attr(all(not(test), target_arch = "wasm32"), no_std)]

//! Allocation-free WebAssembly ABI for one firmware device.
//!
//! A browser creates one module instance per virtual box. Each instance owns an
//! independent [`lofi_app::Device`], audio buffer, mesh RX/TX buffers, clock,
//! peer table, and synth state. The host may only move encoded ESP-NOW-sized
//! packets between instances and provide each device's local hardware time.

use core::cell::UnsafeCell;

use lofi_app::{ArpDirection, Device, DeviceVoice, Engine};
use lofi_core::mesh::wire::{MeshMessage, MESH_WIRE_MAX};
use lofi_core::music::arrangement::BARS_PER_PHRASE;
use lofi_core::music::Role;
use lofi_core::transport::Transport;
use lofi_core::{Micros, NodeId};

const RENDER_FRAMES: usize = 128;
const BPM_MILLI: u32 = 80_000;
const BROADCAST: u32 = 0;
const STATUS_FIELDS: usize = 15;

struct Runtime {
    device: Device,
    output: [i16; RENDER_FRAMES],
    rx: [u8; MESH_WIRE_MAX],
    tx: [u8; MESH_WIRE_MAX],
    tx_destination: u32,
    status: [i32; STATUS_FIELDS],
}

struct RuntimeCell(UnsafeCell<Option<Runtime>>);

// One AudioWorklet thread is the sole caller for each WASM instance.
unsafe impl Sync for RuntimeCell {}

static RUNTIME: RuntimeCell = RuntimeCell(UnsafeCell::new(None));
static SILENCE: [i16; RENDER_FRAMES] = [0; RENDER_FRAMES];
static EMPTY_BYTES: [u8; MESH_WIRE_MAX] = [0; MESH_WIRE_MAX];
static EMPTY_STATUS: [i32; STATUS_FIELDS] = [0; STATUS_FIELDS];

/// Initialize or reset one independent virtual device.
#[no_mangle]
pub extern "C" fn lofi_init(sample_rate: u32, seed: u32, node_id: u32) {
    lofi_init_at(sample_rate, seed, node_id, 0, 0);
}

/// Initialize a virtual device against an explicit shared song-zero timestamp.
/// Browser hosts use a future timestamp while pre-rolling the mock radio, so
/// the first audible frame begins on an exact musical boundary.
#[no_mangle]
pub extern "C" fn lofi_init_at(
    sample_rate: u32,
    seed: u32,
    node_id: u32,
    song_zero_us_low: u32,
    song_zero_us_high: i32,
) {
    lofi_init_at_position(
        sample_rate,
        seed,
        node_id,
        song_zero_us_low,
        song_zero_us_high,
        BPM_MILLI,
        0,
    );
}

/// Initialize at a specific transport position. Listening-study candidates use
/// this to compare distinct eight-bar phrases and source-native tempos without
/// adding any alternate audio path to the firmware engine.
#[no_mangle]
pub extern "C" fn lofi_init_at_position(
    sample_rate: u32,
    seed: u32,
    node_id: u32,
    anchor_us_low: u32,
    anchor_us_high: i32,
    bpm_milli: u32,
    start_phrase: u32,
) {
    let sample_rate = sample_rate.max(1);
    let node_id = node_id.max(1) as NodeId;
    let anchor_us = join_micros(anchor_us_low, anchor_us_high);
    let bpm_milli = bpm_milli.clamp(40_000, 200_000);
    let phrase_ticks = i64::from(start_phrase)
        .saturating_mul(4)
        .saturating_mul(BARS_PER_PHRASE)
        .saturating_mul(i64::from(lofi_core::transport::DEFAULT_TICKS_PER_BEAT));
    let position = Transport::new(0, bpm_milli, lofi_core::transport::DEFAULT_TICKS_PER_BEAT)
        .root_time_for_tick(phrase_ticks);
    let song_zero_us = anchor_us.saturating_sub(position);
    let transport = Transport::new(
        song_zero_us,
        bpm_milli,
        lofi_core::transport::DEFAULT_TICKS_PER_BEAT,
    );
    let voice = DeviceVoice::new(440, ArpDirection::Up);
    let device = Device::new(node_id, voice, transport, seed as u64).with_sample_rate(sample_rate);
    let runtime = Runtime {
        device,
        output: [0; RENDER_FRAMES],
        rx: [0; MESH_WIRE_MAX],
        tx: [0; MESH_WIRE_MAX],
        tx_destination: BROADCAST,
        status: [0; STATUS_FIELDS],
    };

    unsafe { *RUNTIME.0.get() = Some(runtime) };
}

/// Select the composer driving this device: 0 symbolic (default), 1 loops.
/// The loop engine remains selectable for blinded A/B listening studies.
#[no_mangle]
pub extern "C" fn lofi_set_engine(engine: u32) {
    if let Some(runtime) = runtime_mut() {
        let engine = if engine == 1 {
            Engine::Loops
        } else {
            Engine::Symbolic
        };
        runtime.device.set_engine(engine);
    }
}

/// Render one Web Audio quantum at this device's local hardware time.
#[no_mangle]
pub extern "C" fn lofi_render(local_us_low: u32, local_us_high: i32) -> *const i16 {
    let Some(runtime) = runtime_mut() else {
        return SILENCE.as_ptr();
    };
    let local_us = join_micros(local_us_low, local_us_high);
    runtime.device.render_audio(&mut runtime.output, local_us);
    runtime.output.as_ptr()
}

/// Poll for a broadcast mesh beacon. Returns the encoded byte length.
#[no_mangle]
pub extern "C" fn lofi_poll_beacon(local_us_low: u32, local_us_high: i32) -> u32 {
    let Some(runtime) = runtime_mut() else {
        return 0;
    };
    let local_us = join_micros(local_us_low, local_us_high);
    let Some(message) = runtime.device.poll_beacon(local_us) else {
        return 0;
    };
    stage_message(runtime, message, BROADCAST)
}

/// Poll for a directed timing probe. Returns the encoded byte length.
#[no_mangle]
pub extern "C" fn lofi_poll_probe(local_us_low: u32, local_us_high: i32) -> u32 {
    let Some(runtime) = runtime_mut() else {
        return 0;
    };
    let local_us = join_micros(local_us_low, local_us_high);
    let Some((destination, message)) = runtime.device.poll_probe(local_us) else {
        return 0;
    };
    stage_message(runtime, message, destination as u32)
}

/// Decode and ingest the bytes copied into [`lofi_rx_ptr`]. If handling the
/// frame creates an immediate probe response, returns its encoded length.
#[no_mangle]
pub extern "C" fn lofi_receive(length: u32, local_us_low: u32, local_us_high: i32) -> u32 {
    let Some(runtime) = runtime_mut() else {
        return 0;
    };
    let length = length as usize;
    if length > MESH_WIRE_MAX {
        return 0;
    }
    let Ok(message) = MeshMessage::decode(&runtime.rx[..length]) else {
        return 0;
    };
    let local_us = join_micros(local_us_low, local_us_high);
    let Some(reply) = runtime.device.handle(message, local_us) else {
        return 0;
    };
    let destination = message_destination(reply);
    stage_message(runtime, reply, destination)
}

/// Refresh telemetry and return fifteen signed 32-bit fields in linear memory:
/// node id, root id, peers, dispersion us, role, synced, mesh offset us, bar
/// phase 0..1000, root flag, thousandths of a beat until the next phrase, and
/// the stable feature code for the variation arriving at that boundary, plus
/// the compact role mask, spotlight role, current phrase number, and selector id.
#[no_mangle]
pub extern "C" fn lofi_status(local_us_low: u32, local_us_high: i32) -> *const i32 {
    let Some(runtime) = runtime_mut() else {
        return EMPTY_STATUS.as_ptr();
    };
    let local_us = join_micros(local_us_low, local_us_high);
    let quality = runtime.device.quality(local_us);
    let display = runtime.device.display_state(local_us);
    let mesh_offset = runtime
        .device
        .mesh_from_local(local_us)
        .saturating_sub(local_us);
    runtime.status = [
        runtime.device.id() as i32,
        quality.root_id as i32,
        quality.peers as i32,
        quality.dispersion_us,
        role_index(display.role),
        quality.synced as i32,
        mesh_offset.clamp(i32::MIN as Micros, i32::MAX as Micros) as i32,
        display.beat_phase_milli as i32,
        quality.is_root as i32,
        display.beats_to_next_milli.min(i32::MAX as u32) as i32,
        display.next_feature.code() as i32,
        display.role_mask as i32,
        role_index(display.spotlight),
        display.phrase.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
        display.selector.min(i32::MAX as u64) as i32,
    ];
    runtime.status.as_ptr()
}

#[no_mangle]
pub extern "C" fn lofi_render_frames() -> u32 {
    RENDER_FRAMES as u32
}

#[no_mangle]
pub extern "C" fn lofi_wire_capacity() -> u32 {
    MESH_WIRE_MAX as u32
}

#[no_mangle]
pub extern "C" fn lofi_status_fields() -> u32 {
    STATUS_FIELDS as u32
}

#[no_mangle]
pub extern "C" fn lofi_rx_ptr() -> *mut u8 {
    runtime_mut()
        .map(|runtime| runtime.rx.as_mut_ptr())
        .unwrap_or(EMPTY_BYTES.as_ptr() as *mut u8)
}

#[no_mangle]
pub extern "C" fn lofi_tx_ptr() -> *const u8 {
    runtime_mut()
        .map(|runtime| runtime.tx.as_ptr())
        .unwrap_or(EMPTY_BYTES.as_ptr())
}

#[no_mangle]
pub extern "C" fn lofi_tx_destination() -> u32 {
    runtime_mut()
        .map(|runtime| runtime.tx_destination)
        .unwrap_or(BROADCAST)
}

fn runtime_mut() -> Option<&'static mut Runtime> {
    unsafe { (&mut *RUNTIME.0.get()).as_mut() }
}

fn stage_message(runtime: &mut Runtime, message: MeshMessage, destination: u32) -> u32 {
    let (bytes, length) = message.encode();
    runtime.tx[..length].copy_from_slice(&bytes[..length]);
    runtime.tx_destination = destination;
    length as u32
}

fn message_destination(message: MeshMessage) -> u32 {
    match message {
        MeshMessage::Beacon(_) => BROADCAST,
        MeshMessage::ProbeRequest(probe) => probe.target as u32,
        MeshMessage::ProbeResponse(probe) => probe.target as u32,
    }
}

fn join_micros(low: u32, high: i32) -> Micros {
    ((high as i64) << 32) | low as i64
}

fn role_index(role: Role) -> i32 {
    match role {
        Role::Pulse => 0,
        Role::Pocket => 1,
        Role::Low => 2,
        Role::Color => 3,
        Role::Motif => 4,
    }
}

#[cfg(all(not(test), target_arch = "wasm32"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_audible_bounded_pcm() {
        lofi_init(48_000, 2, 7);
        let mut energy = 0u64;
        for block in 0..400 {
            let micros = block * RENDER_FRAMES as i64 * 1_000_000 / 48_000;
            let (low, high) = split_micros(micros);
            let ptr = lofi_render(low, high);
            let block = unsafe { core::slice::from_raw_parts(ptr, RENDER_FRAMES) };
            for &sample in block {
                energy += sample.unsigned_abs() as u64;
            }
        }
        assert!(energy > 1_000);
    }

    #[test]
    fn emits_real_mesh_wire_frames() {
        lofi_init(48_000, 2, 7);
        let length = lofi_poll_beacon(0, 0) as usize;
        assert!(length > 0 && length <= MESH_WIRE_MAX);
        let bytes = unsafe { core::slice::from_raw_parts(lofi_tx_ptr(), length) };
        let message = MeshMessage::decode(bytes).unwrap();
        assert!(matches!(message, MeshMessage::Beacon(beacon) if beacon.sender == 7));
    }

    #[test]
    fn status_exposes_the_complete_role_mask() {
        lofi_init(48_000, 2, 7);
        assert_eq!(lofi_status_fields(), 15);
        let ptr = lofi_status(0, 0);
        let status = unsafe { core::slice::from_raw_parts(ptr, STATUS_FIELDS) };
        assert_eq!(status[11], 0b1_1111);
    }

    #[test]
    fn timestamped_init_starts_on_the_requested_boundary() {
        let song_zero = 2_000_000_i64;
        let (low, high) = split_micros(song_zero);
        lofi_init_at(48_000, 2, 7, low, high);
        let ptr = lofi_status(low, high);
        let status = unsafe { core::slice::from_raw_parts(ptr, STATUS_FIELDS) };
        assert_eq!(status[7], 0);
        assert_eq!(status[9], 32_000);
    }

    #[test]
    fn positioned_init_starts_on_the_requested_phrase_and_tempo() {
        let anchor = 2_000_000_i64;
        let (low, high) = split_micros(anchor);
        lofi_init_at_position(48_000, 2, 7, low, high, 72_000, 7);
        let runtime = runtime_mut().unwrap();
        assert_eq!(runtime.device.transport().bpm_milli, 72_000);
        assert_eq!(
            runtime.device.transport().tick_at(anchor),
            7 * 8 * 4 * i64::from(lofi_core::transport::DEFAULT_TICKS_PER_BEAT)
        );
        let ptr = lofi_status(low, high);
        let status = unsafe { core::slice::from_raw_parts(ptr, STATUS_FIELDS) };
        assert_eq!(status[7], 0);
        assert_eq!(status[9], 32_000);
    }

    fn split_micros(micros: i64) -> (u32, i32) {
        (micros as u32, (micros >> 32) as i32)
    }
}
