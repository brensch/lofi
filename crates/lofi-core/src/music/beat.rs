//! Per-role stateless rendering. A device renders only the roles it's assigned;
//! the simulator mixes them into the full band. Every voice is a pure function
//! of mesh time + the shared arrangement `Params`, so all boxes stay in lockstep
//! and any layering of roles/features sounds coherent. The kick schedule is
//! shared knowledge, so cross-device sidechain ducking works even when the kick
//! lives on another box.

use crate::music::arrangement::{Params, Role};
use crate::music::catalog::{ElementKind, PackedCatalog, AI_CATALOG};
use crate::music::character::{vinyl, warble};
use crate::music::content::{motif_for, signature_for, GrooveSignature};
use crate::music::dsp::{fast_decay, soft_clip};
use crate::music::kit::{Kit, Tone};
use crate::music::progression::ChordSlot;
use crate::music::progression::Progression;
use crate::music::sample::{render_sample, render_sample_pitched};
use crate::music::theory::midi_to_hz;
use crate::transport::Transport;
use crate::Micros;

const TICKS_PER_STEP: i64 = 24;
const STEPS_PER_BAR: i64 = 16;
const SCAN_STEPS: i64 = 48;

/// Everything a voice needs, resolved once per audio block: transport + seed for
/// timing, the arrangement `params` for pattern density, and the `kit` that
/// supplies the tape/vinyl tone profile for this vibe.
#[derive(Clone, Copy, Debug)]
pub struct BeatCtx {
    pub transport: Transport,
    pub seed: u64,
    pub sample_rate: u32,
    pub params: Params,
    pub kit: &'static Kit,
    pub catalog: &'static PackedCatalog,
    pub signature: &'static GrooveSignature,
    pub tone: Tone,
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
        let signature = signature_for(seed);
        Self {
            transport,
            seed,
            sample_rate,
            params,
            kit,
            catalog: &AI_CATALOG,
            signature,
            tone: signature.blend_tone(kit.tone),
            progression,
        }
    }

    /// Use a catalogue memory-mapped by firmware instead of the bundled pack.
    pub const fn with_catalog(mut self, catalog: &'static PackedCatalog) -> Self {
        self.catalog = catalog;
        self
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
    let signature = ctx.signature;

    onsets::<4>(
        mesh_us,
        t,
        si,
        ctx.seed,
        signature.kick_delay_us,
        signature.humanize_us / 2,
        0,
        1.0,
        none,
        |step| kick_step(step, p, signature),
    )
    .into_iter()
    .flatten()
    .map(|onset| {
        let step = onset.step_index.rem_euclid(STEPS_PER_BAR);
        ctx.catalog
            .choose(
                ElementKind::Kick,
                ctx.seed ^ onset.step_index as u64 ^ (step as u64).rotate_left(17),
            )
            .map(|element| render_sample(&element.sample, onset.age))
            .unwrap_or(0.0)
    })
    .sum::<f32>()
        * signature.kick_gain
}

fn drum_snare(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let p = &ctx.params;
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    let signature = ctx.signature;

    let mut snare = onsets::<4>(
        mesh_us,
        t,
        si,
        ctx.seed,
        signature.snare_delay_us,
        signature.humanize_us,
        11,
        0.8,
        none,
        |step| snare_step(step, p, signature),
    )
    .into_iter()
    .flatten()
    .map(|onset| {
        ctx.catalog
            .choose(
                ElementKind::Snare,
                ctx.seed ^ (onset.step_index as u64).rotate_left(23),
            )
            .map(|element| render_sample(&element.sample, onset.age))
            .unwrap_or(0.0)
    })
    .sum::<f32>()
        * signature.snare_gain;
    if p.ghosts {
        snare += onsets::<4>(
            mesh_us,
            t,
            si,
            ctx.seed,
            signature.snare_delay_us + 4_000,
            signature.humanize_us,
            13,
            0.5,
            none,
            |step| ghost_step(step_in_bar(step), p, bar_for_step(step)),
        )
        .into_iter()
        .flatten()
        .map(|onset| {
            ctx.catalog
                .choose(
                    ElementKind::Snare,
                    ctx.seed ^ (onset.step_index as u64).rotate_left(29) ^ 0x0047_484f_5354,
                )
                .map(|element| render_sample(&element.sample, onset.age))
                .unwrap_or(0.0)
        })
        .sum::<f32>()
            * 0.18;
    }
    snare
}

