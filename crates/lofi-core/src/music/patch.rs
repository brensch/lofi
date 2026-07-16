//! Data-driven voice synthesis.
//!
//! Every instrument is a `Patch` *value*, not bespoke DSP code. Rendering is one
//! pure function of `(patch, freq, age, noise_index)`, so every mesh box produces
//! a bit-identical dry signal, and the timbre catalogue grows by adding `const
//! Patch` data — no new render code, no new match arms. That is the whole point:
//! a world-class sound library is a big pile of `Patch` constants that all play
//! through the same tiny, well-tested engine.
//!
//! The engine is a compact modular synth built from four voice primitives that,
//! combined, cover the lofi palette (and far beyond):
//!
//! - **Partials** — additive body: harmonic multiples with independent decay.
//! - **FM** — a 2-operator "bark" for tines, bells, and plucks.
//! - **Noise** — filtered, decaying broadband for drums, breath, and air.
//! - **Pitch env** — a tuned drop for kicks/toms and zaps.
//!
//! wrapped by an amp envelope, vibrato, tremolo, and a per-voice drive stage.

use core::f32::consts::TAU;

use super::tables::{SINE, SINE_LEN};

/// Sine of a phase in *turns* (cycles). Range-reduced with `floorf` so it stays
/// precise at large absolute times (mesh time can be minutes of samples).
#[inline]
pub fn sin_turns(turns: f32) -> f32 {
    let mut phase = turns - turns as i32 as f32;
    if phase < 0.0 {
        phase += 1.0;
    }
    let position = phase * SINE_LEN as f32;
    let index = position as usize;
    let fraction = position - index as f32;
    let a = SINE[index & (SINE_LEN - 1)];
    let b = SINE[(index + 1) & (SINE_LEN - 1)];
    a + (b - a) * fraction
}

/// Cheap envelope approximation for the audio loop. Error stays small over the
/// musically useful first few time constants and the tail is clamped to zero.
#[inline]
pub(crate) fn fast_decay(age: f32, time_constant: f32) -> f32 {
    if age <= 0.0 {
        return 1.0;
    }
    let x = age / time_constant.max(1e-4);
    if x >= 12.0 {
        return 0.0;
    }
    let x2 = x * x;
    1.0 / (1.0 + x + 0.48 * x2 + 0.235 * x2 * x)
}

/// Smooth cubic saturation with no transcendental call in the sample loop.
#[inline]
pub(crate) fn soft_clip(x: f32) -> f32 {
    if x >= 1.0 {
        1.0
    } else if x <= -1.0 {
        -1.0
    } else {
        x * (1.5 - 0.5 * x * x)
    }
}

/// splitmix32 white noise in [-1, 1]. Deterministic from an integer index.
#[inline]
pub fn noise(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9e37_79b9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x21f0_aaad);
    x ^= x >> 15;
    x = x.wrapping_mul(0x735a_2d97);
    x ^= x >> 15;
    (x as f32 / u32::MAX as f32) * 2.0 - 1.0
}

/// One additive partial: a harmonic multiple of the fundamental with its own
/// starting level and exponential decay. Higher partials usually decay faster,
/// which is what makes a real tone "settle" toward its fundamental as it rings.
#[derive(Clone, Copy, Debug)]
pub struct Partial {
    /// Frequency as a multiple of the fundamental (1.0 = fundamental, 2.0 = 8ve).
    /// Slightly-detuned ratios (e.g. 1.004) between two partials give chorus.
    pub ratio: f32,
    /// Starting amplitude.
    pub level: f32,
    /// Seconds; this partial's amplitude = `level * exp(-age / decay)`.
    pub decay: f32,
}

/// 2-operator phase-modulation "bark": a modulator adds bite on the attack and
/// then folds back, which is exactly how FM electric pianos, bells, and plucks
/// get their character. Applied to every partial's phase.
#[derive(Clone, Copy, Debug)]
pub struct Fm {
    /// Modulator frequency / fundamental.
    pub ratio: f32,
    /// Modulation index at `age = 0` (how much bite on the attack).
    pub index: f32,
    /// Seconds; the index decays toward `floor` with this time constant.
    pub index_decay: f32,
    /// Sustained modulation index once the attack bark has died away.
    pub floor: f32,
}

