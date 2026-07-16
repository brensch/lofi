//! Curated, redistributable sample descriptors.
//!
//! The byte arrays live in flash/wasm read-only data. Source provenance and
//! conversion details are recorded in `assets/samples/README.md`.

use super::sample::Sample;

const RATE: u32 = 22_050;

pub static KICK_HARD: Sample = Sample::mulaw(
    include_bytes!("../../../../assets/samples/encoded/kick-hard.ulaw"),
    RATE,
    0.95,
);
pub static KICK_SOFT: Sample = Sample::mulaw(
    include_bytes!("../../../../assets/samples/encoded/kick-soft.ulaw"),
    RATE,
    0.82,
);
pub static SNARE_HARD: Sample = Sample::mulaw(
    include_bytes!("../../../../assets/samples/encoded/snare-hard.ulaw"),
    RATE,
    0.9,
);
pub static SNARE_SOFT: Sample = Sample::mulaw(
    include_bytes!("../../../../assets/samples/encoded/snare-soft.ulaw"),
    RATE,
    0.72,
);
pub static HAT_CLOSED: Sample = Sample::mulaw(
    include_bytes!("../../../../assets/samples/encoded/hat-closed.ulaw"),
    RATE,
    0.72,
);
pub static HAT_PEDAL: Sample = Sample::mulaw(
    include_bytes!("../../../../assets/samples/encoded/hat-pedal.ulaw"),
    RATE,
    0.58,
);

#[derive(Clone, Copy, Debug)]
pub struct DrumBank {
    pub kick_hard: &'static Sample,
    pub kick_soft: &'static Sample,
    pub snare_hard: &'static Sample,
    pub snare_soft: &'static Sample,
    pub hat_closed: &'static Sample,
    pub hat_pedal: &'static Sample,
}

pub static ACOUSTIC_DRUMS: DrumBank = DrumBank {
    kick_hard: &KICK_HARD,
    kick_soft: &KICK_SOFT,
    snare_hard: &SNARE_HARD,
    snare_soft: &SNARE_SOFT,
    hat_closed: &HAT_CLOSED,
    hat_pedal: &HAT_PEDAL,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bank_is_present_and_small() {
        let samples = [
            ACOUSTIC_DRUMS.kick_hard,
            ACOUSTIC_DRUMS.kick_soft,
            ACOUSTIC_DRUMS.snare_hard,
            ACOUSTIC_DRUMS.snare_soft,
            ACOUSTIC_DRUMS.hat_closed,
            ACOUSTIC_DRUMS.hat_pedal,
        ];
        let bytes: usize = samples.iter().map(|sample| sample.len()).sum();
        assert!(samples.iter().all(|sample| !sample.is_empty()));
        assert!(bytes < 128 * 1024, "starter drum bank is {bytes} bytes");
    }
}
