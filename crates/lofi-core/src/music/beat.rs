//! Per-role stateless rendering. A device renders only the roles it's assigned;
//! the simulator mixes them into the full band. Every voice is a pure function
//! of mesh time + the shared arrangement `Params`, so all boxes stay in lockstep
//! and any layering of roles/features sounds coherent. The kick schedule is
//! shared knowledge, so cross-device sidechain ducking works even when the kick
//! lives on another box.

use crate::music::arrangement::{Params, Role};
use crate::music::character::{vinyl, warble};
use crate::music::kit::{Kit, Tone};
use crate::music::patch::{fast_decay, render_patch, soft_clip};
use crate::music::progression::ChordSlot;
use crate::music::progression::Progression;
use crate::music::sample::render_sample;
use crate::music::theory::midi_to_hz;
use crate::transport::Transport;
use crate::Micros;

const TICKS_PER_STEP: i64 = 24;
const STEPS_PER_BAR: i64 = 16;
const SCAN_STEPS: i64 = 48;
const DRUM_LAIDBACK_US: i64 = 12_000;

/// Everything a voice needs, resolved once per audio block: transport + seed for
/// timing, the arrangement `params` for pattern density, and the `kit` that
/// supplies the instrument timbres and tape/vinyl tone for this vibe.
#[derive(Clone, Copy, Debug)]
pub struct BeatCtx {
    pub transport: Transport,
    pub seed: u64,
    pub sample_rate: u32,
    pub params: Params,
    pub kit: &'static Kit,
    progression: Progression,
}

impl BeatCtx {
    pub fn new(
        transport: Transport,
        seed: u64,
        sample_rate: u32,
        params: Params,
        kit: &'static Kit,
    ) -> Self {
        let progression =
            Progression::generate(seed ^ (params.reharm as u64).wrapping_mul(0x2545_f491));
        Self {
            transport,
            seed,
            sample_rate,
            params,
            kit,
            progression,
        }
    }
}

/// Render one mono sample of a single role, in roughly [-1, 1] (pre-color).
pub fn render_role(role: Role, mesh_us: Micros, ctx: BeatCtx) -> f32 {
    match role {
        Role::Pulse => drum_kick(mesh_us, ctx),
        Role::Pocket => drum_snare(mesh_us, ctx) + drum_hats(mesh_us, ctx),
        Role::Low => bass(mesh_us, ctx) * pump_at(mesh_us, ctx) * 0.82,
        Role::Color => {
            keys(mesh_us, ctx) * pump_at(mesh_us, ctx) * 0.78 + texture(mesh_us, ctx) * 0.32
        }
        Role::Motif => lead(mesh_us, ctx) * pump_at(mesh_us, ctx) * 0.58,
    }
}

/// Final per-device coloring applied after summing the device's roles: warm tape
/// saturation, the vinyl air bed, and a gentle sample-value quantization for
/// grit. `tone` comes from the active kit; `air` is pre-scaled by the arrangement
/// dust level so the `Dusty` feature audibly lifts the crackle.
pub fn color(mix: f32, mesh_us: Micros, sample_rate: u32, tone: Tone) -> f32 {
    let nz = noise_index(mesh_us, sample_rate);
    let drive = 1.0 + tone.drive * 0.5;
    let saturated = soft_clip(mix * drive) / drive;
    let air = vinyl(nz.wrapping_add(101), sample_rate, tone.air);
    (saturated + air).clamp(-1.0, 1.0)
}

fn drum_kick(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let p = &ctx.params;
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    let bar = bar_at(t, mesh_us);
    let kit = ctx.kit;

    onsets::<4>(
        mesh_us,
        t,
        si,
        ctx.seed,
        DRUM_LAIDBACK_US,
        0,
        1.0,
        none,
        |s| kick_step(s, p, bar),
    )
    .into_iter()
    .flatten()
    .map(|onset| {
        let step = onset.step_index.rem_euclid(STEPS_PER_BAR);
        let sample = if matches!(step, 0 | 8) {
            kit.drums.kick_hard
        } else {
            kit.drums.kick_soft
        };
        render_sample(sample, onset.age)
    })
    .sum::<f32>()
        * 0.95
}