fn drum_hats(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let p = &ctx.params;
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    let signature = ctx.signature;

    let step_us = beat_us(t) / 4;
    let extra = p.swing_extra as i64;
    let swing = move |s: i64| {
        if s.rem_euclid(4) == 2 {
            step_us * (signature.swing_percent as i64 + extra.min(10)) / 100
        } else {
            0
        }
    };
    onsets::<4>(
        mesh_us,
        t,
        si,
        ctx.seed,
        signature.hat_delay_us,
        signature.humanize_us,
        23,
        0.35,
        swing,
        |step| hat_step(step, p, signature),
    )
    .into_iter()
    .flatten()
    .map(|onset| {
        ctx.catalog
            .choose(
                ElementKind::Hat,
                ctx.seed ^ (onset.step_index as u64).rotate_left(11),
            )
            .map(|element| render_sample(&element.sample, onset.age))
            .unwrap_or(0.0)
    })
    .sum::<f32>()
        * signature.hat_gain
        * (if p.open_hats { 1.22 } else { 1.0 })
}

fn bass(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let p = &ctx.params;
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    let w = warble(mesh_us, ctx.tone);
    let signature = ctx.signature;
    onsets::<4>(
        mesh_us,
        t,
        si,
        ctx.seed,
        signature.tonal_delay_us,
        signature.humanize_us / 2,
        41,
        2.5,
        none,
        |step| bass_step(step, p, signature),
    )
    .into_iter()
    .flatten()
    .map(|onset| {
        let bar = onset.step_index.div_euclid(STEPS_PER_BAR);
        let step = onset.step_index.rem_euclid(STEPS_PER_BAR);
        let slot = *ctx.progression.slot_for_bar(bar);
        let mut note = slot.bass as i32;
        if signature.bass_approach.active(onset.step_index) {
            note = ctx.progression.slot_for_bar(bar + 1).bass as i32 - 1;
        } else if p.bass_walk {
            note = bass_note(&slot, step, p);
        }
        if p.sub_bass {
            note -= 12;
        }
        let phrase = onset.step_index.div_euclid(128) as u64;
        render_pitched(
            ctx.catalog,
            ElementKind::BassNote,
            note.clamp(24, 60) as u8,
            onset.age,
            w,
            ctx.seed ^ phrase.rotate_left(13),
        )
    })
    .sum::<f32>()
        * signature.bass_gain
}

