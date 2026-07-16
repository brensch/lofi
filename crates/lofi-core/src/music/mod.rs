//! Deterministic, sample-only lo-fi composition and playback.
//!
//! The sound library is data and can grow without changing the realtime engine:
//!
//! - **`theory` / `progression`** — the symbolic layer: keys, chords, voicing.
//! - **`catalog`** — allocation-free metadata and audio borrowed from a fixed pack.
//! - **`content`** — harvested groove signatures and sparse motif schedules.
//! - **`sample`** — stateless mu-law playback and bounded repitching.
//! - **`kit`** — deterministic master tone profiles selected from the shared seed.
//! - **`character`** — tape wow/flutter and the vinyl air bed, the shared
//!   imperfection that makes it a record.
//! - **`beat`** — schedules catalogue elements for each distributed role.
//! - **`fx`** — the one stateful piece, the per-device master lowpass.
//!
//! Everything except that lowpass is deterministic from `(seed, roster, mesh
//! tick)`, so boxes agree on the arrangement while rendering their assigned roles.

pub mod arrangement;
pub mod beat;
pub mod catalog;
pub mod character;
pub mod content;
mod dsp;
pub mod fx;
pub mod kit;
pub mod progression;
pub mod sample;
pub mod theory;

mod tables;

pub use arrangement::{Arrangement, Codename, Role};
pub use beat::{color, render_role, BeatCtx};
pub use catalog::{ElementKind, PackedCatalog, PackedElement, AI_CATALOG};
pub use content::{signature_for, GrooveSignature, Motif, NoteEvent};
pub use fx::Lowpass;
pub use kit::{Kit, Tone};
pub use sample::{render_sample, render_sample_looped, render_sample_pitched, Sample};
