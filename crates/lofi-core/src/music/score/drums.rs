//! Symbolic kick, snare, and hat events on the measured signature grids.
//!
//! Every event is a pure function of `(session, params, absolute step)`.
//! Micro-timing carries the lane's measured pocket delay, the shared swing,
//! and bounded humanization — never randomness at render time.

use crate::music::arrangement::Params;
use crate::music::score::session::Session;
use crate::music::score::{Event, STEPS_PER_BAR, STEPS_PER_CYCLE};

/// Bind codes for the drum lanes' scene voices.
pub const KICK_MAIN: u8 = 0;
pub const KICK_ACCENT: u8 = 1;
pub const SNARE_MAIN: u8 = 0;
pub const SNARE_GHOST: u8 = 1;
pub const HAT_CLOSED: u8 = 0;
pub const HAT_OPEN: u8 = 1;

/// Steps into an 8-bar phrase (16 steps × 8 bars).
const STEPS_PER_PHRASE: i64 = STEPS_PER_BAR * 8;

/// Swing plus pocket micro-delay for any lane event at a bar position.
/// Off-beat eighths take the full measured swing; off sixteenths take half.
pub fn swing_us(bar_pos: i64, swing_percent: i64, step_us: i64) -> i64 {
    if bar_pos.rem_euclid(4) == 2 {
        step_us * swing_percent / 100
    } else if bar_pos.rem_euclid(2) == 1 {
        step_us * swing_percent / 200
    } else {
        0
    }
}

fn micro(session: &Session, lane: u64, step: i64, base_delay: i64, step_us: i64) -> i64 {
    let signature = session.signature;
    let swing = i64::from(signature.swing_percent);
    base_delay
        + swing_us(step.rem_euclid(STEPS_PER_BAR), swing, step_us)
        + super::humanize_us(session.seed, lane, step, signature.humanize_us)
}

pub fn kick_at(session: &Session, params: &Params, step: i64, step_us: i64) -> Option<Event> {
    let bar_pos = step.rem_euclid(STEPS_PER_BAR);
    let cycle = step.rem_euclid(STEPS_PER_CYCLE);
    let signature = session.signature;

    let active = if params.half_time {
        bar_pos == 0
    } else {
        let mut active = signature.kick.active(cycle);
        if params.kick_variant % 3 == 1 {
            // Thin variant: keep the downbeat and the first syncopation only.
            active &= matches!(bar_pos, 0..=10);
        }
        if params.kick_variant % 3 == 2 && cycle >= 48 && bar_pos == 14 {
            // Push variant: one extra late kick in the last bar of the cycle.
            active = true;
        }
        active
    };
    if !active {
        return None;
    }

    let accent = bar_pos == 0;
    let level = if accent { 1.0 } else { 0.84 } * signature.kick_gain;
    Some(Event {
        bind: if cycle == 0 { KICK_ACCENT } else { KICK_MAIN },
        midi: 0,
        level,
        delay_us: micro(session, LANE_KICK, step, signature.kick_delay_us, step_us),
    })
}

pub fn snare_at(session: &Session, params: &Params, step: i64, step_us: i64) -> Option<Event> {
    let bar_pos = step.rem_euclid(STEPS_PER_BAR);
    let phrase_pos = step.rem_euclid(STEPS_PER_PHRASE);
    let signature = session.signature;

    let backbeat = if params.half_time {
        bar_pos == 8
    } else {
        matches!(bar_pos, 4 | 12)
    };
    if backbeat {
        let wobble = (super::event_hash(session.seed, 2, step) % 9) as f32 / 100.0;
        return Some(Event {
            bind: SNARE_MAIN,
            midi: 0,
            level: (0.96 + wobble) * signature.snare_gain,
            delay_us: micro(session, 2, step, signature.snare_delay_us, step_us),
        });
    }

    // Phrase-end fill: soft sixteenth pickups into the boundary.
    if params.drum_fill > 0
        && phrase_pos >= STEPS_PER_PHRASE - 4
        && !params.half_time
        && bar_pos >= 12
    {
        let rise = (phrase_pos - (STEPS_PER_PHRASE - 4)) as f32;
        return Some(Event {
            bind: SNARE_GHOST,
            midi: 0,
            level: (0.30 + rise * 0.06) * signature.snare_gain,
            delay_us: micro(session, 2, step, signature.snare_delay_us, step_us),
        });
    }

    // Ghost notes: a consistent pocket, not a per-bar dice roll. The placement
    // hashes the bar's position inside the four-bar cycle so the ghost lands
    // in the same spot every cycle and reads as intent.
    if params.ghosts && !params.half_time {
        let cycle_bar = step.rem_euclid(STEPS_PER_CYCLE).div_euclid(STEPS_PER_BAR);
        let h = super::event_hash(session.seed, 3, cycle_bar);
        if h.is_multiple_of(2) {
            let ghost_pos = [7, 10, 2, 15][(h >> 8) as usize % 4];
            if bar_pos == ghost_pos {
                return Some(Event {
                    bind: SNARE_GHOST,
                    midi: 0,
                    level: 0.22 * signature.snare_gain,
                    delay_us: micro(session, 2, step, signature.snare_delay_us, step_us),
                });
            }
        }
    }
    None
}

