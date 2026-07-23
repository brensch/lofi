//! Bind every pitch the session can emit to a concrete root-tagged one-shot.
//!
//! Resolution is a bounded metadata scan that runs once at start/seed-change.
//! After it, the render loop only indexes fixed tables: no catalogue search,
//! no allocation. Repitch ratios stay small because each lane's register is
//! chosen around the roots its cast source actually recorded.

use crate::music::catalog::{ElementKind, PackedCatalog, PackedElement};
use crate::music::score::session::Session;
use crate::music::theory::midi_to_hz;

/// Chord slots a progression can hold (mirrors `progression::MAX_CHORDS`).
pub const CHORD_SLOTS: usize = 4;
/// Shell voices per chord: third, seventh, extension.
pub const SHELL_VOICES: usize = 3;
/// Lead bind table span: scale degrees `-2..=9` around the tonic.
pub const LEAD_DEGREES: usize = 12;
/// Lowest bass target pitch; the table covers this MIDI note plus 11 above.
pub const BASS_FLOOR: u8 = 23;

/// One playable voice: a sampled element and its repitch ratio.
#[derive(Clone, Copy, Debug)]
pub struct Bind {
    pub element: PackedElement,
    pub rate: f32,
    pub midi: u8,
}

/// Every sampled voice a session needs, resolved to fixed tables.
#[derive(Clone, Copy, Debug)]
pub struct SymbolicScene {
    /// Playback ratio tuning the kick's fundamental to the session key.
    /// An untuned kick stamps its own pitch class over every chord.
    pub kick_rate: f32,
    pub kick_main: Option<PackedElement>,
    pub kick_accent: Option<PackedElement>,
    pub snare_main: Option<PackedElement>,
    pub snare_ghost: Option<PackedElement>,
    pub hat_closed: Option<PackedElement>,
    pub hat_open: Option<PackedElement>,
    /// Bass binds indexed by pitch class of the target note.
    pub bass: [Option<Bind>; 12],
    /// Voice-led shell voicings per progression chord slot.
    pub keys: [[Option<Bind>; SHELL_VOICES]; CHORD_SLOTS],
    /// Lead binds indexed by scale degree offset (`degree + 2`).
    pub lead: [Option<Bind>; LEAD_DEGREES],
}

impl SymbolicScene {
    pub fn resolve(catalog: &'static PackedCatalog, session: &Session) -> Self {
        let seed = session.seed;
        let kick_main = nth_by_energy(catalog, ElementKind::Kick, session.drum_source, 2, 4, seed);
        Self {
            kick_rate: kick_main
                .map(|element| kick_tuning(&element, session))
                .unwrap_or(1.0),
            kick_main,
            kick_accent: nth_by_energy(catalog, ElementKind::Kick, session.drum_source, 3, 4, seed),
            snare_main: nth_by_energy(catalog, ElementKind::Snare, session.drum_source, 2, 4, seed),
            snare_ghost: nth_by_energy(
                catalog,
                ElementKind::Snare,
                session.drum_source,
                0,
                4,
                seed,
            ),
            hat_closed: nth_by_energy(catalog, ElementKind::Hat, session.drum_source, 1, 4, seed),
            hat_open: longest(catalog, ElementKind::Hat, session.drum_source),
            bass: bass_table(catalog, session),
            keys: keys_table(catalog, session),
            lead: lead_table(catalog, session),
        }
    }

    /// Worst repitch distance in semitones across all tonal binds — a straight
    /// quality metric: big ratios sound chipmunked or muddy.
    pub fn worst_repitch_semitones(&self) -> f32 {
        let mut worst = 0.0f32;
        let mut consider = |bind: &Option<Bind>| {
            if let Some(bind) = bind {
                if let Some(root) = bind.element.root_semitone {
                    let distance = (bind.midi as f32 - root as f32).abs();
                    if distance > worst {
                        worst = distance;
                    }
                }
            }
        };
        self.bass.iter().for_each(&mut consider);
        for slot in &self.keys {
            slot.iter().for_each(&mut consider);
        }
        self.lead.iter().for_each(&mut consider);
        worst
    }
}

