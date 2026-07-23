//! Symbolic bass, keys, and lead events.
//!
//! Pitch always derives from the session's progression and scale: the bass
//! walks roots, fifths, and diatonic approach tones; the keys comp shell
//! voicings with anticipation pushes; the lead states a motif and answers it.
//! Rests are first-class — most steps intentionally return nothing.

use crate::music::arrangement::Params;
use crate::music::content::motif_for;
use crate::music::score::drums::swing_us;
use crate::music::score::session::Session;
use crate::music::score::{Event, STEPS_PER_BAR, STEPS_PER_CYCLE};
use crate::music::theory::snap_to_scale;

const STEPS_PER_PHRASE: i64 = STEPS_PER_BAR * 8;
const LANE_BASS: u64 = 6;
const LANE_KEYS: u64 = 7;
const LANE_LEAD: u64 = 8;
/// Broken-chord roll spacing between keys voices, in microseconds.
const ROLL_US: i64 = 33_000;

fn micro(session: &Session, lane: u64, step: i64, step_us: i64) -> i64 {
    let signature = session.signature;
    signature.tonal_delay_us
        + swing_us(
            step.rem_euclid(STEPS_PER_BAR),
            i64::from(signature.swing_percent),
            step_us,
        )
        + super::humanize_us(session.seed, lane, step, signature.humanize_us)
}

/// The pitch class the bass targets at a grid hit, chosen from chord tones.
///
/// The choice hashes the position *within the four-bar cycle*, not the
/// absolute bar: a bassline is a riff with an identity that repeats, and only
/// the arrangement's `bass_shape` feature re-rolls it. Hashing absolute time
/// here measurably destroyed the render's repetition structure.
fn bass_pitch_class(session: &Session, params: &Params, step: i64) -> u8 {
    let bar = step.div_euclid(STEPS_PER_BAR);
    let bar_pos = step.rem_euclid(STEPS_PER_BAR);
    let cycle_pos = step.rem_euclid(STEPS_PER_CYCLE);
    let chord = session.progression.slot_for_bar(bar).chord;
    let root = i32::from(chord.root);
    if bar_pos == 0 {
        return root.rem_euclid(12) as u8;
    }
    let riff = u64::from(params.bass_shape) << 32;
    let h = super::event_hash(session.seed ^ riff, LANE_BASS ^ 0x5e1ec7, cycle_pos);
    let choice = h % if params.bass_busy { 5 } else { 4 };
    let interval = match choice {
        0 | 1 => 0,                 // restate the root
        2 => chord.quality.fifth(), // the fifth
        3 => 0,                     // octave feel comes from velocity
        _ => chord.quality.third(), // busy bass may touch the third
    };
    (root + interval).rem_euclid(12) as u8
}

/// Diatonic approach tone into the next bar's root.
fn approach_pitch_class(session: &Session, step: i64) -> u8 {
    let next_bar = step.div_euclid(STEPS_PER_BAR) + 1;
    let next_root = i32::from(session.progression.slot_for_bar(next_bar).chord.root);
    let key_root = i32::from(session.progression.scale_note(0));
    let below = snap_to_scale(next_root - 1, key_root, &scale_intervals(session));
    let above = snap_to_scale(next_root + 2, key_root, &scale_intervals(session));
    let cycle_bar = next_bar.rem_euclid(STEPS_PER_CYCLE / STEPS_PER_BAR);
    let h = super::event_hash(session.seed, LANE_BASS ^ 0xa99, cycle_bar);
    let chosen = if h.is_multiple_of(3) { above } else { below };
    chosen.rem_euclid(12) as u8
}

/// The session scale as semitone intervals, recovered from the progression.
fn scale_intervals(session: &Session) -> [i32; 7] {
    let tonic = i32::from(session.progression.scale_note(0));
    core::array::from_fn(|degree| {
        (i32::from(session.progression.scale_note(degree as i8)) - tonic).rem_euclid(12)
    })
}

