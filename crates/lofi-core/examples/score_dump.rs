//! Dump the symbolic score as JSONL: the exact events the audio path renders,
//! in a form offline tooling can property-test and diff without listening.
//!
//! ```sh
//! cargo run -p lofi-core --example score_dump -- 7 4 78000 > score.jsonl
//! ```
//!
//! Arguments: seed, phrase count, transport BPM in milli-BPM (all optional).

use lofi_core::music::score::scene::SHELL_VOICES;
use lofi_core::music::score::{drums, tonal, Session, SymbolicScene};
use lofi_core::music::{Arrangement, AI_CATALOG};

const STEPS_PER_BAR: i64 = 16;
const STEPS_PER_PHRASE: i64 = STEPS_PER_BAR * 8;

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args.next().and_then(|v| v.parse().ok()).unwrap_or(2);
    let phrases: i64 = args.next().and_then(|v| v.parse().ok()).unwrap_or(4);
    let bpm_milli: i64 = args.next().and_then(|v| v.parse().ok()).unwrap_or(78_000);

    let session = Session::new(seed, &AI_CATALOG);
    let scene = SymbolicScene::resolve(&AI_CATALOG, &session);
    let roster = [1u64, 2, 3];
    let step_us = 60_000_000_000 / bpm_milli / 4;

    println!(
        "{{\"meta\":{{\"seed\":{seed},\"bpm_milli\":{bpm_milli},\"signature\":\"{}\",\
         \"worst_repitch\":{:.2},\"kick_rate\":{:.4},\"drum_source\":{},\"bass_source\":{},\
         \"keys_source\":{},\"lead_source\":{}}}}}",
        session.signature.name,
        scene.worst_repitch_semitones(),
        scene.kick_rate,
        session.drum_source,
        session.bass_source,
        session.keys_source,
        session.lead_source,
    );

    for step in 0..phrases * STEPS_PER_PHRASE {
        let phrase = step.div_euclid(STEPS_PER_PHRASE);
        let params = Arrangement::at(seed, &roster, phrase).params;
        let bar = step.div_euclid(STEPS_PER_BAR);
        let chord = session.progression.slot_for_bar(bar).chord;

        if let Some(event) = drums::kick_at(&session, &params, step, step_us) {
            row(
                "kick",
                step,
                phrase,
                u32::from(event.bind),
                event.level,
                event.delay_us,
                chord.root,
            );
        }
        if let Some(event) = drums::snare_at(&session, &params, step, step_us) {
            row(
                "snare",
                step,
                phrase,
                u32::from(event.bind),
                event.level,
                event.delay_us,
                chord.root,
            );
        }
        if let Some(event) = drums::hat_at(&session, &params, step, step_us) {
            row(
                "hat",
                step,
                phrase,
                u32::from(event.bind),
                event.level,
                event.delay_us,
                chord.root,
            );
        }
        if let Some(event) = tonal::bass_at(&session, &params, step, step_us) {
            let midi = scene.bass[event.bind as usize % 12]
                .map(|b| b.midi)
                .unwrap_or(0);
            pitched(
                "bass",
                step,
                phrase,
                midi,
                event.level,
                event.delay_us,
                chord.root,
            );
        }
        for event in tonal::keys_at(&session, &params, step, step_us)
            .iter()
            .flatten()
        {
            let slot = (event.bind as usize / SHELL_VOICES).min(3);
            let voice = event.bind as usize % SHELL_VOICES;
            let midi = scene.keys[slot][voice].map(|b| b.midi).unwrap_or(0);
            pitched(
                "keys",
                step,
                phrase,
                midi,
                event.level,
                event.delay_us,
                chord.root,
            );
        }
        if let Some(event) = tonal::lead_at(&session, &params, step, step_us) {
            let midi = scene.lead[event.bind as usize % scene.lead.len()]
                .map(|b| b.midi)
                .unwrap_or(0);
            pitched(
                "lead",
                step,
                phrase,
                midi,
                event.level,
                event.delay_us,
                chord.root,
            );
        }
    }
}

fn row(lane: &str, step: i64, phrase: i64, bind: u32, level: f32, delay_us: i64, chord_root: u8) {
    println!(
        "{{\"lane\":\"{lane}\",\"step\":{step},\"bar_pos\":{},\"phrase\":{phrase},\
         \"bind\":{bind},\"level\":{level:.3},\"delay_us\":{delay_us},\"chord_root\":{chord_root}}}",
        step.rem_euclid(STEPS_PER_BAR)
    );
}

fn pitched(
    lane: &str,
    step: i64,
    phrase: i64,
    midi: u8,
    level: f32,
    delay_us: i64,
    chord_root: u8,
) {
    println!(
        "{{\"lane\":\"{lane}\",\"step\":{step},\"bar_pos\":{},\"phrase\":{phrase},\
         \"midi\":{midi},\"level\":{level:.3},\"delay_us\":{delay_us},\"chord_root\":{chord_root}}}",
        step.rem_euclid(STEPS_PER_BAR)
    );
}
