//! Deterministic jazz-flavoured chord progressions with voice leading.
//!
//! A small grammar of lofi-idiomatic templates (ii-V-i, i-VI-III-VII, modal
//! vamps, I-vi-ii-V) over a seeded key. Voicings are threaded chord-to-chord so
//! the colour tones move smoothly rather than jumping octaves.

use crate::music::theory::{Chord, ChordQuality, Voicing};

use ChordQuality::*;

/// Comfortable key roots (mid register); voicings re-octave around a center.
/// The roots deliberately span flats and sharps: an all-natural list makes
/// every session share overlapping pitch collections, which measurably
/// flattened cross-seed variety.
const KEYS: [u8; 8] = [57, 50, 46, 52, 48, 55, 53, 51]; // A D Bb E C G F Eb

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Major,
    Dorian,
    Aeolian,
    HarmonicMinor,
}

impl Mode {
    const fn intervals(self) -> [i32; 7] {
        match self {
            Self::Major => [0, 2, 4, 5, 7, 9, 11],
            Self::Dorian => [0, 2, 3, 5, 7, 9, 10],
            Self::Aeolian => [0, 2, 3, 5, 7, 8, 10],
            Self::HarmonicMinor => [0, 2, 3, 5, 7, 8, 11],
        }
    }
}

#[derive(Clone, Copy)]
struct Template {
    chords: &'static [(i32, ChordQuality)],
    mode: Mode,
}

/// Each template is one bar per entry: `(semitones above key root, quality)`.
const TEMPLATES: [Template; 6] = [
    Template {
        chords: &[(0, Min9), (5, Min7), (10, Dom9), (0, Min9)],
        mode: Mode::Dorian,
    },
    Template {
        chords: &[(0, Min9), (8, Maj7), (3, Maj9), (10, Dom9)],
        mode: Mode::Aeolian,
    },
    Template {
        chords: &[(2, HalfDim7), (7, Dom9), (0, Min9), (0, Min9)],
        mode: Mode::HarmonicMinor,
    },
    Template {
        chords: &[(0, Maj9), (9, Min7), (2, Min7), (7, Dom9)],
        mode: Mode::Major,
    },
    Template {
        chords: &[(0, Min9), (5, Min9), (0, Min9), (7, Dom9)],
        mode: Mode::HarmonicMinor,
    },
    Template {
        chords: &[(0, Maj9), (5, Maj7), (0, Maj9), (7, Dom9)],
        mode: Mode::Major,
    },
];

const MAX_CHORDS: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChordSlot {
    pub chord: Chord,
    pub voicing: Voicing,
    pub bass: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Progression {
    slots: [ChordSlot; MAX_CHORDS],
    len: usize,
    key: u8,
    mode: Mode,
}

impl Progression {
    /// Build a progression deterministically from a seed.
    pub fn generate(seed: u64) -> Self {
        let template = TEMPLATES[(splitmix(seed) as usize) % TEMPLATES.len()];
        let key = KEYS[(splitmix(seed ^ 0x9e37_79b9) as usize) % KEYS.len()];

        let mut slots = [ChordSlot {
            chord: Chord {
                root: key,
                quality: Min7,
            },
            voicing: Voicing { notes: [60; 4] },
            bass: 36,
        }; MAX_CHORDS];

        let mut previous = None;
        let len = template.chords.len().min(MAX_CHORDS);
        for (ix, &(offset, quality)) in template.chords.iter().take(MAX_CHORDS).enumerate() {
            let root = (key as i32 + offset).clamp(36, 72) as u8;
            let chord = Chord { root, quality };
            let voicing = previous
                .as_ref()
                .map(|voicing| chord.voicing_near(voicing))
                .unwrap_or_else(|| chord.voicing(64));
            previous = Some(voicing);
            slots[ix] = ChordSlot {
                chord,
                voicing,
                bass: place_low(key as i32 + offset),
            };
        }

        Self {
            slots,
            len,
            key,
            mode: template.mode,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn slot_for_bar(&self, bar: i64) -> &ChordSlot {
        let ix = bar.rem_euclid(self.len as i64) as usize;
        &self.slots[ix]
    }

    /// Place a tonic-relative diatonic degree in the lead register. A motif can
    /// therefore keep its contour while the chord progression moves underneath.
    pub fn scale_note(&self, degree: i8) -> u8 {
        let degree = degree as i32;
        let index = degree.rem_euclid(7) as usize;
        let octave = degree.div_euclid(7);
        let tonic = 60 + (self.key as i32).rem_euclid(12);
        (tonic + self.mode.intervals()[index] + octave * 12).clamp(55, 84) as u8
    }
}

/// Place a pitch class low, in the bass register (~MIDI 33-45).
fn place_low(note: i32) -> u8 {
    let class = note.rem_euclid(12);
    (class + 36).clamp(28, 48) as u8
}

fn splitmix(mut x: u64) -> u32 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (z ^ (z >> 31)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(Progression::generate(42), Progression::generate(42));
        assert_ne!(Progression::generate(1), Progression::generate(2));
    }

    #[test]
    fn voicings_in_register() {
        let prog = Progression::generate(7);
        for bar in 0..4 {
            let slot = prog.slot_for_bar(bar);
            assert!(slot.bass >= 28 && slot.bass <= 48);
            for n in slot.voicing.notes {
                assert!((48..=84).contains(&n), "voiced note {n} out of register");
            }
        }
    }

    #[test]
    fn scale_degrees_are_ordered_and_in_register() {
        let progression = Progression::generate(9);
        for degree in -1..10 {
            assert!((55..=84).contains(&progression.scale_note(degree)));
        }
        assert!(progression.scale_note(0) < progression.scale_note(1));
        assert!(progression.scale_note(6) < progression.scale_note(7));
    }
}
