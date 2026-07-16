//! Stateful master coloring. Unlike the voices and the dry mix (which are pure
//! functions of mesh time), an IIR filter is recursive, so it streams per device
//! on the audio thread. That's fine: FX state is local and need not match across
//! the mesh — only the note timing must, and that comes from the dry signal.

use core::f32::consts::TAU;

/// Biquad lowpass (RBJ cookbook). Rolls off the highs for the classic "behind a
/// closed door" lofi tone.
#[derive(Clone, Copy, Debug)]
pub struct Lowpass {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Lowpass {
    pub fn new(cutoff_hz: f32, sample_rate: u32, q: f32) -> Self {
        let w0 = TAU * cutoff_hz / sample_rate as f32;
        let cos_w0 = libm::cosf(w0);
        let alpha = libm::sinf(w0) / (2.0 * q);
        let a0 = 1.0 + alpha;
        Self {
            b0: (1.0 - cos_w0) / 2.0 / a0,
            b1: (1.0 - cos_w0) / a0,
            b2: (1.0 - cos_w0) / 2.0 / a0,
            a1: -2.0 * cos_w0 / a0,
            a2: (1.0 - alpha) / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_dc_attenuates_highs() {
        let mut lp = Lowpass::new(3_500.0, 48_000, 0.707);
        // DC settles to ~unity.
        let mut dc = 0.0;
        for _ in 0..2_000 {
            dc = lp.process(1.0);
        }
        assert!((dc - 1.0).abs() < 0.05, "dc gain off: {dc}");

        // A near-Nyquist tone is strongly attenuated.
        let mut lp = Lowpass::new(3_500.0, 48_000, 0.707);
        let mut peak: f32 = 0.0;
        for n in 0..2_000 {
            let x = if n % 2 == 0 { 1.0 } else { -1.0 };
            peak = peak.max(lp.process(x).abs());
        }
        assert!(peak < 0.2, "highs not attenuated: {peak}");
    }
}