fn bass_table(catalog: &'static PackedCatalog, session: &Session) -> [Option<Bind>; 12] {
    let mut table = [None; 12];
    let (low, high) = root_span(catalog, ElementKind::BassNote, session.bass_source);
    for (pc, entry) in table.iter_mut().enumerate() {
        // Place each pitch class inside `BASS_FLOOR..BASS_FLOOR+12` so targets
        // hug the sampled bass roots (B0..Bb1 in the shipped pack). When the
        // cast source recorded nothing near the upper targets, drop an octave:
        // a low fifth beats a chipmunked one.
        let offset = (pc as i32 - i32::from(BASS_FLOOR)).rem_euclid(12);
        let target = i32::from(BASS_FLOOR) + offset;
        let distance = |candidate: i32| (candidate - low).abs().min((candidate - high).abs());
        let in_span = |candidate: i32| candidate >= low && candidate <= high;
        let target = if in_span(target) || distance(target) <= distance(target - 12) + 1 {
            target
        } else {
            target - 12
        };
        *entry = bind_nearest(
            catalog,
            ElementKind::BassNote,
            session.bass_source,
            target.clamp(0, 127) as u8,
            session.seed ^ pc as u64,
        );
    }
    table
}

fn keys_table(
    catalog: &'static PackedCatalog,
    session: &Session,
) -> [[Option<Bind>; SHELL_VOICES]; CHORD_SLOTS] {
    let mut table = [[None; SHELL_VOICES]; CHORD_SLOTS];
    // The voicing register hugs the roots the cast source actually recorded,
    // so repitch ratios stay small no matter which pack ships.
    let (low, high) = root_span(catalog, ElementKind::KeysNote, session.keys_source);
    let floor = (low - 2).max(36);
    let ceiling = (high + 4).min(72).max(floor + 7);
    let center = (floor + ceiling) / 2;
    let mut previous: [i32; SHELL_VOICES] = [center - 3, center + 1, center + 4];
    for (slot_ix, slot) in table.iter_mut().enumerate() {
        let chord = session.progression.slot_for_bar(slot_ix as i64).chord;
        let root = chord.root as i32;
        let classes = [
            root + chord.quality.third(),
            root + chord.quality.seventh(),
            root + chord.quality.extension(),
        ];
        for (voice, entry) in slot.iter_mut().enumerate() {
            let target =
                place_near(classes[voice], previous[voice]).clamp(floor as u8, ceiling as u8);
            previous[voice] = target as i32;
            *entry = bind_nearest(
                catalog,
                ElementKind::KeysNote,
                session.keys_source,
                target,
                session.seed ^ ((slot_ix * SHELL_VOICES + voice) as u64) << 8,
            );
        }
    }
    table
}

fn lead_table(catalog: &'static PackedCatalog, session: &Session) -> [Option<Bind>; LEAD_DEGREES] {
    let mut table = [None; LEAD_DEGREES];
    for (index, entry) in table.iter_mut().enumerate() {
        let degree = index as i8 - 2;
        let mut midi = session.progression.scale_note(degree) as i32;
        // The pack's lead roots sit below the default lead register; fold the
        // line down toward them so repitch stays gentle.
        while midi > 64 {
            midi -= 12;
        }
        *entry = bind_nearest(
            catalog,
            ElementKind::LeadNote,
            session.lead_source,
            midi.clamp(0, 127) as u8,
            session.seed ^ (index as u64) << 16,
        );
    }
    table
}

