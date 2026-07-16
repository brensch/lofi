//! Phase-locked playback of source-coherent stem scenes.
//!
//! A scene is selected once from the shared seed. Every role then reads a stem
//! harvested from that same source performance, so distributed boxes preserve
//! its rhythm, key, progression, and four-bar phase without streaming audio or
//! sharing mutable playback cursors.

use crate::music::arrangement::{Params, Role, BARS_PER_PHRASE};
use crate::music::catalog::{LoopScene, PackedElement};
use crate::music::character::vinyl;
use crate::music::dsp::soft_clip;
use crate::music::kit::Tone;
use crate::music::sample::{render_sample, render_sample_looped};
use crate::transport::Transport;
use crate::Micros;

const TICKS_PER_BAR: i64 = 384;
const TICKS_PER_BEAT: i64 = 96;
const TICKS_PER_STEP: i64 = 24;
const STEPS_PER_BAR: i64 = 16;
const HIT_LOOKBACK_STEPS: i64 = 4;

#[derive(Clone, Copy, Debug)]
pub struct BeatCtx {
    pub transport: Transport,
    pub scene: LoopScene,
    evolution: Option<BeatEvolution>,
}

#[derive(Clone, Copy, Debug)]
pub struct BeatEvolution {
    pub previous: Params,
    pub current: Params,
    pub phrase: i64,
}

impl BeatCtx {
    pub const fn new(transport: Transport, scene: LoopScene) -> Self {
        Self {
            transport,
            scene,
            evolution: None,
        }
    }

    pub const fn with_evolution(mut self, evolution: BeatEvolution) -> Self {
        self.evolution = Some(evolution);
        self
    }
}

/// Render the mono contribution assigned to one physical module.
pub fn render_role(role: Role, mesh_us: Micros, ctx: BeatCtx) -> f32 {
    match role {
        Role::Pulse => kick(mesh_us, ctx) * role_level(Role::Pulse, mesh_us, ctx),
        Role::Pocket => {
            (snare(mesh_us, ctx) * 0.72 + hats(mesh_us, ctx) * 0.28)
                * role_level(Role::Pocket, mesh_us, ctx)
        }
        Role::Low => {
            loop_element(ctx.scene.bass, mesh_us, ctx.transport)
                * role_level(Role::Low, mesh_us, ctx)
        }
        Role::Color => {
            let harmonic = loop_element(ctx.scene.harmony, mesh_us, ctx.transport);
            let texture = loop_element(ctx.scene.texture, mesh_us, ctx.transport);
            let level = role_level(Role::Color, mesh_us, ctx);
            harmonic * level + texture * level * texture_ratio(ctx)
        }
        Role::Motif => {
            let foreground = ctx.scene.melody.or(ctx.scene.harmony);
            let fallback = if ctx.scene.melody.is_some() {
                1.0
            } else {
                0.42
            };
            loop_element(foreground, mesh_us, ctx.transport)
                * role_level(Role::Motif, mesh_us, ctx)
                * fallback
        }
    }
}

/// Final per-device coloring applied after summing that device's roles.
pub fn color(mix: f32, mesh_us: Micros, sample_rate: u32, tone: Tone) -> f32 {
    let nz = noise_index(mesh_us, sample_rate);
    let drive = 1.0 + tone.drive * 0.5;
    let saturated = soft_clip(mix * drive) / drive;
    let air = vinyl(nz.wrapping_add(101), sample_rate, tone.air);
    (saturated + air).clamp(-1.0, 1.0)
}

fn kick(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let params = ctx.evolution.map(|evolution| evolution.current);
    render_hits(
        ctx.scene.kick,
        mesh_us,
        ctx.transport,
        |step| {
            let position = step.rem_euclid(STEPS_PER_BAR);
            let Some(params) = params else {
                return if matches!(position, 0 | 8) { 1.0 } else { 0.0 };
            };
            if params.half_time {
                return if position == 0 { 1.0 } else { 0.0 };
            }
            let active = match params.kick_variant % 3 {
                1 => matches!(position, 0 | 7 | 10),
                2 => matches!(position, 0 | 6 | 11),
                _ => matches!(position, 0 | 8),
            };
            if active {
                1.0
            } else {
                0.0
            }
        },
        |_| 0,
    )
}

