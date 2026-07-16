//! Tape + vinyl "character": the shared, deterministic imperfection that turns a
//! clean synth mix into a lofi record.
//!
//! Everything here is a pure function of *mesh time* (never per-node state), so
//! the wobble and crackle are identical on every box — the mesh drifts as one
//! tape, not a room full of slightly different ones. The one stateful piece, the
//! master lowpass, lives in `fx` because an IIR needs history; its cutoff comes
//! from the kit's [`Tone`].

use super::kit::Tone;
use super::patch::{fast_decay, noise, sin_turns};
use crate::Micros;

/// Tape pitch instability as a frequency multiplier around 1.0. A slow "wow"
/// plus faster "flutter", each built from incommensurate partials so it never
/// resolves into a clean vibrato. Multiply a voice's frequency by this.
pub fn warble(mesh_us: Micros, tone: Tone) -> f32 {
    let t = mesh_us as f32 / 1_000_000.0;
    let wow = sin_turns(0.60 * t) * 0.6 + sin_turns(0.27 * t) * 0.4;
    let flutter = sin_turns(6.3 * t) * 0.6 + sin_turns(9.1 * t) * 0.4;
    1.0 + tone.wow * wow + tone.flutter * flutter
}

/// The vinyl air bed: steady hiss plus sparse crackle pops, scaled by `air`.
/// `nz` is a per-sample noise index (a deterministic function of mesh time).
pub fn vinyl(nz: u32, sample_rate: u32, air: f32) -> f32 {
    if air <= 0.0 {
        return 0.0;
    }
    let hiss = noise(nz ^ 0x9e37_79b9) * 0.0015;

    // Evaluate one pop candidate every half-second, then give accepted events a
    // short decay. The old per-sample gate produced hundreds of clicks a second.
    let interval = (sample_rate / 2).max(1);
    let bucket = nz / interval;
    let age_samples = nz % interval;
    let gate = noise(bucket ^ 0xa53c_91e7);
    let age = age_samples as f32 / sample_rate.max(1) as f32;
    let pop = if gate > 0.7 && age < 0.012 {
        let shape = fast_decay(age, 0.0035);
        noise(bucket.wrapping_mul(2_654_435_761)) * 0.035 * shape
    } else {
        0.0
    };
    (hiss + pop) * air
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::kit::KIT_DUSTY;

    #[test]
    fn warble_is_small_and_centered() {
        let tone = KIT_DUSTY.tone;
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for i in 0..200_000 {
            let w = warble(i as Micros * 500, tone);
            lo = lo.min(w);
            hi = hi.max(w);
        }
        // Pitch never drifts more than ~1% — musical, not seasick.
        assert!(lo > 0.99 && hi < 1.01, "warble range {lo}..{hi}");
    }

    #[test]
    fn vinyl_bed_is_bounded_and_optional() {
        for i in 0..50_000u32 {
            assert!(vinyl(i, 48_000, 1.0).abs() < 0.05);
        }
        assert_eq!(vinyl(123, 48_000, 0.0), 0.0);
    }

    #[test]
    fn vinyl_pops_are_sparse() {
        let loud = (0..480_000u32)
            .filter(|&i| vinyl(i, 48_000, 1.0).abs() > 0.01)
            .count();
        assert!(loud < 1_000, "vinyl produced {loud} loud samples in 10s");
    }
}