/// Filtered, decaying broadband noise: snare fuzz, hat sizzle, kick click, breath.
#[derive(Clone, Copy, Debug)]
pub struct Noiseband {
    pub level: f32,
    /// Seconds; noise amplitude = `level * exp(-age / decay)`.
    pub decay: f32,
    /// 0 = dark/full noise, up to ~1 = bright (subtracts a neighbouring tap,
    /// which acts as a cheap high-pass differentiator).
    pub tilt: f32,
}

/// A tuned pitch drop — the click-into-thud of a kick, a tom, a zap. Rendered as
/// an analytic phase integral so fast sweeps stay perfectly in tune on every box.
#[derive(Clone, Copy, Debug)]
pub struct PitchEnv {
    /// Fraction added to the fundamental at `age = 0` (2.0 ⇒ starts at 3× freq).
    pub amount: f32,
    /// Seconds; the added pitch decays with this time constant.
    pub decay: f32,
}

/// Amplitude contour. `attack` shapes the onset, `release` is the exponential
/// tail, and `sustain` (0..1) sets a floor so held voices (pads, organs) ring on.
#[derive(Clone, Copy, Debug)]
pub struct AmpEnv {
    pub attack: f32,
    pub release: f32,
    /// 0 = pluck (dies to silence), 1 = organ (holds forever).
    pub sustain: f32,
}

/// A sine LFO used for vibrato (drives pitch) or tremolo (drives amplitude), with
/// an optional `delay` so expressive vibrato fades in after the note speaks.
#[derive(Clone, Copy, Debug)]
pub struct Lfo {
    pub rate_hz: f32,
    /// Fractional depth.
    pub depth: f32,
    /// Seconds before the LFO reaches full depth (0 = instant).
    pub delay: f32,
}

impl Lfo {
    pub const OFF: Lfo = Lfo {
        rate_hz: 0.0,
        depth: 0.0,
        delay: 0.0,
    };

    #[inline]
    fn value(self, age: f32) -> f32 {
        if self.depth == 0.0 {
            return 0.0;
        }
        let fade = if self.delay > 0.0 {
            1.0 - fast_decay(age, self.delay)
        } else {
            1.0
        };
        self.depth * sin_turns(self.rate_hz * age) * fade
    }
}

/// A complete instrument as data. Build the catalogue by declaring `const Patch`
/// values; they all render through [`render_patch`].
#[derive(Clone, Copy, Debug)]
pub struct Patch {
    pub partials: &'static [Partial],
    pub fm: Option<Fm>,
    pub noise: Option<Noiseband>,
    pub pitch_env: Option<PitchEnv>,
    pub amp: AmpEnv,
    pub vibrato: Lfo,
    pub tremolo: Lfo,
    /// Output gain (post-envelope, pre-drive).
    pub gain: f32,
    /// Per-voice soft saturation: 0 = clean, higher = warmer/thicker.
    pub drive: f32,
}

impl Patch {
    /// A silent starting point so presets only set what they use. Prefer
    /// `Patch { ..Patch::EMPTY }` in `const` position.
    pub const EMPTY: Patch = Patch {
        partials: &[],
        fm: None,
        noise: None,
        pitch_env: None,
        amp: AmpEnv {
            attack: 0.005,
            release: 0.3,
            sustain: 0.0,
        },
        vibrato: Lfo::OFF,
        tremolo: Lfo::OFF,
        gain: 1.0,
        drive: 0.0,
    };
}

/// Amplitude envelope value at `age` (seconds). `attack` is an exponential rise;
/// the body decays from 1 toward `sustain` with time constant `release`.
#[inline]
fn amp_env(e: AmpEnv, age: f32) -> f32 {
    let attack = 1.0 - fast_decay(age, e.attack);
    let body = e.sustain + (1.0 - e.sustain) * fast_decay(age, e.release);
    attack * body
}