/// The element of `kind` in `source` with root nearest `target`; falls back to
/// any source if the cast one has no roots at all. Seeded tie-break keeps
/// equal-distance choices varied between sessions but stable within one.
fn bind_nearest(
    catalog: &'static PackedCatalog,
    kind: ElementKind,
    source: u32,
    target: u8,
    seed: u64,
) -> Option<Bind> {
    let pick = |restrict: Option<u32>| -> Option<PackedElement> {
        let mut best: Option<PackedElement> = None;
        let mut best_cost = u64::MAX;
        for index in 0..catalog.len_for_kind(kind) {
            let Some(element) = catalog.choose(kind, index as u64) else {
                continue;
            };
            if restrict.is_some_and(|source| element.source_hash != source) {
                continue;
            }
            let Some(root) = element.root_semitone else {
                continue;
            };
            let distance = (root as i64 - target as i64).unsigned_abs();
            let cost = distance * 16 + super::event_hash(seed, index as u64, 0) % 16;
            if cost < best_cost {
                best = Some(element);
                best_cost = cost;
            }
        }
        best
    };
    let element = pick(Some(source)).or_else(|| pick(None))?;
    let root = element.root_semitone?;
    Some(Bind {
        element,
        rate: midi_to_hz(target) / midi_to_hz(root),
        midi: target,
    })
}

/// Element with rank `rank` of `of` by energy within a source (init-only scan).
fn nth_by_energy(
    catalog: &'static PackedCatalog,
    kind: ElementKind,
    source: u32,
    rank: usize,
    of: usize,
    seed: u64,
) -> Option<PackedElement> {
    let mut count = 0usize;
    for index in 0..catalog.len_for_kind(kind) {
        if catalog
            .choose(kind, index as u64)
            .is_some_and(|element| element.source_hash == source)
        {
            count += 1;
        }
    }
    if count == 0 {
        return catalog.choose(kind, seed);
    }
    let target_rank = (rank * (count - 1)) / of.max(1).saturating_sub(1).max(1);
    let mut best = None;
    for index in 0..catalog.len_for_kind(kind) {
        let Some(element) = catalog.choose(kind, index as u64) else {
            continue;
        };
        if element.source_hash != source {
            continue;
        }
        let mut below = 0usize;
        for other_ix in 0..catalog.len_for_kind(kind) {
            let Some(other) = catalog.choose(kind, other_ix as u64) else {
                continue;
            };
            if other.source_hash != source {
                continue;
            }
            if (other.energy, other_ix) < (element.energy, index) {
                below += 1;
            }
        }
        if below == target_rank.min(count - 1) {
            best = Some(element);
            break;
        }
    }
    best
}

/// Playback-rate ratio that moves the kick's measured fundamental to the
/// session's tonic pitch class, bounded to ±3 semitones. Runs once at scene
/// resolve: a bounded autocorrelation over the kick body, never in render.
fn kick_tuning(element: &PackedElement, session: &Session) -> f32 {
    use crate::music::sample::render_sample;
    let sample = element.sample;
    let rate = sample.sample_rate() as f32;
    let frames = sample.len().min((rate * 0.3) as usize);
    // Fundamental search band: 30..90 Hz, the usual kick territory.
    let (min_lag, max_lag) = ((rate / 90.0) as usize, (rate / 30.0) as usize);
    if frames < max_lag * 2 || min_lag == 0 {
        return 1.0;
    }
    let value = |index: usize| render_sample(&sample, index as f32 / rate);
    let mut best_lag = 0usize;
    let mut best_score = 0.0f32;
    let mut lag = min_lag;
    while lag <= max_lag {
        let mut product = 0.0f32;
        let mut energy = 0.0f32;
        let mut index = max_lag;
        while index < frames {
            let a = value(index);
            let b = value(index - lag);
            product += a * b;
            energy += a * a + b * b;
            index += 2; // decimate: pitch resolution is ample at half density
        }
        let score = if energy > 1e-6 {
            2.0 * product / energy
        } else {
            0.0
        };
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
        lag += 1;
    }
    if best_lag == 0 || best_score < 0.3 {
        return 1.0;
    }
    let measured_hz = rate / best_lag as f32;
    // Nearest octave placement of the session tonic to the measured pitch.
    let tonic_class = i32::from(session.progression.scale_note(0)).rem_euclid(12);
    let mut best_ratio = 1.0f32;
    let mut best_distance = f32::MAX;
    for octave in 0..4 {
        let midi = tonic_class + octave * 12 + 12;
        let target = crate::music::theory::midi_to_hz(midi.clamp(0, 127) as u8);
        let ratio = target / measured_hz;
        let distance = if ratio > 1.0 { ratio } else { 1.0 / ratio };
        if distance < best_distance {
            best_distance = distance;
            best_ratio = ratio;
        }
    }
    best_ratio.clamp(0.8409, 1.1892) // ±3 semitones
}

