//! The `no_std` audio path over the symbolic score.
//!
//! Rendering is stateless: for each output sample the lane looks back over a
//! bounded step window, recomputes the symbolic events (cheap integer math),
//! and sums the still-ringing sampled voices. Clock corrections and block
//! boundaries therefore cannot desynchronize a voice — the same property the
//! loop engine relies on.

use crate::music::arrangement::{Params, Role, BARS_PER_PHRASE};
use crate::music::character::warble;
use crate::music::kit::Tone;
use crate::music::sample::render_sample_pitched;
use crate::music::score::scene::{Bind, SymbolicScene, SHELL_VOICES};
use crate::music::score::session::Session;
use crate::music::score::{drums, tonal, Event, TICKS_PER_STEP};
use crate::music::BeatEvolution;
use crate::transport::Transport;
use crate::Micros;

const TICKS_PER_BAR: i64 = 384;
const TICKS_PER_BEAT: i64 = 96;
/// Lookback windows per lane, in steps: how long a voice can still ring.
const DRUM_LOOKBACK: i64 = 4;
const LEAD_LOOKBACK: i64 = 8;
const BASS_LOOKBACK: i64 = 10;
const KEYS_LOOKBACK: i64 = 12;

/// Everything the symbolic render path needs for one block.
#[derive(Clone, Copy, Debug)]
pub struct ScoreCtx<'a> {
    pub transport: Transport,
    pub session: &'a Session,
    pub scene: &'a SymbolicScene,
    pub evolution: BeatEvolution,
    pub tone: Tone,
}

impl ScoreCtx<'_> {
    fn step_us(&self) -> i64 {
        60_000_000_000 / i64::from(self.transport.bpm_milli.max(1)) / 4
    }

    /// The params in force at an absolute step (lookback can cross a phrase).
    fn params_at(&self, step: i64) -> &Params {
        let phrase = (step * TICKS_PER_STEP).div_euclid(TICKS_PER_BAR * BARS_PER_PHRASE);
        if phrase < self.evolution.phrase {
            &self.evolution.previous
        } else {
            &self.evolution.current
        }
    }
}

/// Render one mono sample of the roles assigned to this module.
pub fn render_role(role: Role, mesh_us: Micros, ctx: &ScoreCtx<'_>) -> f32 {
    let level = role_level(role, mesh_us, ctx);
    if level <= 0.0 {
        return 0.0;
    }
    let value = match role {
        Role::Pulse => kick(mesh_us, ctx),
        Role::Pocket => snare(mesh_us, ctx) + hats(mesh_us, ctx),
        Role::Low => bass(mesh_us, ctx) * duck_gain(mesh_us, ctx, 0.52),
        Role::Color => keys(mesh_us, ctx) * duck_gain(mesh_us, ctx, 0.42),
        Role::Motif => lead(mesh_us, ctx) * duck_gain(mesh_us, ctx, 0.34),
    };
    value * level
}

/// Mesh-wide sidechain without any traffic: the kick is symbolic and
/// deterministic, so every box — including ones not playing the kick — ducks
/// its tonal lanes under the exact same virtual hit. The pump is what makes
/// separate lanes breathe as one record.
fn duck_gain(mesh_us: Micros, ctx: &ScoreCtx<'_>, depth: f32) -> f32 {
    let current_step = current_step(mesh_us, ctx);
    for back in 0..DRUM_LOOKBACK {
        let step = current_step - back;
        let Some(event) = drums::kick_at(ctx.session, ctx.params_at(step), step, ctx.step_us())
        else {
            continue;
        };
        let onset = ctx
            .transport
            .root_time_for_tick(step * TICKS_PER_STEP)
            .saturating_add(event.delay_us);
        if mesh_us < onset {
            continue;
        }
        let age = mesh_us.saturating_sub(onset) as f32 / 1_000_000.0;
        if age > 0.35 {
            break;
        }
        // An 8 ms ease-in avoids clicking the tonal tails, then a ~140 ms
        // recovery gives the classic pocket pump.
        let ramp = (age / 0.008).min(1.0);
        let decay = libm::expf(-(age - 0.008).max(0.0) / 0.14);
        return 1.0 - depth * ramp * decay;
    }
    1.0
}