pub fn hat_at(session: &Session, params: &Params, step: i64, step_us: i64) -> Option<Event> {
    let bar_pos = step.rem_euclid(STEPS_PER_BAR);
    let cycle = step.rem_euclid(STEPS_PER_CYCLE);
    let signature = session.signature;

    let active = if params.half_time {
        bar_pos.rem_euclid(4) == 0
    } else {
        match params.hat_density {
            0 => bar_pos.rem_euclid(4) == 2,
            2.. => true,
            _ => signature.hats.active(cycle),
        }
    };
    if !active {
        return None;
    }

    let open = params.open_hats && bar_pos == 6 && !params.half_time;
    // Off-beat sixteenths sit lower so a dense grid still breathes.
    let dip = if bar_pos.rem_euclid(2) == 1 {
        0.62
    } else {
        1.0
    };
    let wobble = (super::event_hash(session.seed, 4, step) % 13) as f32 / 100.0;
    let level = if open { 1.15 } else { dip * (0.9 + wobble) } * signature.hat_gain;
    let swing_extra = i64::from(params.swing_extra.min(10));
    let extra_swing = if bar_pos.rem_euclid(4) == 2 {
        step_us * swing_extra / 100
    } else {
        0
    };
    Some(Event {
        bind: if open { HAT_OPEN } else { HAT_CLOSED },
        midi: 0,
        level,
        delay_us: micro(session, 5, step, signature.hat_delay_us, step_us) + extra_swing,
    })
}

/// Hash-lane ids keep each drum voice's humanization streams independent.
const LANE_KICK: u64 = 1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::catalog::AI_CATALOG;

    fn fixture() -> (Session, Params) {
        let session = Session::new(7, &AI_CATALOG);
        let params = base_params();
        (session, params)
    }

    fn base_params() -> Params {
        crate::music::Arrangement::at(7, &[1, 2, 3], 0).params
    }

    #[test]
    fn backbeat_is_inviolable() {
        let (session, mut params) = fixture();
        params.half_time = false;
        for bar in 0..32 {
            for &pos in &[4_i64, 12] {
                let step = bar * STEPS_PER_BAR + pos;
                let hit = snare_at(&session, &params, step, 190_000).unwrap();
                assert_eq!(hit.bind, SNARE_MAIN);
                assert!(hit.level > 0.3);
            }
        }
    }

    #[test]
    fn ghosts_never_replace_the_backbeat() {
        let (session, mut params) = fixture();
        params.ghosts = true;
        for step in 0..256 {
            if let Some(hit) = snare_at(&session, &params, step, 190_000) {
                let pos = step.rem_euclid(STEPS_PER_BAR);
                if hit.bind == SNARE_GHOST {
                    assert!(!matches!(pos, 4 | 12));
                    assert!(hit.level < 0.3 * session.signature.snare_gain + 0.2);
                }
            }
        }
    }

    #[test]
    fn half_time_thins_every_drum_lane() {
        let (session, mut params) = fixture();
        params.half_time = true;
        let mut kicks = 0;
        let mut snares = 0;
        let mut hats = 0;
        for step in 0..STEPS_PER_CYCLE {
            kicks += kick_at(&session, &params, step, 190_000).is_some() as u32;
            snares += snare_at(&session, &params, step, 190_000).is_some() as u32;
            hats += hat_at(&session, &params, step, 190_000).is_some() as u32;
        }
        assert_eq!(kicks, 4);
        assert_eq!(snares, 4);
        assert_eq!(hats, 16);
    }

    #[test]
    fn swing_delays_offbeats_only() {
        assert_eq!(swing_us(0, 18, 100_000), 0);
        assert_eq!(swing_us(4, 18, 100_000), 0);
        assert_eq!(swing_us(2, 18, 100_000), 18_000);
        assert_eq!(swing_us(3, 18, 100_000), 9_000);
    }

    #[test]
    fn drum_lanes_stay_deterministic() {
        let (session, params) = fixture();
        for step in 0..128 {
            assert_eq!(
                kick_at(&session, &params, step, 190_000),
                kick_at(&session, &params, step, 190_000)
            );
        }
    }
}
