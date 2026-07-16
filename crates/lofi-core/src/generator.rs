#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoiceRole {
    Kick,
    Bass,
    Chord(u8),
    Lead,
    Texture,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoteEvent {
    pub midi_note: u8,
    pub velocity: u8,
    pub gate_ticks: u16,
}

pub fn role_note(seed: u64, bar: u32, step: u8, role: VoiceRole) -> Option<NoteEvent> {
    let chord = chord_for_bar(seed, bar);
    match role {
        VoiceRole::Kick => {
            if step.is_multiple_of(4) || (step == 14 && chance(seed, bar, step, 3, 10)) {
                Some(NoteEvent {
                    midi_note: 36,
                    velocity: 115,
                    gate_ticks: 6,
                })
            } else {
                None
            }
        }
        VoiceRole::Bass => {
            if step.is_multiple_of(4) {
                Some(NoteEvent {
                    midi_note: chord.root,
                    velocity: 96,
                    gate_ticks: 18,
                })
            } else {
                None
            }
        }
        VoiceRole::Chord(voice) => {
            if step == 0 || step == 8 {
                let note = chord.degree((voice as usize) % 4).saturating_add(12);
                Some(NoteEvent {
                    midi_note: note,
                    velocity: 72,
                    gate_ticks: 36,
                })
            } else {
                None
            }
        }
        VoiceRole::Lead => {
            if step % 2 == 1 && chance(seed, bar, step, 4, 10) {
                let degree = (hash4(seed, bar, step, 9) % 4) as usize;
                Some(NoteEvent {
                    midi_note: chord.degree(degree).saturating_add(24),
                    velocity: 64,
                    gate_ticks: 9,
                })
            } else {
                None
            }
        }
        VoiceRole::Texture => {
            if step == 4 || step == 12 {
                Some(NoteEvent {
                    midi_note: chord.degree(2).saturating_add(24),
                    velocity: 40,
                    gate_ticks: 48,
                })
            } else {
                None
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Chord {
    root: u8,
    third: u8,
    fifth: u8,
    seventh: u8,
}

impl Chord {
    fn degree(self, ix: usize) -> u8 {
        match ix {
            0 => self.root,
            1 => self.third,
            2 => self.fifth,
            _ => self.seventh,
        }
    }
}

fn chord_for_bar(seed: u64, bar: u32) -> Chord {
    const ROOTS: [u8; 8] = [45, 48, 50, 52, 53, 55, 57, 60];
    let root = ROOTS[(hash2(seed, bar) as usize) & 7];
    let minor = (hash2(seed ^ 0x9e37_79b9, bar) & 1) == 0;
    let third = root + if minor { 3 } else { 4 };
    Chord {
        root,
        third,
        fifth: root + 7,
        seventh: root + if minor { 10 } else { 11 },
    }
}

fn chance(seed: u64, bar: u32, step: u8, numerator: u32, denominator: u32) -> bool {
    hash4(seed, bar, step, 0) % denominator < numerator
}

fn hash2(seed: u64, bar: u32) -> u32 {
    hash4(seed, bar, 0, 0)
}

fn hash4(seed: u64, bar: u32, step: u8, salt: u8) -> u32 {
    let mut x = seed ^ ((bar as u64) << 32) ^ ((step as u64) << 8) ^ salt as u64;
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    (x ^ (x >> 31)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic() {
        assert_eq!(
            role_note(123, 4, 0, VoiceRole::Bass),
            role_note(123, 4, 0, VoiceRole::Bass)
        );
        assert_ne!(
            role_note(123, 4, 0, VoiceRole::Bass),
            role_note(124, 4, 0, VoiceRole::Bass)
        );
    }
}
