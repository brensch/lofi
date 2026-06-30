//! Minimal PulseAudio playback via the "simple" API, loaded at runtime.
//!
//! `cpal` on Linux only speaks ALSA, which has no route to sound under WSLg
//! (no card; audio lives on a PulseAudio server). Rather than require the
//! 40-package ALSA→Pulse bridge, we `dlopen` the `libpulse-simple` that WSLg
//! already ships and push interleaved S16LE frames straight to the server.
//! `pa_simple_write` blocks until the server accepts the data, which paces
//! playback at realtime for us.

use std::ffi::c_void;
use std::os::raw::c_char;

use libloading::{Library, Symbol};

#[repr(C)]
struct PaSampleSpec {
    format: i32,
    rate: u32,
    channels: u8,
}

// PA_SAMPLE_S16LE
const SAMPLE_FORMAT_S16LE: i32 = 3;
// PA_STREAM_PLAYBACK
const STREAM_PLAYBACK: i32 = 1;

type PaSimpleNew = unsafe extern "C" fn(
    *const c_char,       // server (null = default, honours PULSE_SERVER)
    *const c_char,       // app name
    i32,                 // direction
    *const c_char,       // device (null = default sink)
    *const c_char,       // stream description
    *const PaSampleSpec, // sample spec
    *const c_void,       // channel map (null)
    *const c_void,       // buffer attr (null)
    *mut i32,            // error out
) -> *mut c_void;
type PaSimpleWrite = unsafe extern "C" fn(*mut c_void, *const c_void, usize, *mut i32) -> i32;
type PaSimpleFree = unsafe extern "C" fn(*mut c_void);

/// An open PulseAudio playback stream. 48 kHz stereo S16LE.
pub struct PulseSink {
    handle: *mut c_void,
    write: PaSimpleWrite,
    free: PaSimpleFree,
}

impl PulseSink {
    pub fn try_open(rate: u32) -> Option<Self> {
        unsafe {
            let lib = Library::new("libpulse-simple.so.0").ok()?;
            let new: Symbol<PaSimpleNew> = lib.get(b"pa_simple_new\0").ok()?;
            let write: Symbol<PaSimpleWrite> = lib.get(b"pa_simple_write\0").ok()?;
            let free: Symbol<PaSimpleFree> = lib.get(b"pa_simple_free\0").ok()?;
            let (new, write, free) = (*new, *write, *free);

            let spec = PaSampleSpec {
                format: SAMPLE_FORMAT_S16LE,
                rate,
                channels: 2,
            };
            let mut err = 0i32;
            let handle = new(
                std::ptr::null(),
                b"lofi-ui\0".as_ptr() as *const c_char,
                STREAM_PLAYBACK,
                std::ptr::null(),
                b"groove\0".as_ptr() as *const c_char,
                &spec,
                std::ptr::null(),
                std::ptr::null(),
                &mut err,
            );
            if handle.is_null() {
                return None;
            }
            // Keep the library mapped for the life of the process; the stored
            // function pointers depend on it.
            std::mem::forget(lib);
            Some(PulseSink {
                handle,
                write,
                free,
            })
        }
    }

    /// Write interleaved L/R S16LE frames. Blocks until the server accepts them.
    pub fn write(&mut self, interleaved: &[i16]) {
        let mut err = 0i32;
        let bytes = std::mem::size_of_val(interleaved);
        unsafe {
            (self.write)(
                self.handle,
                interleaved.as_ptr() as *const c_void,
                bytes,
                &mut err,
            );
        }
    }
}

impl Drop for PulseSink {
    fn drop(&mut self) {
        unsafe { (self.free)(self.handle) };
    }
}