fn drum_snare(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let p = &ctx.params;
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    let bar = bar_at(t, mesh_us);
    let kit = ctx.kit;

    let mut snare = onsets::<4>(
        mesh_us,
        t,
        si,
        ctx.seed,
        DRUM_LAIDBACK_US,
        11,
        0.8,
        none,
        |s| snare_step(s, p, bar),
    )
    .into_iter()
    .flatten()
    .map(|onset| render_sample(kit.drums.snare_hard, onset.age))
    .sum::<f32>()
        * 0.6;
    if p.ghosts {
        snare += onsets::<4>(
            mesh_us,
            t,
            si,
            ctx.seed,
            DRUM_LAIDBACK_US,
            13,
            0.5,
            none,
            |s| ghost_step(s, p),
        )
        .into_iter()
        .flatten()
        .map(|onset| render_sample(kit.drums.snare_soft, onset.age))
        .sum::<f32>()
            * 0.18;
    }
    snare
}

fn drum_hats(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let p = &ctx.params;
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    let bar = bar_at(t, mesh_us);
    let kit = ctx.kit;

    let step_us = beat_us(t) / 4;
    let extra = p.swing_extra as i64;
    let swing = move |s: i64| {
        if s.rem_euclid(4) == 2 {
            step_us * (18 + extra) / 100
        } else {
            0
        }
    };
    onsets::<4>(mesh_us, t, si, ctx.seed, 4_000, 23, 0.35, swing, |s| {
        hat_step(s, p, bar)
    })
    .into_iter()
    .flatten()
    .map(|onset| {
        let sample = if onset.step_index.rem_euclid(4) == 0 {
            kit.drums.hat_closed
        } else {
            kit.drums.hat_pedal
        };
        render_sample(sample, onset.age)
    })
    .sum::<f32>()
        * (if p.open_hats { 0.42 } else { 0.32 })
}

fn bass(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let p = &ctx.params;
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    let w = warble(mesh_us, ctx.kit.tone);
    onsets::<4>(mesh_us, t, si, ctx.seed, 5_000, 41, 2.5, none, |s| {
        bass_step(s, p)
    })
    .into_iter()
    .flatten()
    .map(|onset| {
        let bar = onset.step_index.div_euclid(STEPS_PER_BAR);
        let step = onset.step_index.rem_euclid(STEPS_PER_BAR);
        let slot = *ctx.progression.slot_for_bar(bar);
        let mut note = slot.bass as i32;
        if p.bass_walk {
            note = bass_note(&slot, step, p);
        }
        if p.sub_bass {
            note -= 12;
        }
        render_patch(
            ctx.kit.bass,
            midi_to_hz(note.clamp(24, 60) as u8) * w,
            onset.age,
            0,
        )
    })
    .sum::<f32>()
        * 0.7
}

fn keys(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let p = &ctx.params;
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    let w = warble(mesh_us, ctx.kit.tone);
    let mut sum = 0.0;
    for onset in onsets::<4>(mesh_us, t, si, ctx.seed, 3_000, 31, 3.0, none, |s| {
        keys_step(s, p)
    })
    .into_iter()
    .flatten()
    {
        let bar = onset.step_index.div_euclid(STEPS_PER_BAR);
        let slot = *ctx.progression.slot_for_bar(bar);
        for note in slot.voicing.iter() {
            sum += render_patch(ctx.kit.keys, midi_to_hz(note) * w, onset.age, 0);
        }
        if p.rich_chords {
            let top = chord_tone(&slot, 4, 1);
            sum += render_patch(ctx.kit.keys, midi_to_hz(top) * w, onset.age, 0) * 0.5;
        }
    }
    sum * if p.keys_shape.is_multiple_of(2) {
        0.24
    } else {
        0.3
    }
}

fn lead(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let p = &ctx.params;
    if !p.lead_on {
        return 0.0;
    }
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    let w = warble(mesh_us, ctx.kit.tone);
    onsets::<4>(mesh_us, t, si, ctx.seed, 6_000, 53, 3.0, none, |s| {
        lead_step(s, p)
    })
    .into_iter()
    .flatten()
    .map(|onset| {
        let bar = onset.step_index.div_euclid(STEPS_PER_BAR);
        let step = onset.step_index.rem_euclid(STEPS_PER_BAR);
        let slot = *ctx.progression.slot_for_bar(bar);
        let phrase = bar.div_euclid(8);
        let note = lead_note(&slot, phrase, bar, step, p);
        render_patch(ctx.kit.lead, midi_to_hz(note) * w, onset.age, 0)
    })
    .sum::<f32>()
        * if p.lead_busy { 0.28 } else { 0.34 }
}

