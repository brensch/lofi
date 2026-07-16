//! Pitch, chord, and scale theory for the procedural composer. `no_std`, and
//! deterministic (frequencies come from a build-time table shared by host and
//! firmware).

use super::tables::MIDI_HZ;

/// MIDI note number to frequency in Hz. A4 (69) = 440 Hz.
pub fn midi_to_hz(note: u8) -> f32 {
    MIDI_HZ[note.min(127) as usize]
}

/// The jazz/lofi chord vocabulary. Values are the colour tones we voice; the
/// root is voiced separately by the bass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChordQuality {
    Maj7,
    Maj9,
    Maj69,
    Min7,
    Min9,
    Min11,
    Dom7,
    Dom9,
    HalfDim7,
}

impl ChordQuality {
    /// Semitone of the third above the root (minor 3 or major 3).
    pub fn third(self) -> i32 {
        match self {
            ChordQuality::Min7
            | ChordQuality::Min9
            | ChordQuality::Min11
            | ChordQuality::HalfDim7 => 3,
            _ => 4,
        }
    }

    /// Semitone of the fifth (diminished or perfect).
    pub fn fifth(self) -> i32 {
        match self {
            ChordQuality::HalfDim7 => 6,
            _ => 7,
        }
    }

    /// Semitone of the seventh (minor 7 or major 7).
    pub fn seventh(self) -> i32 {
        match self {
            ChordQuality::Maj7 | ChordQuality::Maj9 | ChordQuality::Maj69 => 11,
            _ => 10,
        }
    }

    /// Semitone of the extension we like on top (9th, or 6th for 6/9 chords).
    pub fn extension(self) -> i32 {
        match self {
            ChordQuality::Maj69 => 9, // the 6
            _ => 14,                  // the 9
        }
    }
}

/// A chord = root MIDI note + quality.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Chord {
    pub root: u8,
    pub quality: ChordQuality,
}

/// Up to four voiced MIDI notes (the colour tones, octave-placed).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Voicing {
    pub notes: [u8; 4],
}

impl Voicing {
    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        self.notes.iter().copied()
    }
}

impl Chord {
    /// Voice the colour tones (3rd, 7th, extension, 5th) close to `center`, for
    /// smooth voice leading. The caller threads `center` between chords.
    pub fn voicing(self, center: i32) -> Voicing {
        let r = self.root as i32;
        let q = self.quality;
        let notes = [
            place_near(r + q.third(), center),
            place_near(r + q.seventh(), center),
            place_near(r + q.extension(), center + 3),
            place_near(r + q.fifth(), center - 2),
        ];
        Voicing { notes }
    }

    /// Average of a voicing's notes, used as the next chord's target center.
    pub fn center_of(voicing: &Voicing) -> i32 {
        let sum: i32 = voicing.notes.iter().map(|n| *n as i32).sum();
        sum / 4
    }
}

/// The MIDI note with the same pitch class as `pc` placed nearest `center`.
fn place_near(pc: i32, center: i32) -> u8 {
    let class = pc.rem_euclid(12);
    let k = (center - class + 6).div_euclid(12);
    (class + 12 * k).clamp(0, 127) as u8
}

/// Minor/Dorian scale degrees (semitones) used for bass passing tones and leads.
pub const DORIAN: [i32; 7] = [0, 2, 3, 5, 7, 9, 10];

/// Nearest scale tone at or above `note` for the given key root and scale.
pub fn snap_to_scale(note: i32, key_root: i32, scale: &[i32]) -> i32 {
    let rel = (note - key_root).rem_euclid(12);
    let octave = (note - key_root).div_euclid(12);
    let mut best = scale[0];
    let mut best_dist = 12;
    for &deg in scale {
        let d = (deg - rel).abs();
        if d < best_dist {
            best_dist = d;
            best = deg;
        }
    }
    key_root + octave * 12 + best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a440() {
        assert!((midi_to_hz(69) - 440.0).abs() < 0.01);
        assert!((midi_to_hz(57) - 220.0).abs() < 0.01); // A3
    }

    #[test]
    fn min7_spelling() {
        let c = ChordQuality::Min7;
        assert_eq!(c.third(), 3);
        assert_eq!(c.fifth(), 7);
        assert_eq!(c.seventh(), 10);
    }

    #[test]
    fn voicing_lands_near_center() {
        let chord = Chord {
            root: 50,
            quality: ChordQuality::Min9,
        }; // D min9
        let v = chord.voicing(64);
        for n in v.notes {
            assert!((n as i32 - 64).abs() <= 9, "note {n} too far from center");
        }
    }
}