/// Bass event at an absolute step. `Event::bind` is the target pitch class.
pub fn bass_at(session: &Session, params: &Params, step: i64, step_us: i64) -> Option<Event> {
    let bar_pos = step.rem_euclid(STEPS_PER_BAR);
    let cycle = step.rem_euclid(STEPS_PER_CYCLE);
    let signature = session.signature;

    if params.half_time {
        if bar_pos != 0 {
            return None;
        }
    } else if params.bass_walk && step.rem_euclid(STEPS_PER_PHRASE) >= STEPS_PER_PHRASE - 8 {
        // Spotlight walk: quarter-note scale walk through the final bar.
        if !matches!(bar_pos, 8 | 10 | 12 | 14) {
            return None;
        }
        let walk_ix = ((bar_pos - 8) / 2) as usize;
        let intervals = scale_intervals(session);
        let degrees = [0usize, 2, 4, 5];
        let root = i32::from(
            session
                .progression
                .slot_for_bar(step.div_euclid(STEPS_PER_BAR))
                .chord
                .root,
        );
        let pc = (root + intervals[degrees[walk_ix]]).rem_euclid(12) as u8;
        return Some(Event {
            bind: pc,
            midi: 0,
            level: 0.78 * signature.bass_gain,
            delay_us: micro(session, LANE_BASS, step, step_us),
        });
    }

    let approach = signature.bass_approach.active(cycle) && !params.half_time;
    let main = signature.bass.active(cycle) || bar_pos == 0;
    let busy_extra = params.bass_busy && bar_pos == 8 && !signature.bass.active(cycle);
    if !(main || approach || busy_extra) {
        return None;
    }

    let (pc, level) = if approach && !main {
        (approach_pitch_class(session, step), 0.62)
    } else if bar_pos == 0 {
        (bass_pitch_class(session, params, step), 0.95)
    } else {
        (bass_pitch_class(session, params, step), 0.74)
    };
    Some(Event {
        bind: pc,
        midi: 0,
        level: level * signature.bass_gain,
        delay_us: micro(session, LANE_BASS, step, step_us),
    })
}

/// Keys strike at an absolute step: up to three shell voices, rolled when the
/// comping shape is broken. `Event::bind` encodes `slot * SHELL_VOICES + voice`.
pub fn keys_at(
    session: &Session,
    params: &Params,
    step: i64,
    step_us: i64,
) -> [Option<Event>; super::scene::SHELL_VOICES] {
    const NONE: Option<Event> = None;
    let mut out = [NONE; super::scene::SHELL_VOICES];
    let bar = step.div_euclid(STEPS_PER_BAR);
    let bar_pos = step.rem_euclid(STEPS_PER_BAR);
    let cycle_bar = bar.rem_euclid(4);

    // The comping pattern: a downbeat statement, a softer answer inside the
    // bar (the motion real playing has), and an anticipation push into the
    // next chord. `first_voice` trims the answer to the upper structure.
    let (slot_bar, strike_level, first_voice) = if bar_pos == 0 {
        if params.keys_sparse && !matches!(cycle_bar, 0 | 2) {
            return out;
        }
        (bar, 0.66, 0)
    } else if bar_pos == 7 && !params.keys_sparse && matches!(cycle_bar, 0 | 2) {
        (bar, 0.4, 1)
    } else if bar_pos == 14 && !params.keys_sparse && matches!(cycle_bar, 1 | 3) {
        (bar + 1, 0.5, 0)
    } else {
        return out;
    };

    let progression_len = session.progression.len().max(1) as i64;
    let slot = slot_bar.rem_euclid(progression_len) as usize;
    // A rolled chord belongs on the downbeat statement only; answers and
    // anticipation pushes are tight rhythmic punches.
    let broken = params.keys_shape % 2 == 1 && bar_pos == 0;
    let voices = if params.rich_chords { 3 } else { 2 };
    let base_delay = micro(session, LANE_KEYS, step, step_us);
    for (voice, entry) in out
        .iter_mut()
        .enumerate()
        .take(voices)
        .skip(first_voice.min(voices.saturating_sub(1)))
    {
        let roll = if broken { ROLL_US * voice as i64 } else { 0 };
        let top = voice + 1 == voices;
        *entry = Some(Event {
            bind: (slot * super::scene::SHELL_VOICES + voice) as u8,
            midi: 0,
            level: strike_level + if top { 0.07 } else { 0.0 },
            delay_us: base_delay + roll,
        });
    }
    out
}