fn snare(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let params = ctx.evolution.map(|evolution| evolution.current);
    render_hits(
        ctx.scene.snare,
        mesh_us,
        ctx.transport,
        |step| {
            let position = step.rem_euclid(STEPS_PER_BAR);
            if matches!(position, 4 | 12) {
                return 1.0;
            }
            let Some(params) = params else {
                return 0.0;
            };
            if params.ghosts && position == 10 {
                return 0.22;
            }
            let phrase_step = step.rem_euclid(STEPS_PER_BAR * BARS_PER_PHRASE);
            if params.drum_fill > 0
                && phrase_step >= STEPS_PER_BAR * (BARS_PER_PHRASE - 1)
                && matches!(position, 14 | 15)
            {
                return 0.34;
            }
            0.0
        },
        |_| 0,
    )
}

fn hats(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let step_us = 60_000_000_000_i64 / ctx.transport.bpm_milli.max(1) as i64 / 4;
    let params = ctx.evolution.map(|evolution| evolution.current);
    let cadence = ctx
        .evolution
        .is_some_and(|evolution| evolution.phrase.rem_euclid(4) == 3);
    let density = if cadence {
        1
    } else {
        params.map(|value| value.hat_density).unwrap_or(1)
    };
    let swing = params
        .map(|value| 10 + value.swing_extra.min(10) as i64)
        .unwrap_or(12);
    render_hits(
        ctx.scene.hat,
        mesh_us,
        ctx.transport,
        |step| {
            let position = step.rem_euclid(STEPS_PER_BAR);
            let active = match density {
                0 => position.rem_euclid(4) == 0,
                2.. => true,
                _ => position.rem_euclid(2) == 0,
            };
            if !active {
                return 0.0;
            }
            if density >= 2 && position.rem_euclid(2) == 1 {
                0.64
            } else if params.is_some_and(|value| value.open_hats) && position.rem_euclid(4) == 2 {
                1.12
            } else {
                1.0
            }
        },
        |step| {
            let delayed = if density >= 2 {
                step.rem_euclid(2) == 1
            } else {
                step.rem_euclid(4) == 2
            };
            if delayed {
                step_us * swing / 100
            } else {
                0
            }
        },
    )
}

fn render_hits(
    element: Option<PackedElement>,
    mesh_us: Micros,
    transport: Transport,
    velocity: impl Fn(i64) -> f32,
    delay_us: impl Fn(i64) -> Micros,
) -> f32 {
    let Some(element) = element else {
        return 0.0;
    };
    let current_step = transport.tick_at(mesh_us).div_euclid(TICKS_PER_STEP);
    let mut sum = 0.0;
    for back in 0..HIT_LOOKBACK_STEPS {
        let step = current_step - back;
        let velocity = velocity(step);
        if velocity <= 0.0 {
            continue;
        }
        let start = transport.root_time_for_tick(step * TICKS_PER_STEP) + delay_us(step);
        let age = mesh_us.saturating_sub(start) as f32 / 1_000_000.0;
        if mesh_us >= start {
            sum += render_sample(&element.sample, age) * velocity;
        }
    }
    sum
}

fn role_level(role: Role, mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let Some(evolution) = ctx.evolution else {
        return match role {
            Role::Pulse => 0.86,
            Role::Pocket => 1.0,
            Role::Low => 0.64,
            Role::Color => 0.56,
            Role::Motif => 0.48,
        };
    };
    let previous = level_for(role, evolution.previous, evolution.phrase - 1);
    let current = level_for(role, evolution.current, evolution.phrase);
    let phrase_ticks = TICKS_PER_BAR * BARS_PER_PHRASE;
    let ticks_into_phrase = ctx
        .transport
        .tick_at(mesh_us)
        .rem_euclid(phrase_ticks)
        .min(TICKS_PER_BEAT);
    let amount = ticks_into_phrase as f32 / TICKS_PER_BEAT as f32;
    let smooth = amount * amount * (3.0 - 2.0 * amount);
    previous + (current - previous) * smooth
}