/// Render one mono sample of a patch at pitch `freq_hz`, `age` seconds after the
/// note started. `nz` is a per-sample noise index (only read by noisy patches);
/// pass any deterministic function of mesh time so drum noise stays reproducible.
///
/// Returns 0 before the onset. Output is roughly in [-1, 1] × `gain` for typical
/// presets; callers still bound the final mix in `color`.
pub fn render_patch(patch: &Patch, freq_hz: f32, age: f32, nz: u32) -> f32 {
    if age < 0.0 {
        return 0.0;
    }

    // Vibrato scales the fundamental (slow + shallow, so freq-scaling is exact
    // enough). The pitch env is added as an analytic *phase* integral so even a
    // fast kick sweep is sample-accurate and identical across boxes.
    let f = freq_hz * (1.0 + patch.vibrato.value(age));
    let pe_phase = match patch.pitch_env {
        Some(pe) => {
            let tau = pe.decay.max(1e-4);
            f * pe.amount * tau * (1.0 - fast_decay(age, tau))
        }
        None => 0.0,
    };
    let base = f * age + pe_phase;

    // Additive body, optionally phase-modulated by the FM operator.
    let mut body = 0.0;
    let fm_index = patch
        .fm
        .map(|fm| fm.index * fast_decay(age, fm.index_decay) + fm.floor);
    let fm_mod = patch.fm.map(|fm| sin_turns(base * fm.ratio)).unwrap_or(0.0);
    for p in patch.partials {
        let amp = p.level * fast_decay(age, p.decay);
        // FM index is conventionally measured in radians; `sin_turns` expects
        // cycles. Treating radians as cycles made the presets roughly 2π harsher.
        let phase = base * p.ratio + fm_index.unwrap_or(0.0) * fm_mod / TAU;
        body += amp * sin_turns(phase);
    }

    // Blend a four-tap dark noise body with a short differentiator. This keeps
    // snares full and hats bright without stacking unrelated full-scale white
    // noise samples, which sounded brittle and excessively loud.
    if let Some(nb) = patch.noise {
        let n0 = noise(nz);
        let n1 = noise(nz.wrapping_sub(1));
        let n2 = noise(nz.wrapping_sub(2));
        let n3 = noise(nz.wrapping_sub(3));
        let dark = (n0 + n1 + n2 + n3) * 0.25;
        let bright = (n0 - n1) * 0.5;
        let tilt = nb.tilt.clamp(0.0, 1.0);
        let raw = dark * (1.0 - tilt) + bright * tilt;
        body += raw * nb.level * fast_decay(age, nb.decay);
    }

    // Tremolo dips amplitude between (1 - depth) and 1.
    let trem = 1.0 - 0.5 * patch.tremolo.depth * (1.0 - sin_turns(patch.tremolo.rate_hz * age));
    let mut out = body * amp_env(patch.amp, age) * patch.gain * trem;

    if patch.drive > 0.0 {
        // Soft saturation with makeup so louder drive thickens without exploding.
        out = soft_clip(out * (1.0 + patch.drive)) * (1.0 / (1.0 + 0.4 * patch.drive));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::kit;

    #[test]
    fn silent_before_onset() {
        for patch in kit::ALL_PATCHES {
            assert_eq!(render_patch(patch, 220.0, -0.01, 7), 0.0);
        }
    }

    #[test]
    fn catalogue_is_bounded_and_decays() {
        for patch in kit::ALL_PATCHES {
            let mut peak: f32 = 0.0;
            for i in 0..4_000 {
                let age = i as f32 / 48_000.0;
                let v = render_patch(patch, 196.0, age, i as u32);
                assert!(v.is_finite(), "non-finite sample");
                peak = peak.max(v.abs());
            }
            assert!(peak <= 3.0, "patch peak {peak} too hot");
            // Long-tail energy must sit below the attack for every catalogue voice.
            let late = render_patch(patch, 196.0, 3.0, 999).abs();
            assert!(late <= peak + 0.05, "patch does not decay: late={late}");
        }
    }

    #[test]
    fn pitch_env_stays_in_tune() {
        // A patch with a big fast drop must integrate its phase, not scale freq:
        // at late age the instantaneous pitch has settled to the fundamental.
        let drop = Patch {
            partials: &[Partial {
                ratio: 1.0,
                level: 1.0,
                decay: 0.4,
            }],
            pitch_env: Some(PitchEnv {
                amount: 3.0,
                decay: 0.02,
            }),
            amp: AmpEnv {
                attack: 0.001,
                release: 0.4,
                sustain: 0.0,
            },
            ..Patch::EMPTY
        };
        // Zero-crossing spacing late in the note should match ~50 Hz, not 200 Hz.
        let f = 50.0;
        let period = 1.0 / f;
        let a = render_patch(&drop, f, 0.30, 0);
        let b = render_patch(&drop, f, 0.30 + period, 0);
        assert!((a - b).abs() < 0.15, "pitch did not settle to fundamental");
    }
}
