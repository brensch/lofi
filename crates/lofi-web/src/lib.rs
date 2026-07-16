#![cfg_attr(all(not(test), target_arch = "wasm32"), no_std)]

//! Allocation-free WebAssembly ABI for one firmware device.
//!
//! A browser creates one module instance per virtual box. Each instance owns an
//! independent [`lofi_app::Device`], audio buffer, mesh RX/TX buffers, clock,
//! peer table, and synth state. The host may only move encoded ESP-NOW-sized
//! packets between instances and provide each device's local hardware time.

use core::cell::UnsafeCell;

use lofi_app::{ArpDirection, Device, DeviceVoice};
use lofi_core::mesh::wire::{MeshMessage, MESH_WIRE_MAX};
use lofi_core::music::Role;
use lofi_core::transport::Transport;
use lofi_core::{Micros, NodeId};

const RENDER_FRAMES: usize = 128;
const BPM_MILLI: u32 = 80_000;
const BROADCAST: u32 = 0;
const STATUS_FIELDS: usize = 10;

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
    let sample_rate = sample_rate.max(1);
    let node_id = node_id.max(1) as NodeId;
    let transport = Transport::new(0, BPM_MILLI, lofi_core::transport::DEFAULT_TICKS_PER_BEAT);
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

/// Refresh telemetry and return ten signed 32-bit fields in linear memory:
/// node id, root id, peers, dispersion us, role, synced, mesh offset us, bar
/// phase 0..1000, root flag, and milliseconds until the next phrase.
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
        display.change_in_millis.min(i32::MAX as u32) as i32,
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

    fn split_micros(micros: i64) -> (u32, i32) {
        (micros as u32, (micros >> 32) as i32)
    }
}