/// Lead event at an absolute step. `Event::bind` indexes the scene's degree
/// table (`degree + 2`). The motif transposes per phrase and thins at the arc
/// edges so the melody breathes instead of looping verbatim.
pub fn lead_at(session: &Session, params: &Params, step: i64, step_us: i64) -> Option<Event> {
    if !params.lead_on {
        return None;
    }
    let phrase = step.div_euclid(STEPS_PER_PHRASE);
    let arc = phrase.rem_euclid(4);
    let cycle = step.rem_euclid(STEPS_PER_CYCLE);
    let motif = motif_for(session.signature, params.lead_shape);
    let event = motif.event_at(cycle)?;

    // Rest the tail of quiet phrases: the motif states less, not more.
    if matches!(arc, 0 | 3) && !params.lead_busy && event.velocity < 52 {
        return None;
    }

    // Phrase-scale call and answer: even phrases state near home, odd phrases
    // answer shifted. Partitioning by parity guarantees adjacent phrases
    // always differ even when the arrangement cards happen to repeat.
    let options = if phrase.rem_euclid(2) == 0 {
        [0, 2]
    } else {
        [1, -1]
    };
    let transpose = options[(super::event_hash(session.seed, LANE_LEAD, phrase) % 2) as usize];
    let degree = i32::from(event.degree) + transpose;
    let bind = (degree + 2).clamp(0, super::scene::LEAD_DEGREES as i32 - 1) as u8;
    Some(Event {
        bind,
        midi: 0,
        level: f32::from(event.velocity) / 127.0 * 0.62,
        delay_us: micro(session, LANE_LEAD, step, step_us),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::catalog::AI_CATALOG;
    use crate::music::Arrangement;

    fn fixture(seed: u64) -> (Session, Params) {
        (
            Session::new(seed, &AI_CATALOG),
            Arrangement::at(seed, &[1, 2, 3], 1).params,
        )
    }

    #[test]
    fn bass_downbeat_is_always_the_chord_root() {
        for seed in 0..16 {
            let (session, params) = fixture(seed);
            for bar in 0..16 {
                let event = bass_at(&session, &params, bar * STEPS_PER_BAR, 190_000)
                    .expect("downbeat bass");
                let root = session
                    .progression
                    .slot_for_bar(bar)
                    .chord
                    .root
                    .rem_euclid(12);
                assert_eq!(event.bind, root, "seed {seed} bar {bar}");
            }
        }
    }

    #[test]
    fn approach_tones_are_diatonic() {
        for seed in 0..16 {
            let (session, params) = fixture(seed);
            let intervals = scale_intervals(&session);
            let tonic = i32::from(session.progression.scale_note(0)).rem_euclid(12);
            for step in 0..256_i64 {
                let cycle = step.rem_euclid(STEPS_PER_CYCLE);
                if session.signature.bass_approach.active(cycle)
                    && !session.signature.bass.active(cycle)
                {
                    if let Some(event) = bass_at(&session, &params, step, 190_000) {
                        let rel = (i32::from(event.bind) - tonic).rem_euclid(12);
                        assert!(
                            intervals.contains(&rel),
                            "seed {seed} step {step}: approach pc {} not in scale",
                            event.bind
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn keys_leave_space() {
        for seed in 0..8 {
            let (session, params) = fixture(seed);
            let mut strikes = 0;
            for step in 0..STEPS_PER_CYCLE {
                if keys_at(&session, &params, step, 190_000)[0].is_some() {
                    strikes += 1;
                }
            }
            assert!(
                (2..=8).contains(&strikes),
                "seed {seed}: {strikes} keys strikes per four bars"
            );
        }
    }

    #[test]
    fn anticipation_pushes_voice_the_next_chord() {
        for seed in 0..8 {
            let (session, mut params) = fixture(seed);
            params.keys_sparse = false;
            let step = STEPS_PER_BAR + 14; // bar 1, and-of-four
            let voices = keys_at(&session, &params, step, 190_000);
            if let Some(event) = voices[0] {
                let expected_slot = 2_i64.rem_euclid(session.progression.len().max(1) as i64) as u8;
                assert_eq!(event.bind / 3, expected_slot);
            }
        }
    }

    #[test]
    fn lead_rests_more_in_quiet_phrases() {
        for seed in 0..8 {
            let (session, mut params) = fixture(seed);
            params.lead_on = true;
            params.lead_busy = false;
            let mut peak = 0;
            let mut quiet = 0;
            for step in 0..STEPS_PER_PHRASE {
                peak += lead_at(&session, &params, STEPS_PER_PHRASE * 2 + step, 190_000).is_some()
                    as u32;
                quiet += lead_at(&session, &params, step, 190_000).is_some() as u32;
            }
            assert!(quiet <= peak, "seed {seed}: quiet phrase busier than peak");
        }
    }

    #[test]
    fn tonal_lanes_are_deterministic() {
        let (session, params) = fixture(3);
        for step in 0..128 {
            assert_eq!(
                bass_at(&session, &params, step, 190_000),
                bass_at(&session, &params, step, 190_000)
            );
            assert_eq!(
                lead_at(&session, &params, step, 190_000),
                lead_at(&session, &params, step, 190_000)
            );
        }
    }
}
