//! The disciplined mesh clock.
//!
//! Wraps the affine [`ClockModel`] (offset + rate, NTP-style) with the two
//! things musical scheduling needs on top:
//!
//! 1. A cold-start *step*: the first reference observation snaps the clock onto
//!    the mesh timeline instead of slewing in slowly.
//! 2. A *monotonic* scheduling output: [`MeshClock::schedule_now`] never goes
//!    backwards even when discipline nudges the offset down, so a scheduled beat
//!    can never be re-fired or skipped. A backward correction shows up as the
//!    clock briefly holding still, never reversing.

use crate::clock::{ClockModel, DisciplineConfig, Observation};
use crate::Micros;

/// Consecutive rejected observations before we conclude the clock is genuinely
/// in the wrong place (not just a bad packet) and step onto the reference.
const STEPOUT_REJECTS: u8 = 3;

#[derive(Clone, Copy, Debug)]
pub struct MeshClock {
    model: ClockModel,
    last_emitted_us: Micros,
    has_reference: bool,
    reject_streak: u8,
}

impl Default for MeshClock {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshClock {
    pub const fn new() -> Self {
        Self {
            model: ClockModel::new(),
            last_emitted_us: Micros::MIN,
            has_reference: false,
            reject_streak: 0,
        }
    }

    /// True once at least one reference observation has been applied.
    pub const fn has_reference(&self) -> bool {
        self.has_reference
    }

    pub const fn offset_us(&self) -> Micros {
        self.model.offset_us()
    }

    pub const fn rate_ppb(&self) -> i32 {
        self.model.rate_ppb()
    }

    /// Mesh time for a local instant. May be non-monotonic across discipline
    /// steps; use this for *measurement*, not for scheduling.
    pub fn mesh_from_local(&self, local_us: Micros) -> Micros {
        self.model.root_from_local(local_us)
    }

    /// Local instant for a target mesh time (for scheduling future edges).
    pub fn local_from_mesh(&self, mesh_us: Micros) -> Micros {
        self.model.local_from_root(mesh_us)
    }

    /// Monotonic mesh time for scheduling. Never decreases for non-decreasing
    /// `local_us`, even immediately after a backward discipline correction.
    pub fn schedule_now(&mut self, local_us: Micros) -> Micros {
        let mesh = self
            .model
            .root_from_local(local_us)
            .max(self.last_emitted_us);
        self.last_emitted_us = mesh;
        mesh
    }

    /// Become the mesh reference (the root): freeze the current mapping and stop
    /// expecting upstream corrections. Mesh time stays continuous.
    pub fn anchor_as_root(&mut self) {
        self.has_reference = true;
    }

    /// Apply a reference observation: at local time `local_rx_us`, an upstream
    /// peer says mesh time is `observed_mesh_us`. The first one steps; later
    /// ones slew under `cfg` (the engine narrows `cfg` for high-weight peers).
    pub fn observe(
        &mut self,
        local_rx_us: Micros,
        observed_mesh_us: Micros,
        cfg: DisciplineConfig,
    ) -> Observation {
        if !self.has_reference {
            self.model = ClockModel::with_offset(observed_mesh_us.saturating_sub(local_rx_us));
            self.has_reference = true;
            self.reject_streak = 0;
            return self.stepped();
        }

        let observation = self.model.observe(local_rx_us, observed_mesh_us, cfg);
        if observation.accepted {
            self.reject_streak = 0;
            return observation;
        }

        // A persistently rejected error means the clock is genuinely in the
        // wrong place (a real phase step), not a single bad packet. Step onto
        // the reference instead of rejecting forever.
        self.reject_streak = self.reject_streak.saturating_add(1);
        if self.reject_streak >= STEPOUT_REJECTS {
            self.model.reanchor(local_rx_us, observed_mesh_us);
            self.reject_streak = 0;
            return self.stepped();
        }
        observation
    }

    fn stepped(&self) -> Observation {
        Observation {
            accepted: true,
            error_us: 0,
            offset_us: self.model.offset_us(),
            rate_ppb: self.model.rate_ppb(),
        }
    }

    /// Force a re-step on the next observation (used when the mesh root changes
    /// on a cluster merge, so we realign quickly instead of slewing for minutes).
    pub fn request_resync(&mut self) {
        self.has_reference = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> DisciplineConfig {
        DisciplineConfig {
            reject_offset_us: 50_000,
            ..DisciplineConfig::default()
        }
    }

    #[test]
    fn cold_start_steps_onto_timeline() {
        let mut clock = MeshClock::new();
        clock.observe(1_000_000, 5_000_000, cfg());
        // Immediately reads close to the reference, no slow slew-in.
        assert_eq!(clock.mesh_from_local(1_000_000), 5_000_000);
    }

    #[test]
    fn converges_under_drift() {
        // Reference runs 80us per 100ms faster than our local clock.
        let mut clock = MeshClock::new();
        let mut err_late = 0;
        for step in 0..400 {
            let local = step * 100_000;
            let reference = local + 50_000 + step * 80; // offset + drift
            let obs = clock.observe(local, reference, cfg());
            if step > 200 {
                err_late = err_late.max(obs.error_us.abs());
            }
        }
        assert!(err_late < 1_500, "late error too large: {err_late}");
    }

    #[test]
    fn schedule_now_never_goes_backwards() {
        let mut clock = MeshClock::new();
        clock.observe(0, 1_000_000, cfg());
        let mut last = clock.schedule_now(0);
        // Hammer it with observations that jerk the offset both ways.
        for step in 1..2_000 {
            let local = step * 1_000;
            let jitter = if step % 2 == 0 { 9_000 } else { -9_000 };
            clock.observe(local, local + 1_000_000 + jitter, cfg());
            let now = clock.schedule_now(local);
            assert!(now >= last, "mesh went backwards at step {step}");
            last = now;
        }
    }
}
