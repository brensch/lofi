//! Reviewed musical content distilled from offline AI references.
//!
//! The source renders never ship here. This catalogue stores compact musical
//! signatures: four-bar grids, scale-degree motifs, dynamics, and production
//! measurements. All lookup is fixed-size, deterministic, and allocation-free.

use super::kit::Tone;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoteEvent {
    /// Sixteenth-note position in a four-bar phrase (`0..64`).
    pub step: u8,
    /// Diatonic degree relative to the progression tonic; may cross octaves.
    pub degree: i8,
    /// MIDI-like velocity retained as a compact dynamic contour.
    pub velocity: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct Motif {
    pub name: &'static str,
    pub events: &'static [NoteEvent],
    pattern: StepPattern,
}

impl Motif {
    pub fn active_at(&self, step: i64) -> bool {
        self.pattern.active(step)
    }

    pub fn event_at(&self, step: i64) -> Option<NoteEvent> {
        let phrase_step = step.rem_euclid(64) as u8;
        self.events
            .iter()
            .copied()
            .find(|event| event.step == phrase_step)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StepPattern(u64);

impl StepPattern {
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }

    pub fn active(self, step: i64) -> bool {
        self.0 & (1_u64 << step.rem_euclid(64)) != 0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GrooveSignature {
    pub name: &'static str,
    pub kick: StepPattern,
    pub snare: StepPattern,
    pub hats: StepPattern,
    pub bass: StepPattern,
    pub bass_approach: StepPattern,
    pub motif: u8,
    pub kick_delay_us: i64,
    pub snare_delay_us: i64,
    pub hat_delay_us: i64,
    pub tonal_delay_us: i64,
    pub humanize_us: i64,
    pub swing_percent: u8,
    pub cutoff_hz: f32,
    pub wow: f32,
    pub flutter: f32,
    pub drive: f32,
    pub air: f32,
    pub kick_gain: f32,
    pub snare_gain: f32,
    pub hat_gain: f32,
    pub bass_gain: f32,
}

impl GrooveSignature {
    /// Blend measured reference character with a kit's instrument-specific tone.
    pub fn blend_tone(self, kit: Tone) -> Tone {
        Tone {
            wow: (kit.wow + self.wow) * 0.5,
            flutter: (kit.flutter + self.flutter) * 0.5,
            cutoff_hz: (kit.cutoff_hz + self.cutoff_hz) * 0.5,
            drive: (kit.drive + self.drive) * 0.5,
            air: (kit.air + self.air) * 0.5,
        }
    }
}

const fn steps(values: &[u8]) -> StepPattern {
    let mut bits = 0_u64;
    let mut index = 0;
    while index < values.len() {
        bits |= 1_u64 << values[index];
        index += 1;
    }
    StepPattern::new(bits)
}

// These are deliberately rewritten scale-degree contours, not raw model MIDI.
// They retain the references' density, rests, register, and call/answer shape.
const FLOATING_A: &[NoteEvent] = &[
    NoteEvent {
        step: 4,
        degree: 4,
        velocity: 62,
    },
    NoteEvent {
        step: 12,
        degree: 3,
        velocity: 54,
    },
    NoteEvent {
        step: 20,
        degree: 2,
        velocity: 58,
    },
    NoteEvent {
        step: 28,
        degree: 0,
        velocity: 50,
    },
    NoteEvent {
        step: 36,
        degree: 2,
        velocity: 64,
    },
    NoteEvent {
        step: 43,
        degree: 3,
        velocity: 56,
    },
    NoteEvent {
        step: 52,
        degree: 1,
        velocity: 52,
    },
    NoteEvent {
        step: 60,
        degree: 0,
        velocity: 48,
    },
];

const WINDOWLIGHT: &[NoteEvent] = &[
    NoteEvent {
        step: 1,
        degree: 2,
        velocity: 66,
    },
    NoteEvent {
        step: 14,
        degree: 0,
        velocity: 54,
    },
    NoteEvent {
        step: 15,
        degree: 1,
        velocity: 61,
    },
    NoteEvent {
        step: 17,
        degree: 2,
        velocity: 68,
    },
    NoteEvent {
        step: 24,
        degree: 2,
        velocity: 56,
    },
    NoteEvent {
        step: 31,
        degree: 0,
        velocity: 51,
    },
    NoteEvent {
        step: 33,
        degree: 1,
        velocity: 64,
    },
    NoteEvent {
        step: 36,
        degree: 4,
        velocity: 53,
    },
    NoteEvent {
        step: 41,
        degree: 2,
        velocity: 59,
    },
    NoteEvent {
        step: 47,
        degree: 4,
        velocity: 62,
    },
];

const POLAROID: &[NoteEvent] = &[
    NoteEvent {
        step: 2,
        degree: 1,
        velocity: 57,
    },
    NoteEvent {
        step: 7,
        degree: 5,
        velocity: 64,
    },
    NoteEvent {
        step: 13,
        degree: 4,
        velocity: 53,
    },
    NoteEvent {
        step: 18,
        degree: 3,
        velocity: 60,
    },
    NoteEvent {
        step: 26,
        degree: 4,
        velocity: 49,
    },
    NoteEvent {
        step: 34,
        degree: 3,
        velocity: 63,
    },
    NoteEvent {
        step: 42,
        degree: 2,
        velocity: 55,
    },
    NoteEvent {
        step: 50,
        degree: 1,
        velocity: 52,
    },
    NoteEvent {
        step: 58,
        degree: 0,
        velocity: 47,
    },
];

const LAST_TRAIN: &[NoteEvent] = &[
    NoteEvent {
        step: 3,
        degree: 4,
        velocity: 60,
    },
    NoteEvent {
        step: 10,
        degree: 3,
        velocity: 51,
    },
    NoteEvent {
        step: 14,
        degree: 2,
        velocity: 56,
    },
    NoteEvent {
        step: 22,
        degree: 1,
        velocity: 48,
    },
    NoteEvent {
        step: 29,
        degree: 2,
        velocity: 62,
    },
    NoteEvent {
        step: 38,
        degree: 4,
        velocity: 58,
    },
    NoteEvent {
        step: 45,
        degree: 3,
        velocity: 54,
    },
    NoteEvent {
        step: 54,
        degree: 1,
        velocity: 50,
    },
    NoteEvent {
        step: 61,
        degree: 0,
        velocity: 45,
    },
];

const BLUE_HOUR: &[NoteEvent] = &[
    NoteEvent {
        step: 5,
        degree: 2,
        velocity: 55,
    },
    NoteEvent {
        step: 9,
        degree: 3,
        velocity: 61,
    },
    NoteEvent {
        step: 17,
        degree: 4,
        velocity: 58,
    },
    NoteEvent {
        step: 25,
        degree: 2,
        velocity: 50,
    },
    NoteEvent {
        step: 30,
        degree: 1,
        velocity: 47,
    },
    NoteEvent {
        step: 37,
        degree: 2,
        velocity: 62,
    },
    NoteEvent {
        step: 46,
        degree: 0,
        velocity: 53,
    },
    NoteEvent {
        step: 55,
        degree: -1,
        velocity: 48,
    },
    NoteEvent {
        step: 62,
        degree: 0,
        velocity: 44,
    },
];

const SOFT_FOCUS: &[NoteEvent] = &[
    NoteEvent {
        step: 2,
        degree: 0,
        velocity: 52,
    },
    NoteEvent {
        step: 8,
        degree: 2,
        velocity: 59,
    },
    NoteEvent {
        step: 15,
        degree: 1,
        velocity: 50,
    },
    NoteEvent {
        step: 21,
        degree: 4,
        velocity: 63,
    },
    NoteEvent {
        step: 31,
        degree: 3,
        velocity: 48,
    },
    NoteEvent {
        step: 34,
        degree: 2,
        velocity: 57,
    },
    NoteEvent {
        step: 40,
        degree: 4,
        velocity: 61,
    },
    NoteEvent {
        step: 48,
        degree: 3,
        velocity: 51,
    },
    NoteEvent {
        step: 57,
        degree: 1,
        velocity: 47,
    },
];

pub const MOTIFS: &[Motif] = &[
    Motif {
        name: "FLOATING A",
        events: FLOATING_A,
        pattern: steps(&[4, 12, 20, 28, 36, 43, 52, 60]),
    },
    Motif {
        name: "WINDOWLIGHT",
        events: WINDOWLIGHT,
        pattern: steps(&[1, 14, 15, 17, 24, 31, 33, 36, 41, 47]),
    },
    Motif {
        name: "POLAROID",
        events: POLAROID,
        pattern: steps(&[2, 7, 13, 18, 26, 34, 42, 50, 58]),
    },
    Motif {
        name: "LAST TRAIN",
        events: LAST_TRAIN,
        pattern: steps(&[3, 10, 14, 22, 29, 38, 45, 54, 61]),
    },
    Motif {
        name: "BLUE HOUR",
        events: BLUE_HOUR,
        pattern: steps(&[5, 9, 17, 25, 30, 37, 46, 55, 62]),
    },
    Motif {
        name: "SOFT FOCUS",
        events: SOFT_FOCUS,
        pattern: steps(&[2, 8, 15, 21, 31, 34, 40, 48, 57]),
    },
];

/// Reference A: open hats, a restrained kick, and the darkest foreground.
pub const SIGNATURE_FLOATING: GrooveSignature = GrooveSignature {
    name: "FLOATING",
    kick: steps(&[0, 7, 10, 16, 23, 26, 32, 39, 42, 48, 55, 58]),
    snare: steps(&[4, 12, 20, 28, 36, 44, 52, 60]),
    hats: steps(&[
        2, 4, 6, 10, 12, 14, 18, 20, 22, 26, 30, 34, 36, 38, 42, 44, 46, 50, 54, 58, 60, 62,
    ]),
    bass: steps(&[0, 10, 16, 26, 32, 42, 48, 58]),
    bass_approach: steps(&[14, 30, 46, 62]),
    motif: 0,
    kick_delay_us: 4_000,
    snare_delay_us: 4_000,
    hat_delay_us: 4_000,
    tonal_delay_us: 4_000,
    humanize_us: 700,
    swing_percent: 17,
    cutoff_hz: 5_800.0,
    wow: 0.0032,
    flutter: 0.0008,
    drive: 0.17,
    air: 0.28,
    kick_gain: 0.9,
    snare_gain: 0.56,
    hat_gain: 0.29,
    bass_gain: 0.64,
};

/// Reference B: a later kick pocket, darker drums, and a busier guitar answer.
pub const SIGNATURE_WINDOWLIGHT: GrooveSignature = GrooveSignature {
    name: "WINDOWLIGHT",
    kick: steps(&[0, 7, 14, 16, 22, 26, 32, 39, 42, 48, 55, 58]),
    snare: steps(&[4, 12, 20, 28, 36, 44, 52, 60]),
    hats: steps(&[
        2, 6, 8, 10, 14, 18, 22, 24, 26, 30, 34, 38, 40, 42, 46, 50, 54, 58, 62,
    ]),
    bass: steps(&[0, 7, 16, 23, 32, 39, 48, 55]),
    bass_approach: steps(&[14, 30, 46, 62]),
    motif: 1,
    kick_delay_us: 7_000,
    snare_delay_us: 7_000,
    hat_delay_us: 7_000,
    tonal_delay_us: 7_000,
    humanize_us: 900,
    swing_percent: 20,
    cutoff_hz: 4_700.0,
    wow: 0.0038,
    flutter: 0.0010,
    drive: 0.2,
    air: 0.22,
    kick_gain: 0.96,
    snare_gain: 0.61,
    hat_gain: 0.25,
    bass_gain: 0.7,
};

/// Reference C: steady three-kick bars, late hats, and the softest high end.
pub const SIGNATURE_POLAROID: GrooveSignature = GrooveSignature {
    name: "POLAROID",
    kick: steps(&[0, 6, 8, 16, 22, 24, 32, 38, 40, 48, 54, 56]),
    snare: steps(&[4, 12, 20, 28, 36, 44, 52, 60]),
    hats: steps(&[2, 6, 10, 14, 18, 22, 26, 30, 34, 38, 42, 46, 50, 54, 58, 62]),
    bass: steps(&[0, 8, 16, 24, 32, 40, 48, 56]),
    bass_approach: steps(&[15, 31, 47, 63]),
    motif: 2,
    kick_delay_us: 6_000,
    snare_delay_us: 6_000,
    hat_delay_us: 6_000,
    tonal_delay_us: 6_000,
    humanize_us: 800,
    swing_percent: 18,
    cutoff_hz: 3_900.0,
    wow: 0.0042,
    flutter: 0.0011,
    drive: 0.18,
    air: 0.32,
    kick_gain: 0.88,
    snare_gain: 0.58,
    hat_gain: 0.24,
    bass_gain: 0.58,
};

pub const SIGNATURES: &[GrooveSignature] = &[
    SIGNATURE_FLOATING,
    SIGNATURE_WINDOWLIGHT,
    SIGNATURE_POLAROID,
];

pub fn signature_for(seed: u64) -> &'static GrooveSignature {
    &SIGNATURES[(mix64(seed ^ 0x7369_676e_6174_7572) % SIGNATURES.len() as u64) as usize]
}

pub fn motif_for(signature: &GrooveSignature, variation: u8) -> &'static Motif {
    &MOTIFS[(signature.motif as usize + variation as usize) % MOTIFS.len()]
}

fn mix64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_is_bounded_and_ordered() {
        for motif in MOTIFS {
            assert!((7..=12).contains(&motif.events.len()));
            assert!(motif
                .events
                .windows(2)
                .all(|pair| pair[0].step < pair[1].step));
            assert!(motif.events.iter().all(|event| event.step < 64));
            for step in 0..64 {
                assert_eq!(motif.active_at(step), motif.event_at(step).is_some());
            }
        }
    }

    #[test]
    fn every_signature_is_reachable() {
        let mut seen = [false; 3];
        for seed in 0..500 {
            let selected = signature_for(seed).name;
            let index = SIGNATURES
                .iter()
                .position(|item| item.name == selected)
                .unwrap();
            seen[index] = true;
        }
        assert!(seen.iter().all(|value| *value));
    }

    #[test]
    fn distributed_roles_share_a_tight_timing_pocket() {
        for signature in SIGNATURES {
            let delays = [
                signature.kick_delay_us,
                signature.snare_delay_us,
                signature.hat_delay_us,
                signature.tonal_delay_us,
            ];
            let minimum = delays.iter().min().unwrap();
            let maximum = delays.iter().max().unwrap();
            assert!(
                maximum - minimum <= 2_000,
                "{} timing spread",
                signature.name
            );
            assert!(
                signature.humanize_us <= 1_000,
                "{} humanization",
                signature.name
            );
        }
    }
}
