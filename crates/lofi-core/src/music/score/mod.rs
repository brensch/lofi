//! The symbolic music engine.
//!
//! Every note and hit exists as a symbol — lane, step, pitch, velocity,
//! micro-delay — before it touches audio. Samples supply timbre only; the
//! music is data that host tooling can dump, property-test, and diff without
//! listening. See `docs/SYMBOLIC_MUSIC.md`.
//!
//! Layers, all deterministic from `(seed, roster, mesh tick)`:
//!
//! - **`session`** — seed → key, progression, groove signature, source casting.
//! - **`scene`** — session → concrete one-shot bindings per pitch (init-only).
//! - **`drums`** — kick/snare/hat events on the measured signature grids.
//! - **`tonal`** — bass lines, shell-voiced keys comping, and lead motifs.
//! - **`render`** — the `no_std` allocation-free audio path over the binds.

pub mod drums;
pub mod render;
pub mod scene;
pub mod session;
pub mod tonal;

pub use render::{render_role, ScoreCtx};
pub use scene::SymbolicScene;
pub use session::Session;

/// Sixteenth-note steps per bar; the composer's rhythmic resolution.
pub const STEPS_PER_BAR: i64 = 16;
/// Steps in one four-bar pattern cycle (matches `content::StepPattern`).
pub const STEPS_PER_CYCLE: i64 = 64;
/// Transport ticks per sixteenth step (96 ticks per beat).
pub const TICKS_PER_STEP: i64 = 24;

/// One symbolic note/hit, common to every lane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    /// Index into the lane's scene bind table (which sampled voice plays).
    pub bind: u8,
    /// Concrete MIDI pitch, or the drum voice code for unpitched lanes.
    pub midi: u8,
    /// Linear gain 0..=1 derived from the symbolic velocity.
    pub level: f32,
    /// Micro-timing offset from the quantized step, in microseconds.
    /// Swing, the lane's measured pocket, and humanization all land here.
    pub delay_us: i64,
}

/// Deterministic per-event hash used for humanization and seeded choices.
pub(crate) fn event_hash(seed: u64, lane: u64, step: i64) -> u64 {
    let mut x = seed
        ^ lane.wrapping_mul(0xc2b2_ae3d_27d4_eb4f)
        ^ (step as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Bounded, deterministic humanization: at most `±bound_us` around the grid.
pub(crate) fn humanize_us(seed: u64, lane: u64, step: i64, bound_us: i64) -> i64 {
    if bound_us == 0 {
        return 0;
    }
    let h = event_hash(seed, lane, step);
    (h % (2 * bound_us + 1) as u64) as i64 - bound_us
}
