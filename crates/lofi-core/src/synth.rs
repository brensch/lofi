use crate::sequencer::BeepVoice;
use crate::Micros;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderConfig {
    pub sample_rate: u32,
    pub amplitude: i16,
}

impl RenderConfig {
    pub const fn new(sample_rate: u32, amplitude: i16) -> Self {
        Self {
            sample_rate,
            amplitude,
        }
    }
}

pub fn render_beep_block(
    out: &mut [i16],
    start_root_time_us: Micros,
    voice: BeepVoice,
    cfg: RenderConfig,
) {
    for (ix, sample) in out.iter_mut().enumerate() {
        let t = start_root_time_us + (ix as Micros * 1_000_000 / cfg.sample_rate as Micros);
        *sample = voice.sample_i16(t, cfg.sample_rate, cfg.amplitude);
    }
}
