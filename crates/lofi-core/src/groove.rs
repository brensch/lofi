use crate::event::Section;
use crate::transport::Transport;
use crate::Micros;

const TICKS_PER_STEP: i64 = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroovePart {
    Full,
    Drums,
    Harmony,
    Bass,
    ArpUp,
    ArpDown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GrooveConfig {
    pub sample_rate: u32,
    pub seed: u64,
    pub part: GroovePart,
}

pub fn sample_i16(
    root_time_us: Micros,
    transport: Transport,
    section: Section,
    cfg: GrooveConfig,
) -> i16 {
    let tick = transport.tick_at(root_time_us);
    let step = (tick / TICKS_PER_STEP).rem_euclid(16) as u8;
    let tick_in_step = tick.rem_euclid(TICKS_PER_STEP);
    let bar = (tick / (TICKS_PER_STEP * 16)).max(0) as u32;
    let step_start_us = transport.root_time_for_tick(tick - tick_in_step);
    let step_phase_us = root_time_us.saturating_sub(step_start_us);
    let chord = chord_notes(cfg.seed, bar);

    let mut out = 0i32;
    match cfg.part {
        GroovePart::Full => {
            out += drums(root_time_us, step, step_phase_us, section, cfg.seed);
            out += bass(root_time_us, step, step_phase_us, chord, section);
            out += harmony(root_time_us, step, step_phase_us, chord, section);
            out += arp(
                root_time_us,
                step,
                step_phase_us,
                chord,
                ArpDirection::Up,
                section,
            ) / 2;
        }
        GroovePart::Drums => out += drums(root_time_us, step, step_phase_us, section, cfg.seed),
        GroovePart::Harmony => out += harmony(root_time_us, step, step_phase_us, chord, section),
        GroovePart::Bass => out += bass(root_time_us, step, step_phase_us, chord, section),
        GroovePart::ArpUp => {
            out += arp(
                root_time_us,
                step,
                step_phase_us,
                chord,
                ArpDirection::Up,
                section,
            )
        }
        GroovePart::ArpDown => {
            out += arp(
                root_time_us,
                step,
                step_phase_us,
                chord,
                ArpDirection::Down,
                section,
            )
        }
    }

    let crushed = bitcrush(out, 7);
    crushed.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

fn drums(root_time_us: Micros, step: u8, phase_us: Micros, section: Section, seed: u64) -> i32 {
    let mut out = 0;

    if matches!(step, 0 | 7 | 10) || (matches!(section, Section::Drop) && step == 14) {
        out += kick(phase_us);
    }

    if matches!(step, 4 | 12) {
        out += snare(root_time_us, phase_us, seed);
    }

    if step.is_multiple_of(2) || (matches!(section, Section::Drop) && !step.is_multiple_of(2)) {
        out += hat(root_time_us, phase_us, seed);
    }

    if step == 15 && matches!(section, Section::Drop) {
        out += snare(root_time_us, phase_us / 2, seed ^ 0x51a7);
    }

    out
}

fn kick(phase_us: Micros) -> i32 {
    if phase_us > 170_000 {
        return 0;
    }
    let env = decay(170_000 - phase_us, 170_000, 8_500);
    let freq = 48 + ((170_000 - phase_us).max(0) as u32 * 70 / 170_000);
    tri(root_time_us_from_phase(phase_us, freq), freq, env)
}

fn snare(root_time_us: Micros, phase_us: Micros, seed: u64) -> i32 {
    if phase_us > 130_000 {
        return 0;
    }
    let env = decay(130_000 - phase_us, 130_000, 4_800);
    let noise = noise_i32(seed, root_time_us / 54, env);
    let body = tri(phase_us, 185, env / 3);
    noise + body
}

fn hat(root_time_us: Micros, phase_us: Micros, seed: u64) -> i32 {
    if phase_us > 42_000 {
        return 0;
    }
    let env = decay(42_000 - phase_us, 42_000, 2_300);
    let n1 = noise_i32(seed ^ 0x000a_11ce, root_time_us / 17, env);
    let n2 = noise_i32(seed ^ 0x5eed, root_time_us / 31, env);
    (n1 - n2) / 2
}

fn bass(root_time_us: Micros, step: u8, phase_us: Micros, chord: [u8; 4], section: Section) -> i32 {
    let note = match step {
        0 | 1 => Some(chord[0] - 12),
        6 | 7 => Some(chord[2] - 12),
        10 => Some(chord[1] - 12),
        14 if matches!(section, Section::Drop) => Some(chord[3] - 12),
        _ => None,
    };
    let Some(note) = note else {
        return 0;
    };
    if phase_us > 220_000 {
        return 0;
    }
    let env = decay(220_000 - phase_us, 220_000, 3_200);
    let freq = midi_frequency_hz(note);
    (tri(root_time_us, freq, env) + square(root_time_us, freq / 2, env / 4)) / 2
}

fn harmony(
    root_time_us: Micros,
    step: u8,
    phase_us: Micros,
    chord: [u8; 4],
    section: Section,
) -> i32 {
    let active = match section {
        Section::Intro => step == 8,
        Section::Groove => matches!(step, 0 | 8),
        Section::Drop => matches!(step, 0 | 4 | 8 | 12),
        Section::Breakdown => step == 0,
    };
    if !active || phase_us > 360_000 {
        return 0;
    }

    let env = decay(360_000 - phase_us, 360_000, 1_700);
    let wobble = ((root_time_us / 120_000).rem_euclid(5) - 2) as i32;
    let mut out = 0;
    for note in chord {
        out += tri(
            root_time_us,
            midi_frequency_hz(note + 12).saturating_add_signed(wobble),
            env / 5,
        );
    }
    out / 2
}

fn arp(
    root_time_us: Micros,
    step: u8,
    phase_us: Micros,
    chord: [u8; 4],
    direction: ArpDirection,
    section: Section,
) -> i32 {
    let gate_us = match section {
        Section::Intro => 80_000,
        Section::Groove => 120_000,
        Section::Drop => 170_000,
        Section::Breakdown => 220_000,
    };
    if phase_us > gate_us {
        return 0;
    }
    let ix = match direction {
        ArpDirection::Up => step.rem_euclid(4) as usize,
        ArpDirection::Down => (3 - step.rem_euclid(4)) as usize,
    };
    let octave = if matches!(section, Section::Drop) && step >= 8 {
        24
    } else {
        12
    };
    let env = decay(gate_us - phase_us, gate_us, 1_600);
    tri(root_time_us, midi_frequency_hz(chord[ix] + octave), env)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArpDirection {
    Up,
    Down,
}

fn chord_notes(seed: u64, bar: u32) -> [u8; 4] {
    const ROOTS: [u8; 8] = [45, 48, 50, 52, 53, 55, 57, 60];
    let ix = splitmix32(seed ^ ((bar / 2) as u64)) as usize & 7;
    let root = ROOTS[ix];
    let minor = (splitmix32(seed ^ 0x9e37_79b9 ^ bar as u64) & 1) == 0;
    [root, root + if minor { 3 } else { 4 }, root + 7, root + 10]
}

fn decay(remaining: Micros, total: Micros, amp: i32) -> i32 {
    (remaining.max(0) as i128 * amp as i128 / total.max(1) as i128) as i32
}

fn tri(time_us: Micros, freq_hz: u32, amp: i32) -> i32 {
    let period = (1_000_000 / freq_hz.max(1)) as Micros;
    let p = time_us.rem_euclid(period);
    let quarter = (period / 4).max(1);
    if p < quarter {
        -amp + (p as i32 * amp * 2 / quarter as i32)
    } else if p < quarter * 3 {
        amp - ((p - quarter) as i32 * amp * 2 / (quarter * 2) as i32)
    } else {
        -amp + ((p - quarter * 3) as i32 * amp * 2 / quarter as i32)
    }
}

fn square(time_us: Micros, freq_hz: u32, amp: i32) -> i32 {
    let period = (1_000_000 / freq_hz.max(1)) as Micros;
    if time_us.rem_euclid(period) < period / 2 {
        amp
    } else {
        -amp
    }
}

fn noise_i32(seed: u64, bucket: Micros, amp: i32) -> i32 {
    let raw = splitmix32(seed ^ bucket as u64);
    ((raw as i32 & 0xffff) - 0x8000) * amp / 0x8000
}

fn bitcrush(sample: i32, bits: u8) -> i32 {
    let shift = 16u8.saturating_sub(bits).min(15);
    (sample >> shift) << shift
}

fn root_time_us_from_phase(phase_us: Micros, _freq_hz: u32) -> Micros {
    phase_us
}

fn splitmix32(mut x: u64) -> u32 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    (x ^ (x >> 31)) as u32
}

fn midi_frequency_hz(note: u8) -> u32 {
    const TABLE: [u32; 49] = [
        131, 139, 147, 156, 165, 175, 185, 196, 208, 220, 233, 247, 262, 277, 294, 311, 330, 349,
        370, 392, 415, 440, 466, 494, 523, 554, 587, 622, 659, 698, 740, 784, 831, 880, 932, 988,
        1047, 1109, 1175, 1245, 1319, 1397, 1480, 1568, 1661, 1760, 1865, 1976, 2093,
    ];
    let clamped = note.clamp(48, 96);
    TABLE[(clamped - 48) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groove_is_deterministic_and_audible() {
        let transport = Transport::default_at(0);
        let cfg = GrooveConfig {
            sample_rate: 48_000,
            seed: 1,
            part: GroovePart::Full,
        };
        let a = sample_i16(1_000, transport, Section::Groove, cfg);
        let b = sample_i16(1_000, transport, Section::Groove, cfg);
        assert_eq!(a, b);
        assert_ne!(a, 0);
    }
}