fn keys(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let p = &ctx.params;
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    let w = warble(mesh_us, ctx.tone);
    let signature = ctx.signature;
    let mut sum = 0.0;
    for onset in onsets::<4>(
        mesh_us,
        t,
        si,
        ctx.seed,
        signature.tonal_delay_us,
        signature.humanize_us / 2,
        31,
        3.0,
        none,
        |step| keys_step(step_in_bar(step), p),
    )
    .into_iter()
    .flatten()
    {
        let bar = onset.step_index.div_euclid(STEPS_PER_BAR);
        let slot = *ctx.progression.slot_for_bar(bar);
        let step = step_in_bar(onset.step_index);
        let selector = ctx.seed ^ (bar.div_euclid(8) as u64).rotate_left(19);
        if step == 0 {
            for note in slot.voicing.iter() {
                sum += render_pitched(
                    ctx.catalog,
                    ElementKind::KeysNote,
                    note,
                    onset.age,
                    w,
                    selector,
                );
            }
        } else {
            let answer = slot.voicing.notes[2 + (bar.rem_euclid(2) as usize)];
            sum += render_pitched(
                ctx.catalog,
                ElementKind::KeysNote,
                answer,
                onset.age,
                w,
                selector,
            ) * 0.8;
        }
        if p.rich_chords && step == 0 {
            let top = chord_tone(&slot, 4, 1);
            sum += render_pitched(
                ctx.catalog,
                ElementKind::KeysNote,
                top,
                onset.age,
                w,
                selector,
            ) * 0.5;
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
    let w = warble(mesh_us, ctx.tone);
    let signature = ctx.signature;
    let motif = motif_for(signature, p.lead_shape);
    onsets::<4>(
        mesh_us,
        t,
        si,
        ctx.seed,
        signature.tonal_delay_us,
        signature.humanize_us,
        53,
        3.0,
        none,
        |step| motif.active_at(step),
    )
    .into_iter()
    .flatten()
    .map(|onset| {
        let event = motif
            .event_at(onset.step_index)
            .expect("onset came from motif event");
        let note = ctx.progression.scale_note(event.degree);
        let velocity = event.velocity as f32 / 127.0;
        let phrase = onset.step_index.div_euclid(128) as u64;
        render_pitched(
            ctx.catalog,
            ElementKind::LeadNote,
            note,
            onset.age,
            w,
            ctx.seed ^ phrase.rotate_left(29),
        ) * velocity
    })
    .sum::<f32>()
        * if p.lead_busy { 0.28 } else { 0.34 }
}

fn texture(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    let mut bed = 0.0;
    for onset in onsets::<4>(mesh_us, t, si, ctx.seed, 0, 0, 61, 8.0, none, |step| {
        texture_step(step, &ctx.params)
    })
    .into_iter()
    .flatten()
    {
        let selector = ctx.seed ^ (onset.step_index as u64).rotate_left(7);
        if let Some(element) = ctx.catalog.choose(ElementKind::TextureLoop, selector) {
            bed += render_sample(&element.sample, onset.age);
        }
    }
    bed * if ctx.params.texture_shape.is_multiple_of(2) {
        0.18
    } else {
        0.12
    }
}

fn pump_at(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let t = &ctx.transport;
    let si = t.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    match onset(
        mesh_us,
        t,
        si,
        ctx.seed,
        ctx.signature.kick_delay_us,
        ctx.signature.humanize_us / 2,
        0,
        none,
        |step| kick_step(step, &ctx.params, ctx.signature),
    ) {
        Some(age) if age >= 0.0 => 1.0 - 0.5 * fast_decay(age, 0.16),
        _ => 1.0,
    }
}

fn step_in_bar(step: i64) -> i64 {
    step.rem_euclid(STEPS_PER_BAR)
}

fn bar_for_step(step: i64) -> i64 {
    step.div_euclid(STEPS_PER_BAR)
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
    humanize_us: i64,
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
        humanize_us,
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
    humanize_us: i64,
    salt: u32,
    max_age: f32,
    swing: impl Fn(i64) -> i64,
    active: impl Fn(i64) -> bool,
) -> [Option<Onset>; N] {
    let mut found = [None; N];
    let mut count = 0;
    for back in 0..SCAN_STEPS {
        let s = step_index - back;
        if !active(s) {
            continue;
        }
        let grid = t.root_time_for_tick(s * TICKS_PER_STEP);
        let jitter = (noise_seeded(seed, s as u32 ^ (salt << 16)) * humanize_us as f32) as i64;
        let onset = grid + laidback_us + jitter + swing(step_in_bar(s));
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

fn kick_step(step: i64, p: &Params, signature: &GrooveSignature) -> bool {
    let s = step_in_bar(step);
    let bar = bar_for_step(step);
    if p.half_time {
        return s == 0 || (is_fill_bar(bar) && matches!(s, 14));
    }
    let core = signature.kick.active(step);
    let variation = match p.kick_variant % 3 {
        1 => s == 11,
        2 => s == 6,
        _ => false,
    };
    let turnaround = is_fill_bar(bar)
        && match (p.kick_variant + p.drum_fill) % 3 {
            1 => s == 15,
            2 => s == 13,
            _ => s == 14,
        };
    core || variation || turnaround
}

fn snare_step(step: i64, p: &Params, signature: &GrooveSignature) -> bool {
    let s = step_in_bar(step);
    let bar = bar_for_step(step);
    if p.half_time {
        s == 8 || (is_fill_bar(bar) && matches!(s, 11 | 15))
    } else {
        signature.snare.active(step) || (is_fill_bar(bar) && p.drum_fill % 2 == 1 && s == 15)
    }
}

fn ghost_step(s: i64, p: &Params, bar: i64) -> bool {
    s == 7 || (is_fill_bar(bar) && p.drum_fill % 2 == 1 && s == 15)
}

fn hat_step(step: i64, p: &Params, signature: &GrooveSignature) -> bool {
    let s = step_in_bar(step);
    let bar = bar_for_step(step);
    let core = match p.hat_density {
        0 => matches!(s, 0 | 8),
        2 => s % 2 == 0 || matches!(s, 3 | 11),
        _ => signature.hats.active(step),
    };
    core || (is_fill_bar(bar) && p.drum_fill > 0 && s == 15)
}

fn is_fill_bar(bar: i64) -> bool {
    bar.rem_euclid(4) == 3
}

fn keys_step(s: i64, p: &Params) -> bool {
    if p.keys_sparse {
        s == 0
    } else {
        match p.keys_shape % 3 {
            1 => matches!(s, 0 | 10),
            2 => matches!(s, 0 | 6),
            _ => matches!(s, 0 | 12),
        }
    }
}

fn bass_step(step: i64, p: &Params, signature: &GrooveSignature) -> bool {
    let s = step_in_bar(step);
    if p.bass_busy {
        match p.bass_shape % 3 {
            1 => matches!(s, 0 | 7 | 10 | 14),
            2 => matches!(s, 0 | 5 | 11 | 15),
            _ => matches!(s, 0 | 8 | 10 | 14),
        }
    } else {
        signature.bass.active(step)
            || signature.bass_approach.active(step)
            || (p.bass_shape % 3 == 2 && s == 14)
    }
}

fn texture_step(step: i64, p: &Params) -> bool {
    let s = step_in_bar(step);
    let bar = bar_for_step(step);
    if p.texture_shape.is_multiple_of(2) {
        bar.rem_euclid(2) == 0 && s == 0
    } else {
        s == 0
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

#[inline]
fn render_pitched(
    catalog: &'static PackedCatalog,
    kind: ElementKind,
    target: u8,
    age: f32,
    warble_ratio: f32,
    selector: u64,
) -> f32 {
    let Some(element) = catalog.nearest_note(kind, target, selector) else {
        return 0.0;
    };
    let Some(root) = element.root_semitone else {
        return 0.0;
    };
    let ratio = midi_to_hz(target) / midi_to_hz(root) * warble_ratio;
    render_sample_pitched(&element.sample, age, ratio)
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

        let age = onset(grid, &t, step, seed, 0, 3_500, 0, none, |_| true);
        assert!(
            age.is_some(),
            "the previous note was cut before the late onset"
        );
    }

    #[test]
    fn harvested_drum_pattern_repeats_every_four_bars() {
        let p = Params::base_for_test(7);
        let signature = &crate::music::content::SIGNATURE_FLOATING;
        for step in 0..64 {
            assert_eq!(
                kick_step(step, &p, signature),
                kick_step(step + 64, &p, signature)
            );
            assert_eq!(
                hat_step(step, &p, signature),
                hat_step(step + 64, &p, signature)
            );
        }
        for bar in 0..4 {
            assert!(snare_step(bar * 16 + 4, &p, signature));
            assert!(snare_step(bar * 16 + 12, &p, signature));
        }
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
