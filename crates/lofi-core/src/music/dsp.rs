//! Small deterministic DSP primitives shared by sample playback effects.

use super::tables::{SINE, SINE_LEN};

/// Sine of a phase in turns, interpolated from a fixed table.
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

/// Smooth cubic saturation with no transcendental call in the audio loop.
#[inline]
pub fn soft_clip(x: f32) -> f32 {
    if x >= 1.0 {
        1.0
    } else if x <= -1.0 {
        -1.0
    } else {
        x * (1.5 - 0.5 * x * x)
    }
}

/// Deterministic splitmix32 white noise in `[-1, 1]`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_are_bounded() {
        for index in 0..10_000_u32 {
            assert!(noise(index).abs() <= 1.0);
            assert!(sin_turns(index as f32 * 0.013).abs() <= 1.0);
        }
        assert_eq!(soft_clip(2.0), 1.0);
        assert_eq!(soft_clip(-2.0), -1.0);
    }
}