/// Lowest and highest tagged roots for a kind within a source.
fn root_span(catalog: &'static PackedCatalog, kind: ElementKind, source: u32) -> (i32, i32) {
    let mut low = i32::MAX;
    let mut high = i32::MIN;
    for index in 0..catalog.len_for_kind(kind) {
        let Some(element) = catalog.choose(kind, index as u64) else {
            continue;
        };
        if element.source_hash != source {
            continue;
        }
        if let Some(root) = element.root_semitone {
            low = low.min(i32::from(root));
            high = high.max(i32::from(root));
        }
    }
    if low > high {
        (48, 60)
    } else {
        (low, high)
    }
}

/// Longest element within a source: the closest thing to an "open" hat.
fn longest(
    catalog: &'static PackedCatalog,
    kind: ElementKind,
    source: u32,
) -> Option<PackedElement> {
    let mut best: Option<PackedElement> = None;
    for index in 0..catalog.len_for_kind(kind) {
        let Some(element) = catalog.choose(kind, index as u64) else {
            continue;
        };
        if element.source_hash != source {
            continue;
        }
        if best.is_none_or(|current| element.sample.len() > current.sample.len()) {
            best = Some(element);
        }
    }
    best
}

/// The MIDI note with pitch class of `pc` nearest `center`.
fn place_near(pc: i32, center: i32) -> u8 {
    let class = pc.rem_euclid(12);
    let k = (center - class + 6).div_euclid(12);
    (class + 12 * k).clamp(0, 127) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::catalog::AI_CATALOG;

    fn scene(seed: u64) -> SymbolicScene {
        let session = Session::new(seed, &AI_CATALOG);
        SymbolicScene::resolve(&AI_CATALOG, &session)
    }

    #[test]
    fn every_voice_is_bound() {
        for seed in 0..24 {
            let scene = scene(seed);
            assert!(scene.kick_main.is_some() && scene.kick_accent.is_some());
            assert!(scene.snare_main.is_some() && scene.snare_ghost.is_some());
            assert!(scene.hat_closed.is_some() && scene.hat_open.is_some());
            assert!(scene.bass.iter().all(Option::is_some));
            assert!(scene.lead.iter().all(Option::is_some));
            for slot in &scene.keys {
                assert!(slot.iter().all(Option::is_some));
            }
        }
    }

    #[test]
    fn repitch_ratios_stay_musical() {
        for seed in 0..24 {
            let worst = scene(seed).worst_repitch_semitones();
            assert!(worst <= 6.0, "seed {seed} repitches {worst} semitones");
        }
    }

    #[test]
    fn lanes_keep_one_instrument_voice() {
        for seed in 0..24 {
            let scene = scene(seed);
            let source = scene.bass[0].unwrap().element.source_hash;
            assert!(scene
                .bass
                .iter()
                .flatten()
                .all(|bind| bind.element.source_hash == source));
            let lead_source = scene.lead[0].unwrap().element.source_hash;
            assert!(scene
                .lead
                .iter()
                .flatten()
                .all(|bind| bind.element.source_hash == lead_source));
        }
    }

    #[test]
    fn shell_voices_lead_smoothly_between_chords() {
        for seed in 0..24 {
            let scene = scene(seed);
            for voice in 0..SHELL_VOICES {
                for slot in 1..CHORD_SLOTS {
                    let previous = scene.keys[slot - 1][voice].unwrap().midi as i32;
                    let current = scene.keys[slot][voice].unwrap().midi as i32;
                    assert!(
                        (previous - current).abs() <= 6,
                        "seed {seed} voice {voice} jumps {previous}->{current}"
                    );
                }
            }
        }
    }

    #[test]
    fn ghost_snare_is_no_louder_than_the_backbeat() {
        for seed in 0..8 {
            let scene = scene(seed);
            assert!(
                scene.snare_ghost.unwrap().energy <= scene.snare_main.unwrap().energy,
                "seed {seed} ghost snare out-hits the backbeat"
            );
        }
    }
}