fn texture(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    let w = warble(mesh_us, ctx.kit.tone);

    let mut pad = 0.0;
    for onset in onsets::<4>(mesh_us, t, si, ctx.seed, 0, 61, 8.0, none, |s| {
        texture_step(s, &ctx.params)
    })
    .into_iter()
    .flatten()
    {
        let bar = onset.step_index.div_euclid(STEPS_PER_BAR);
        let slot = *ctx.progression.slot_for_bar(bar);
        for note in slot.voicing.iter() {
            pad += render_patch(ctx.kit.pad, midi_to_hz(note) * w, onset.age, 0);
        }
    }
    pad * if ctx.params.texture_shape.is_multiple_of(2) {
        0.18
    } else {
        0.12
    }
}

fn pump_at(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    match onset(mesh_us, t, si, ctx.seed, DRUM_LAIDBACK_US, 0, none, |s| {
        kick_step(s, &ctx.params, bar_at(t, mesh_us))
    }) {
        Some(age) if age >= 0.0 => 1.0 - 0.5 * fast_decay(age, 0.16),
        _ => 1.0,
    }
}

fn bar_at(t: &Transport, mesh_us: Micros) -> i64 {
    t.tick_at(mesh_us)
        .div_euclid(TICKS_PER_STEP * STEPS_PER_BAR)
}

fn beat_us(t: &Transport) -> i64 {
    60_000_000_000 / t.bpm_milli.max(1) as i64
}

fn none(_: i64) -> i64 {
    0
}

#[allow(clippy::too_many_arguments)]
fn onset(
    mesh_us: Micros,
    t: &Transport,
    step_index: i64,
    seed: u64,
    laidback_us: i64,
    salt: u32,
    swing: impl Fn(i64) -> i64,
    active: impl Fn(i64) -> bool,
) -> Option<f32> {
    onsets::<1>(
        mesh_us,
        t,
        step_index,
        seed,
        laidback_us,
        salt,
        f32::MAX,
        swing,
        active,
    )[0]
    .map(|onset| onset.age)
}

#[derive(Clone, Copy, Debug)]
struct Onset {
    age: f32,
    step_index: i64,
}

/// Collect a bounded set of still-audible note starts without allocation.
/// Keeping the event's absolute step lets callers retain its original pitch at
/// chord boundaries instead of abruptly retuning a release tail.
#[allow(clippy::too_many_arguments)]
fn onsets<const N: usize>(
    mesh_us: Micros,
    t: &Transport,
    step_index: i64,
    seed: u64,
    laidback_us: i64,
    salt: u32,
    max_age: f32,
    swing: impl Fn(i64) -> i64,
    active: impl Fn(i64) -> bool,
) -> [Option<Onset>; N] {
    let mut found = [None; N];
    let mut count = 0;
    for back in 0..SCAN_STEPS {
        let s = step_index - back;
        let sib = s.rem_euclid(STEPS_PER_BAR);
        if !active(sib) {
            continue;
        }
        let grid = t.root_time_for_tick(s * TICKS_PER_STEP);
        let jitter = (noise_seeded(seed, s as u32 ^ (salt << 16)) * 3_500.0) as i64;
        let onset = grid + laidback_us + jitter + swing(sib);
        let age = (mesh_us - onset) as f32 / 1_000_000.0;
        if age >= 0.0 {
            if age > max_age {
                break;
            }
            found[count] = Some(Onset { age, step_index: s });
            count += 1;
            if count == N {
                break;
            }
        }
    }
    found
}