fn level_for(role: Role, params: Params, phrase: i64) -> f32 {
    let arc = phrase.rem_euclid(4) as usize;
    match role {
        Role::Pulse => [0.80, 0.84, 0.88, 0.80][arc],
        Role::Pocket => [0.88, 0.96, 1.0, 0.86][arc],
        Role::Low => {
            let base = [0.54, 0.59, 0.64, 0.50][arc];
            base + if params.bass_busy { 0.05 } else { 0.0 }
                + if params.sub_bass { 0.03 } else { 0.0 }
        }
        Role::Color => {
            let base = [0.48, 0.55, 0.60, 0.48][arc];
            base + if params.rich_chords { 0.07 } else { 0.0 }
                - if params.keys_sparse { 0.10 } else { 0.0 }
        }
        Role::Motif => {
            let base = [0.14, 0.34, 0.46, 0.22][arc];
            if params.lead_on {
                base + if params.lead_busy { 0.05 } else { 0.0 }
            } else {
                0.0
            }
        }
    }
}

fn texture_ratio(ctx: BeatCtx) -> f32 {
    let Some(evolution) = ctx.evolution else {
        return if ctx.scene.harmony.is_some() {
            0.22
        } else {
            0.44
        };
    };
    let base = if ctx.scene.harmony.is_some() {
        0.20
    } else {
        0.42
    };
    base + (evolution.current.texture_shape % 3) as f32 * 0.05
}

fn loop_element(element: Option<PackedElement>, mesh_us: Micros, transport: Transport) -> f32 {
    let Some(element) = element else {
        return 0.0;
    };
    let bars = i64::from(element.bars.max(1));
    let cycle_ticks = bars * TICKS_PER_BAR;
    let tick = transport.tick_at(mesh_us);
    let cycle_start = tick.div_euclid(cycle_ticks) * cycle_ticks;
    let cycle_start_us = transport.root_time_for_tick(cycle_start);
    let age = mesh_us.saturating_sub(cycle_start_us) as f32 / 1_000_000.0;
    let source_bpm = f32::from(element.bpm.max(1));
    let playback_rate = transport.bpm_milli as f32 / (source_bpm * 1_000.0);
    render_sample_looped(&element.sample, age, playback_rate)
}

fn noise_index(mesh_us: Micros, sample_rate: u32) -> u32 {
    ((mesh_us.max(0) as i128 * sample_rate as i128) / 1_000_000) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::catalog::AI_CATALOG;

    fn ctx(seed: u64) -> BeatCtx {
        BeatCtx::new(
            Transport::new(0, 80_000, 96),
            AI_CATALOG.loop_scene(seed).unwrap(),
        )
    }

    #[test]
    fn every_role_is_bounded_and_audible_across_scenes() {
        for seed in 0..3 {
            let ctx = ctx(seed);
            for role in [
                Role::Pulse,
                Role::Pocket,
                Role::Low,
                Role::Color,
                Role::Motif,
            ] {
                let mut energy = 0.0;
                for frame in 0..48_000 {
                    let sample = render_role(role, frame * 1_000_000 / 48_000, ctx);
                    assert!(sample.abs() <= 1.5, "{role:?} sample {sample}");
                    energy += sample.abs();
                }
                assert!(energy > 0.1, "{role:?} seed {seed} is silent");
            }
        }
    }

    #[test]
    fn scene_loops_repeat_on_the_shared_four_bar_boundary() {
        let ctx = ctx(1);
        let four_bars_us = 12_000_000;
        for role in [Role::Low, Role::Color, Role::Motif] {
            for offset in (0..100_000).step_by(1_000) {
                let first = render_role(role, 2_000_000 + offset, ctx);
                let repeated = render_role(role, 2_000_000 + four_bars_us + offset, ctx);
                assert!((first - repeated).abs() < 0.001, "{role:?} did not repeat");
            }
        }
    }

    #[test]
    fn phrase_levels_crossfade_for_exactly_one_shared_beat() {
        let roster = [1, 2, 3];
        let previous = crate::music::Arrangement::at(2, &roster, 0);
        let current = crate::music::Arrangement::at(2, &roster, 1);
        let ctx = ctx(2).with_evolution(BeatEvolution {
            previous: previous.params,
            current: current.params,
            phrase: 1,
        });

        let boundary_us = 24_000_000;
        assert_eq!(
            role_level(Role::Pulse, boundary_us, ctx),
            level_for(Role::Pulse, previous.params, 0)
        );
        assert_eq!(
            role_level(Role::Pulse, boundary_us + 750_000, ctx),
            level_for(Role::Pulse, current.params, 1)
        );
    }
}
