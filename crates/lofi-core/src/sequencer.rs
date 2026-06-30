use crate::Micros;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BeepVoice {
    pub beat_period_us: Micros,
    pub duration_us: Micros,
    pub frequency_hz: u32,
}

impl BeepVoice {
    pub const fn new(beat_period_us: Micros, duration_us: Micros, frequency_hz: u32) -> Self {
        Self {
            beat_period_us,
            duration_us,
            frequency_hz,
        }
    }

    pub fn sample_i16(&self, root_time_us: Micros, sample_rate: u32, amplitude: i16) -> i16 {
        let phase_us = root_time_us.rem_euclid(self.beat_period_us);
        if phase_us >= self.duration_us {
            return 0;
        }

        let sample_index = ((phase_us as i128 * sample_rate as i128) / 1_000_000) as i64;
        let period_samples = (sample_rate / self.frequency_hz).max(1) as i64;
        if sample_index.rem_euclid(period_samples) < period_samples / 2 {
            amplitude
        } else {
            -amplitude
        }
    }
}