fn kick_step(s: i64, p: &Params, bar: i64) -> bool {
    if p.half_time {
        return s == 0 || (is_fill_bar(bar) && matches!(s, 14));
    }
    let fill = is_fill_bar(bar);
    match (p.kick_variant + p.drum_fill + bar.rem_euclid(4) as u8) % 5 {
        1 => matches!(s, 0 | 6 | 10) || (fill && s == 14),
        2 => matches!(s, 0 | 3 | 8 | 11),
        3 => matches!(s, 0 | 5 | 10 | 13),
        4 => matches!(s, 0 | 7 | 12) || (fill && s == 15),
        _ => matches!(s, 0 | 7 | 10),
    }
}

fn snare_step(s: i64, p: &Params, bar: i64) -> bool {
    if p.half_time {
        s == 8 || (is_fill_bar(bar) && matches!(s, 11 | 15))
    } else {
        matches!(s, 4 | 12) || (is_fill_bar(bar) && p.drum_fill % 2 == 1 && s == 15)
    }
}

fn ghost_step(s: i64, p: &Params) -> bool {
    match p.drum_fill % 3 {
        1 => matches!(s, 3 | 7 | 15),
        2 => matches!(s, 2 | 11 | 15),
        _ => matches!(s, 7 | 15),
    }
}

fn hat_step(s: i64, p: &Params, bar: i64) -> bool {
    let bar_shape = (p.drum_fill + bar.rem_euclid(4) as u8) % 4;
    match p.hat_density {
        0 => matches!(s, 0 | 8) || (bar_shape == 2 && s == 12),
        2 => !matches!((bar_shape, s), (1, 3 | 11) | (2, 7 | 15)),
        _ => match bar_shape {
            1 => matches!(s, 0 | 2 | 6 | 8 | 10 | 14),
            2 => matches!(s, 0 | 4 | 6 | 8 | 12 | 14),
            3 => s % 2 == 0 || s == 15,
            _ => s % 2 == 0,
        },
    }
}

fn is_fill_bar(bar: i64) -> bool {
    bar.rem_euclid(4) == 3
}

fn keys_step(s: i64, p: &Params) -> bool {
    if p.keys_sparse {
        matches!(s, 0 | 12) && p.keys_shape % 2 == 1 || s == 0
    } else {
        match p.keys_shape % 3 {
            1 => matches!(s, 0 | 4 | 11),
            2 => matches!(s, 0 | 7 | 12 | 14),
            _ => matches!(s, 0 | 6 | 10),
        }
    }
}

fn bass_step(s: i64, p: &Params) -> bool {
    if p.bass_busy {
        match p.bass_shape % 3 {
            1 => matches!(s, 0 | 3 | 7 | 10 | 14),
            2 => matches!(s, 0 | 5 | 8 | 11 | 15),
            _ => matches!(s, 0 | 4 | 8 | 10 | 14),
        }
    } else {
        match p.bass_shape % 3 {
            1 => matches!(s, 0 | 7),
            2 => matches!(s, 0 | 10 | 14),
            _ => matches!(s, 0 | 10),
        }
    }
}

fn lead_step(s: i64, p: &Params) -> bool {
    if p.lead_busy {
        match p.lead_shape % 4 {
            1 => matches!(s, 1 | 5 | 9 | 13),
            2 => matches!(s, 2 | 5 | 11 | 15),
            3 => matches!(s, 3 | 6 | 10 | 14),
            _ => matches!(s, 2 | 6 | 10 | 14),
        }
    } else {
        match p.lead_shape % 4 {
            1 => matches!(s, 2 | 11),
            2 => matches!(s, 5 | 13),
            3 => matches!(s, 3 | 10 | 14),
            _ => matches!(s, 4 | 12),
        }
    }
}

fn texture_step(s: i64, p: &Params) -> bool {
    if p.texture_shape.is_multiple_of(2) {
        s == 0
    } else {
        matches!(s, 0 | 8)
    }
}

fn bass_note(slot: &ChordSlot, step: i64, p: &Params) -> i32 {
    const WALK_A: [usize; 4] = [0, 2, 1, 2];
    const WALK_B: [usize; 4] = [0, 1, 2, 3];
    const WALK_C: [usize; 4] = [0, 2, 0, 1];
    let pattern = match p.bass_shape % 3 {
        1 => WALK_B,
        2 => WALK_C,
        _ => WALK_A,
    };
    bass_chord_tone(slot, pattern[(step / 4 % 4) as usize])
}