fn kick(mesh_us: Micros, ctx: &ScoreCtx<'_>) -> f32 {
    sum_lane(mesh_us, ctx, DRUM_LOOKBACK, 1.0, |step| {
        let event = drums::kick_at(ctx.session, ctx.params_at(step), step, ctx.step_us())?;
        let element = if event.bind == drums::KICK_ACCENT {
            ctx.scene.kick_accent
        } else {
            ctx.scene.kick_main
        }?;
        // The kick is pitched to the session tonic: an untuned kick is the
        // loudest note in the mix, playing in the wrong key.
        Some((
            event,
            Bind {
                element,
                rate: ctx.scene.kick_rate,
                midi: 0,
            },
        ))
    })
}

fn snare(mesh_us: Micros, ctx: &ScoreCtx<'_>) -> f32 {
    sum_lane(mesh_us, ctx, DRUM_LOOKBACK, 1.0, |step| {
        let event = drums::snare_at(ctx.session, ctx.params_at(step), step, ctx.step_us())?;
        let element = if event.bind == drums::SNARE_GHOST {
            ctx.scene.snare_ghost
        } else {
            ctx.scene.snare_main
        }?;
        Some((
            event,
            Bind {
                element,
                rate: 1.0,
                midi: 0,
            },
        ))
    })
}

fn hats(mesh_us: Micros, ctx: &ScoreCtx<'_>) -> f32 {
    sum_lane(mesh_us, ctx, DRUM_LOOKBACK, 1.0, |step| {
        let event = drums::hat_at(ctx.session, ctx.params_at(step), step, ctx.step_us())?;
        let element = if event.bind == drums::HAT_OPEN {
            ctx.scene.hat_open
        } else {
            ctx.scene.hat_closed
        }?;
        Some((
            event,
            Bind {
                element,
                rate: 1.0,
                midi: 0,
            },
        ))
    })
}

fn bass(mesh_us: Micros, ctx: &ScoreCtx<'_>) -> f32 {
    sum_lane(mesh_us, ctx, BASS_LOOKBACK, 1.0, |step| {
        let event = tonal::bass_at(ctx.session, ctx.params_at(step), step, ctx.step_us())?;
        let bind = ctx.scene.bass[event.bind as usize % 12]?;
        Some((event, bind))
    })
}

fn keys(mesh_us: Micros, ctx: &ScoreCtx<'_>) -> f32 {
    let warble = warble(mesh_us, ctx.tone);
    let current_step = current_step(mesh_us, ctx);
    let mut sum = 0.0;
    for back in 0..KEYS_LOOKBACK {
        let step = current_step - back;
        let strikes = tonal::keys_at(ctx.session, ctx.params_at(step), step, ctx.step_us());
        for event in strikes.iter().flatten() {
            let slot = event.bind as usize / SHELL_VOICES;
            let voice = event.bind as usize % SHELL_VOICES;
            let Some(bind) = ctx.scene.keys[slot.min(3)][voice] else {
                continue;
            };
            // Doubled with a light detune: one dry harvested note reads as a
            // ringtone; two slightly split copies read as an instrument.
            sum += voice_sample(mesh_us, ctx, step, *event, bind, warble) * 0.6;
            let detuned = Bind {
                rate: bind.rate * 1.0045,
                ..bind
            };
            sum += voice_sample(mesh_us, ctx, step, *event, detuned, warble) * 0.45;
        }
    }
    sum
}

fn lead(mesh_us: Micros, ctx: &ScoreCtx<'_>) -> f32 {
    let warble = warble(mesh_us, ctx.tone);
    sum_lane(mesh_us, ctx, LEAD_LOOKBACK, warble, |step| {
        let event = tonal::lead_at(ctx.session, ctx.params_at(step), step, ctx.step_us())?;
        let bind = ctx.scene.lead[event.bind as usize % ctx.scene.lead.len()]?;
        Some((event, bind))
    })
}

fn current_step(mesh_us: Micros, ctx: &ScoreCtx<'_>) -> i64 {
    ctx.transport.tick_at(mesh_us).div_euclid(TICKS_PER_STEP)
}

fn sum_lane(
    mesh_us: Micros,
    ctx: &ScoreCtx<'_>,
    lookback: i64,
    warble: f32,
    event_at: impl Fn(i64) -> Option<(Event, Bind)>,
) -> f32 {
    let current_step = current_step(mesh_us, ctx);
    let mut sum = 0.0;
    for back in 0..lookback {
        let step = current_step - back;
        if let Some((event, bind)) = event_at(step) {
            sum += voice_sample(mesh_us, ctx, step, event, bind, warble);
        }
    }
    sum
}

