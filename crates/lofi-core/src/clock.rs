use crate::Micros;

const PPB_DENOM: i128 = 1_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClockModel {
    offset_us: Micros,
    rate_ppb: i32,
    last_sample: Option<Sample>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Sample {
    local_us: Micros,
    root_us: Micros,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisciplineConfig {
    pub offset_smoothing_shift: u8,
    pub rate_smoothing_shift: u8,
    pub max_rate_ppb: i32,
    pub reject_offset_us: Micros,
}

impl Default for DisciplineConfig {
    fn default() -> Self {
        Self {
            offset_smoothing_shift: 4,
            rate_smoothing_shift: 5,
            max_rate_ppb: 250_000,
            reject_offset_us: 250_000,
        }
    }
}

impl ClockModel {
    pub const fn new() -> Self {
        Self {
            offset_us: 0,
            rate_ppb: 0,
            last_sample: None,
        }
    }

    pub const fn with_offset(offset_us: Micros) -> Self {
        Self {
            offset_us,
            rate_ppb: 0,
            last_sample: None,
        }
    }

    pub const fn offset_us(&self) -> Micros {
        self.offset_us
    }

    pub const fn rate_ppb(&self) -> i32 {
        self.rate_ppb
    }

    pub fn root_from_local(&self, local_us: Micros) -> Micros {
        let rate_adjust = (local_us as i128 * self.rate_ppb as i128) / PPB_DENOM;
        local_us
            .saturating_add(self.offset_us)
            .saturating_add(saturating_i128_to_i64(rate_adjust))
    }

    pub fn local_from_root(&self, root_us: Micros) -> Micros {
        let denom = PPB_DENOM + self.rate_ppb as i128;
        let local = ((root_us - self.offset_us) as i128 * PPB_DENOM) / denom;
        saturating_i128_to_i64(local)
    }

    pub fn observe(
        &mut self,
        local_rx_us: Micros,
        observed_root_us: Micros,
        cfg: DisciplineConfig,
    ) -> Observation {
        let predicted = self.root_from_local(local_rx_us);
        let error_us = observed_root_us.saturating_sub(predicted);

        if error_us.abs() > cfg.reject_offset_us && self.last_sample.is_some() {
            return Observation {
                accepted: false,
                error_us,
                offset_us: self.offset_us,
                rate_ppb: self.rate_ppb,
            };
        }

        let offset_step = shift_toward_zero(error_us, cfg.offset_smoothing_shift);
        self.offset_us = self.offset_us.saturating_add(offset_step);

        if let Some(last) = self.last_sample {
            let local_dt = local_rx_us.saturating_sub(last.local_us);
            let root_dt = observed_root_us.saturating_sub(last.root_us);
            if local_dt > 20_000 && root_dt > 20_000 {
                let measured_ppb =
                    (((root_dt - local_dt) as i128) * PPB_DENOM / local_dt as i128) as i32;
                let measured_ppb = measured_ppb.clamp(-cfg.max_rate_ppb, cfg.max_rate_ppb);
                let rate_error = measured_ppb.saturating_sub(self.rate_ppb);
                self.rate_ppb = self
                    .rate_ppb
                    .saturating_add(shift_i32_toward_zero(rate_error, cfg.rate_smoothing_shift))
                    .clamp(-cfg.max_rate_ppb, cfg.max_rate_ppb);
            }
        }

        self.last_sample = Some(Sample {
            local_us: local_rx_us,
            root_us: observed_root_us,
        });

        Observation {
            accepted: true,
            error_us,
            offset_us: self.offset_us,
            rate_ppb: self.rate_ppb,
        }
    }
}

impl Default for ClockModel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Observation {
    pub accepted: bool,
    pub error_us: Micros,
    pub offset_us: Micros,
    pub rate_ppb: i32,
}

fn shift_toward_zero(value: i64, shift: u8) -> i64 {
    if shift == 0 {
        value
    } else if value >= 0 {
        value >> shift
    } else {
        -((-value) >> shift)
    }
}

fn shift_i32_toward_zero(value: i32, shift: u8) -> i32 {
    if shift == 0 {
        value
    } else if value >= 0 {
        value >> shift
    } else {
        -((-value) >> shift)
    }
}

fn saturating_i128_to_i64(value: i128) -> i64 {
    value.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_converges() {
        let mut clock = ClockModel::new();
        let cfg = DisciplineConfig::default();
        for t in (0..2_000_000).step_by(100_000) {
            clock.observe(t + 12_000, t, cfg);
        }
        assert!(clock.offset_us() < -8_000);
        assert!(clock.offset_us() > -13_000);
    }
}
