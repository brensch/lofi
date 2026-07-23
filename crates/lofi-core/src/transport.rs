use crate::Micros;

pub const DEFAULT_TICKS_PER_BEAT: u32 = 96;
const MICROS_PER_MILLI_MINUTE: i64 = 60_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Transport {
    pub song_zero_us: Micros,
    pub bpm_milli: u32,
    pub ticks_per_beat: u32,
}

impl Transport {
    pub const fn new(song_zero_us: Micros, bpm_milli: u32, ticks_per_beat: u32) -> Self {
        Self {
            song_zero_us,
            bpm_milli,
            ticks_per_beat,
        }
    }

    pub const fn default_at(song_zero_us: Micros) -> Self {
        Self::new(song_zero_us, 90_000, DEFAULT_TICKS_PER_BEAT)
    }

    pub fn tick_at(&self, root_time_us: Micros) -> i64 {
        let elapsed = root_time_us.saturating_sub(self.song_zero_us);
        let ticks_per_milli_minute = i64::from(self.bpm_milli) * i64::from(self.ticks_per_beat);
        mul_div_trunc(elapsed, ticks_per_milli_minute, MICROS_PER_MILLI_MINUTE)
    }

    pub fn root_time_for_tick(&self, tick: i64) -> Micros {
        let ticks_per_milli_minute = i64::from(self.bpm_milli) * i64::from(self.ticks_per_beat);
        let elapsed_us = mul_div_ceil(tick, MICROS_PER_MILLI_MINUTE, ticks_per_milli_minute.max(1));
        self.song_zero_us.saturating_add(elapsed_us)
    }

    pub fn retimed(&self, root_time_us: Micros, new_bpm_milli: u32) -> Self {
        let tick = self.tick_at(root_time_us);
        let new_t = Self::new(0, new_bpm_milli, self.ticks_per_beat);
        let zero = root_time_us.saturating_sub(new_t.root_time_for_tick(tick));
        Self::new(zero, new_bpm_milli, self.ticks_per_beat)
    }
}

fn mul_div_trunc(value: i64, multiplier: i64, denominator: i64) -> i64 {
    let quotient = value / denominator;
    let remainder = value % denominator;
    quotient
        .wrapping_mul(multiplier)
        .wrapping_add(remainder.wrapping_mul(multiplier) / denominator)
}

fn mul_div_ceil(value: i64, multiplier: i64, denominator: i64) -> i64 {
    let quotient = value / denominator;
    let remainder = value % denominator;
    quotient.wrapping_mul(multiplier).wrapping_add(div_ceil_i64(
        remainder.wrapping_mul(multiplier),
        denominator,
    ))
}

fn div_ceil_i64(numerator: i64, denominator: i64) -> i64 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder != 0 && ((numerator > 0) == (denominator > 0)) {
        quotient + 1
    } else {
        quotient
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_time_to_ticks() {
        let t = Transport::new(1_000_000, 120_000, 96);
        assert_eq!(t.tick_at(1_500_000), 96);
        assert_eq!(t.root_time_for_tick(96), 1_500_000);
    }

    #[test]
    fn tempo_change_keeps_tick_continuous() {
        let old = Transport::new(0, 120_000, 96);
        let changed = old.retimed(1_000_000, 90_000);
        assert_eq!(old.tick_at(1_000_000), changed.tick_at(1_000_000));
    }

    #[test]
    fn decomposed_math_matches_wide_reference() {
        for bpm_milli in [40_000, 72_000, 80_000, 123_456, 200_000] {
            let transport = Transport::new(-123_456, bpm_milli, 96);
            for elapsed in [
                -86_400_000_000_i64,
                -2_666,
                0,
                2_666,
                86_400_000_000,
                432_000_000_000,
            ] {
                let root = transport.song_zero_us + elapsed;
                let expected =
                    (elapsed as i128 * i128::from(bpm_milli) * 96 / 60_000_000_000_i128) as i64;
                assert_eq!(transport.tick_at(root), expected);
            }
            for tick in [-1_000_000_i64, -1, 0, 1, 1_000_000] {
                let numerator = i128::from(tick) * 60_000_000_000_i128;
                let denominator = i128::from(bpm_milli) * 96;
                let quotient = numerator / denominator;
                let remainder = numerator % denominator;
                let expected =
                    quotient + i128::from(remainder != 0 && ((numerator > 0) == (denominator > 0)));
                assert_eq!(
                    transport.root_time_for_tick(tick),
                    transport.song_zero_us + expected as i64,
                );
            }
        }
    }
}