fn voice_sample(
    mesh_us: Micros,
    ctx: &ScoreCtx<'_>,
    step: i64,
    event: Event,
    bind: Bind,
    warble: f32,
) -> f32 {
    let onset = ctx
        .transport
        .root_time_for_tick(step * TICKS_PER_STEP)
        .saturating_add(event.delay_us);
    if mesh_us < onset {
        return 0.0;
    }
    let age = mesh_us.saturating_sub(onset) as f32 / 1_000_000.0;
    render_sample_pitched(&bind.element.sample, age, bind.rate * warble) * event.level
}

/// The four-phrase energy arc, crossfaded over the first beat of a phrase —
/// the same audible contract as the loop engine's `role_level`.
fn role_level(role: Role, mesh_us: Micros, ctx: &ScoreCtx<'_>) -> f32 {
    let previous = level_for(role, &ctx.evolution.previous, ctx.evolution.phrase - 1);
    let current = level_for(role, &ctx.evolution.current, ctx.evolution.phrase);
    let phrase_ticks = TICKS_PER_BAR * BARS_PER_PHRASE;
    let into_phrase = ctx
        .transport
        .tick_at(mesh_us)
        .rem_euclid(phrase_ticks)
        .min(TICKS_PER_BEAT);
    let amount = into_phrase as f32 / TICKS_PER_BEAT as f32;
    let smooth = amount * amount * (3.0 - 2.0 * amount);
    previous + (current - previous) * smooth
}

fn level_for(role: Role, params: &Params, phrase: i64) -> f32 {
    let arc = phrase.rem_euclid(4) as usize;
    match role {
        Role::Pulse => [0.82, 0.86, 0.9, 0.79][arc],
        Role::Pocket => [0.86, 0.96, 1.0, 0.82][arc],
        // The low end cedes real room: the first renders measured 85 %+ of
        // the energy below 150 Hz with the harmony nearly inaudible, so the
        // tonal lanes are pushed until the mid band actually registers.
        Role::Low => [0.66, 0.70, 0.74, 0.62][arc],
        Role::Color => {
            let base = [1.05, 1.15, 1.25, 0.95][arc];
            base - if params.keys_sparse { 0.12 } else { 0.0 }
        }
        Role::Motif => {
            if params.lead_on {
                [0.6, 0.95, 1.1, 0.78][arc]
            } else {
                0.0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::catalog::AI_CATALOG;
    use crate::music::Arrangement;

    fn fixture(seed: u64) -> (Session, SymbolicScene, BeatEvolution) {
        let session = Session::new(seed, &AI_CATALOG);
        let scene = SymbolicScene::resolve(&AI_CATALOG, &session);
        let roster = [1u64, 2, 3];
        let evolution = BeatEvolution {
            previous: Arrangement::at(seed, &roster, 0).params,
            current: Arrangement::at(seed, &roster, 1).params,
            phrase: 1,
            spotlight: Arrangement::at(seed, &roster, 1).spotlight,
        };
        (session, scene, evolution)
    }

    #[test]
    fn every_role_is_bounded_and_audible() {
        for seed in 0..4 {
            let (session, scene, evolution) = fixture(seed);
            let ctx = ScoreCtx {
                transport: Transport::new(0, 78_000, 96),
                session: &session,
                scene: &scene,
                evolution,
                tone: crate::music::kit::KIT_DUSTY.tone,
            };
            for role in [
                Role::Pulse,
                Role::Pocket,
                Role::Low,
                Role::Color,
                Role::Motif,
            ] {
                let mut energy = 0.0;
                for frame in 0..96_000_i64 {
                    let sample = render_role(role, frame * 1_000_000 / 48_000, &ctx);
                    assert!(sample.abs() <= 2.0, "{role:?} sample {sample}");
                    energy += sample.abs();
                }
                assert!(energy > 0.05, "{role:?} seed {seed} is silent over 2 s");
            }
        }
    }

    #[test]
    fn output_repeats_only_with_variation_over_the_cycle() {
        let (session, scene, evolution) = fixture(2);
        let ctx = ScoreCtx {
            transport: Transport::new(0, 78_000, 96),
            session: &session,
            scene: &scene,
            evolution,
            tone: crate::music::kit::KIT_DUSTY.tone,
        };
        // Humanization is seeded per absolute step, so two consecutive 4-bar
        // cycles must not be bit-identical even under the same params.
        let cycle_us = 4 * 4 * 60_000_000 / 78;
        let mut difference = 0.0;
        for offset in (0..2_000_000).step_by(250) {
            let a = render_role(Role::Pocket, 1_000_000 + offset, &ctx);
            let b = render_role(Role::Pocket, 1_000_000 + cycle_us + offset, &ctx);
            difference += (a - b).abs();
        }
        assert!(difference > 0.01, "cycles are robotically identical");
    }
}
