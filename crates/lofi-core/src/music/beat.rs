//! Phase-locked playback of source-coherent stem scenes.
//!
//! A scene is selected once from the shared seed. Every role then reads a stem
//! harvested from that same source performance, so distributed boxes preserve
//! its rhythm, key, progression, and four-bar phase without streaming audio or
//! sharing mutable playback cursors.

use crate::music::arrangement::Role;
use crate::music::catalog::{LoopScene, PackedElement};
use crate::music::character::vinyl;
use crate::music::dsp::soft_clip;
use crate::music::kit::Tone;
use crate::music::sample::render_sample_looped;
use crate::transport::Transport;
use crate::Micros;

const TICKS_PER_BAR: i64 = 384;

#[derive(Clone, Copy, Debug)]
pub struct BeatCtx {
    pub transport: Transport,
    pub scene: LoopScene,
}

impl BeatCtx {
    pub const fn new(transport: Transport, scene: LoopScene) -> Self {
        Self { transport, scene }
    }
}

/// Render the mono contribution assigned to one physical module.
pub fn render_role(role: Role, mesh_us: Micros, ctx: BeatCtx) -> f32 {
    match role {
        Role::Pulse => drums(mesh_us, ctx) * 0.52,
        Role::Pocket => drums(mesh_us, ctx) * 0.34,
        Role::Low => loop_element(ctx.scene.bass, mesh_us, ctx.transport) * 0.95,
        Role::Color => {
            let harmonic = loop_element(ctx.scene.harmony, mesh_us, ctx.transport);
            let texture = loop_element(ctx.scene.texture, mesh_us, ctx.transport);
            harmonic * 0.56
                + texture
                    * if ctx.scene.harmony.is_some() {
                        0.12
                    } else {
                        0.24
                    }
        }
        Role::Motif => {
            let foreground = ctx.scene.melody.or(ctx.scene.harmony);
            loop_element(foreground, mesh_us, ctx.transport)
                * if ctx.scene.melody.is_some() {
                    0.48
                } else {
                    0.18
                }
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

fn drums(mesh_us: Micros, ctx: BeatCtx) -> f32 {
    let bar = ctx.transport.tick_at(mesh_us).div_euclid(TICKS_PER_BAR);
    let phase = bar.rem_euclid(4) as usize;
    loop_element(ctx.scene.drums[phase], mesh_us, ctx.transport)
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
}
