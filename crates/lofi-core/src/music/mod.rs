//! Procedural lofi composition + synthesis.
//!
//! Layered so the sound library is *data*, not code, and grows without touching
//! the engine:
//!
//! - **`theory` / `progression`** — the symbolic layer: keys, chords, voicing.
//! - **`patch`** — a compact modular synth. Every instrument is a `Patch` value
//!   rendered by one pure function; add a timbre by adding a `const`.
//! - **`kit`** — the catalogue: the `Patch` presets plus `Kit`s that bundle one
//!   instrument per role into a coherent *vibe*, selected from the shared seed.
//! - **`character`** — tape wow/flutter and the vinyl air bed, the shared
//!   imperfection that makes it a record.
//! - **`beat`** — turns the arrangement into per-role samples through the kit.
//! - **`fx`** — the one stateful piece, the per-device master lowpass.
//!
//! Everything except that lowpass is deterministic from `(seed, roster, mesh
//! tick)`, so every box in the mesh renders the identical dry signal.

pub mod arrangement;
pub mod beat;
pub mod character;
pub mod fx;
pub mod kit;
pub mod patch;
pub mod progression;
pub mod sample;
pub mod sample_bank;
pub mod theory;

mod tables;

pub use arrangement::{Arrangement, Codename, Role};
pub use beat::{color, render_role, BeatCtx};
pub use fx::Lowpass;
pub use kit::{Kit, Tone};
pub use patch::{render_patch, Patch};
pub use sample::{render_sample, Sample};