fn lead_note(slot: &ChordSlot, phrase: i64, bar: i64, step: i64, p: &Params) -> u8 {
    const MOTIFS: [[usize; 8]; 4] = [
        [0, 1, 2, 1, 0, 2, 3, 1],
        [2, 1, 0, 1, 4, 2, 1, 0],
        [0, 2, 1, 3, 2, 0, 1, 2],
        [1, 2, 4, 2, 1, 0, 2, 1],
    ];
    let shape = (p.lead_shape % 4) as usize;
    let phrase_turn = (phrase as usize).wrapping_add((bar as usize) & 1);
    let ix = ((step / 2) as usize).wrapping_add(phrase_turn) % MOTIFS[shape].len();
    let degree = MOTIFS[shape][ix];
    chord_tone(slot, degree, 1)
}

fn chord_tone(slot: &ChordSlot, degree: usize, octave: i32) -> u8 {
    let intervals = [
        0,
        slot.chord.quality.third(),
        slot.chord.quality.fifth(),
        12,
        slot.chord.quality.third() + 12,
        slot.chord.quality.fifth() + 12,
    ];
    (slot.chord.root as i32 + intervals[degree % intervals.len()] + octave * 12).clamp(60, 84) as u8
}

fn bass_chord_tone(slot: &ChordSlot, degree: usize) -> i32 {
    let intervals = [
        0,
        slot.chord.quality.third(),
        slot.chord.quality.fifth(),
        12,
    ];
    (slot.bass as i32 + intervals[degree % intervals.len()]).clamp(28, 55)
}

fn noise_index(mesh_us: Micros, sample_rate: u32) -> u32 {
    (mesh_us as i128 * sample_rate as i128 / 1_000_000) as u32
}

fn noise_seeded(seed: u64, n: u32) -> f32 {
    let mut x = (seed as u32) ^ n.wrapping_mul(0x9e37_79b9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x21f0_aaad);
    x ^= x >> 15;
    (x as f32 / u32::MAX as f32) * 2.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::arrangement::Arrangement;

    fn ctx() -> BeatCtx {
        BeatCtx::new(
            Transport::new(0, 75_000, 96),
            7,
            48_000,
            Arrangement::at(7, &[1, 2, 3], 3).params,
            crate::music::kit::kit_for(7),
        )
    }

    #[test]
    fn roles_bounded() {
        // Sweep every kit so no vibe can produce an out-of-range or clipping mix.
        for kit in crate::music::kit::KITS {
            let mut c = ctx();
            c.kit = kit;
            for i in 0..12_000 {
                let mesh = i as Micros * 20;
                let mut sum = 0.0;
                for role in crate::music::arrangement::ROLES {
                    let v = render_role(role, mesh, c);
                    assert!(v.abs() <= 3.0, "role {role:?} out of range: {v}");
                    sum += v;
                }
                assert!(color(sum, mesh, 48_000, kit.tone).abs() <= 1.0);
            }
        }
    }

    #[test]
    fn pulse_makes_sound() {
        let energy: f32 = (0..4_000)
            .map(|i| render_role(Role::Pulse, i as Micros * 250, ctx()).abs())
            .sum();
        assert!(energy > 1.0);
    }

    #[test]
    fn late_humanized_onset_keeps_previous_note_alive() {
        let t = Transport::new(0, 75_000, 96);
        let step = 1;
        let grid = t.root_time_for_tick(step * TICKS_PER_STEP);
        let seed = (0..u64::MAX)
            .find(|&seed| noise_seeded(seed, step as u32) > 0.5)
            .unwrap();

        let age = onset(grid, &t, step, seed, 0, 0, none, |_| true);
        assert!(
            age.is_some(),
            "the previous note was cut before the late onset"
        );
    }

    #[test]
    fn chord_boundary_preserves_release_tail() {
        let c = ctx();
        let bar_us = beat_us(&c.transport) * 4;
        let boundary = c.transport.song_zero_us + bar_us * 3;
        let before = render_role(Role::Color, boundary - 21, c);
        let after = render_role(Role::Color, boundary, c);

        assert!(
            (after - before).abs() < 0.08,
            "chord transition jumped from {before} to {after}"
        );
    }
}
