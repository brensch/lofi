use crate::Micros;

pub const DEFAULT_TICKS_PER_BEAT: u32 = 96;

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
        let numerator = elapsed as i128 * self.bpm_milli as i128 * self.ticks_per_beat as i128;
        (numerator / 60_000_000_000i128) as i64
    }

    pub fn root_time_for_tick(&self, tick: i64) -> Micros {
        let denom = self.bpm_milli as i128 * self.ticks_per_beat as i128;
        let numerator = tick as i128 * 60_000_000_000i128;
        let elapsed_us = div_ceil_i128(numerator, denom);
        self.song_zero_us
            .saturating_add(elapsed_us.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
    }

    pub fn retimed(&self, root_time_us: Micros, new_bpm_milli: u32) -> Self {
        let tick = self.tick_at(root_time_us);
        let new_t = Self::new(0, new_bpm_milli, self.ticks_per_beat);
        let zero = root_time_us.saturating_sub(new_t.root_time_for_tick(tick));
        Self::new(zero, new_bpm_milli, self.ticks_per_beat)
    }
}

fn div_ceil_i128(numerator: i128, denominator: i128) -> i128 {
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
}
