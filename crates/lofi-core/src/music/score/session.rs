//! Seed → the session's musical identity: key, progression, groove signature,
//! and which sampled source performance voices each lane.
//!
//! Casting sources is a bounded scan over fixed catalogue metadata and runs
//! only when a device starts or changes seed — never in the render loop.

use crate::music::catalog::{ElementKind, PackedCatalog};
use crate::music::content::{signature_for, GrooveSignature};
use crate::music::progression::Progression;

/// The most distinct source performances a pack can cast from.
const MAX_SOURCES: usize = 8;

/// The session's symbolic identity, shared by every box in the mesh.
#[derive(Clone, Copy, Debug)]
pub struct Session {
    pub seed: u64,
    pub progression: Progression,
    pub signature: &'static GrooveSignature,
    /// Source performance supplying kick, snare, and hat one-shots.
    pub drum_source: u32,
    /// Source performance supplying bass one-shots.
    pub bass_source: u32,
    /// Source performance supplying keys one-shots.
    pub keys_source: u32,
    /// Source performance supplying lead one-shots.
    pub lead_source: u32,
}

impl Session {
    pub fn new(seed: u64, catalog: &'static PackedCatalog) -> Self {
        Self {
            seed,
            progression: Progression::generate(seed),
            signature: signature_for(seed),
            drum_source: cast_drum_source(catalog, seed),
            bass_source: cast_tonal_source(catalog, ElementKind::BassNote, seed ^ 0xba55),
            keys_source: cast_tonal_source(catalog, ElementKind::KeysNote, seed ^ 0x4b45),
            lead_source: cast_tonal_source(catalog, ElementKind::LeadNote, seed ^ 0x1ead),
        }
    }
}

/// Distinct source hashes for one element kind, with element counts.
fn sources_for(
    catalog: &'static PackedCatalog,
    kind: ElementKind,
) -> ([(u32, u32); MAX_SOURCES], usize) {
    let mut sources = [(0u32, 0u32); MAX_SOURCES];
    let mut len = 0;
    for index in 0..catalog.len_for_kind(kind) {
        let Some(element) = catalog.choose(kind, index as u64) else {
            continue;
        };
        if let Some(entry) = sources[..len]
            .iter_mut()
            .find(|entry| entry.0 == element.source_hash)
        {
            entry.1 += 1;
        } else if len < MAX_SOURCES {
            sources[len] = (element.source_hash, 1);
            len += 1;
        }
    }
    (sources, len)
}

/// Pick a drum family that has all three voices, seeded among candidates.
fn cast_drum_source(catalog: &'static PackedCatalog, seed: u64) -> u32 {
    let (kicks, kick_len) = sources_for(catalog, ElementKind::Kick);
    let mut complete = [0u32; MAX_SOURCES];
    let mut complete_len = 0;
    for &(source, _) in &kicks[..kick_len] {
        let has_snare = has_source(catalog, ElementKind::Snare, source);
        let has_hat = has_source(catalog, ElementKind::Hat, source);
        if has_snare && has_hat && complete_len < MAX_SOURCES {
            complete[complete_len] = source;
            complete_len += 1;
        }
    }
    if complete_len == 0 {
        return kicks.first().map(|entry| entry.0).unwrap_or(0);
    }
    complete[(super::event_hash(seed, 0xd505, 0) % complete_len as u64) as usize]
}

/// Pick the best-covered tonal source; seeded tie-break between equals.
/// Coverage weighs element count, so a lane keeps one consistent instrument
/// voice with the most root choices available for small repitch ratios.
fn cast_tonal_source(catalog: &'static PackedCatalog, kind: ElementKind, seed: u64) -> u32 {
    let (sources, len) = sources_for(catalog, kind);
    if len == 0 {
        return 0;
    }
    let mut best = sources[0];
    let mut best_score = 0u64;
    for &(source, count) in &sources[..len] {
        let score = (count as u64) * 16 + super::event_hash(seed, source as u64, 0) % 16;
        if score > best_score {
            best = (source, count);
            best_score = score;
        }
    }
    best.0
}

fn has_source(catalog: &'static PackedCatalog, kind: ElementKind, source: u32) -> bool {
    (0..catalog.len_for_kind(kind)).any(|index| {
        catalog
            .choose(kind, index as u64)
            .is_some_and(|element| element.source_hash == source)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::catalog::AI_CATALOG;

    #[test]
    fn sessions_are_deterministic_and_fully_cast() {
        for seed in 0..24 {
            let a = Session::new(seed, &AI_CATALOG);
            let b = Session::new(seed, &AI_CATALOG);
            assert_eq!(a.drum_source, b.drum_source);
            assert_eq!(a.bass_source, b.bass_source);
            assert_eq!(a.keys_source, b.keys_source);
            assert_eq!(a.lead_source, b.lead_source);
            assert_ne!(a.drum_source, 0, "seed {seed} has no drum family");
            assert_ne!(a.bass_source, 0, "seed {seed} has no bass source");
            assert_ne!(a.keys_source, 0, "seed {seed} has no keys source");
            assert_ne!(a.lead_source, 0, "seed {seed} has no lead source");
        }
    }

    #[test]
    fn drum_family_is_complete() {
        for seed in 0..24 {
            let session = Session::new(seed, &AI_CATALOG);
            for kind in [ElementKind::Kick, ElementKind::Snare, ElementKind::Hat] {
                assert!(
                    has_source(&AI_CATALOG, kind, session.drum_source),
                    "seed {seed} drum family missing {kind:?}"
                );
            }
        }
    }

    #[test]
    fn seeds_reach_more_than_one_drum_family() {
        let mut distinct = [0u32; 8];
        let mut len = 0;
        for seed in 0..64 {
            let source = Session::new(seed, &AI_CATALOG).drum_source;
            if !distinct[..len].contains(&source) && len < distinct.len() {
                distinct[len] = source;
                len += 1;
            }
        }
        assert!(len >= 2, "drum casting is stuck on one source");
    }
}
