//! The distributed generative arrangement.
//!
//! Boxes take turns. Every phrase (~8 bars) the next device in the mesh roster
//! is the *selector* and deterministically picks one feature card; the active
//! arrangement is a sliding window of the most recent picks. Because the pick is
//! `hash(seed, phrase, selector_id)`, every box computes the identical
//! arrangement with no streaming, yet which box selects changes the outcome — so
//! boxes have distinct taste. The active combination hashes to a codename.

use crate::NodeId;

/// The jobs a box can take. A lone box plays them all; a swarm spreads them out.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Pulse,
    Pocket,
    Low,
    Color,
    Motif,
}

pub const ROLES: [Role; 5] = [
    Role::Pulse,
    Role::Pocket,
    Role::Low,
    Role::Color,
    Role::Motif,
];

impl Role {
    pub fn label(self) -> &'static str {
        match self {
            Role::Pulse => "PULSE",
            Role::Pocket => "POCKET",
            Role::Low => "LOW",
            Role::Color => "COLOR",
            Role::Motif => "MOTIF",
        }
    }

    const fn bit(self) -> u8 {
        1 << self as u8
    }

    /// Does device `index` of `size` play this role?
    pub fn assigned_to(self, index: usize, size: usize) -> bool {
        RolePlan::for_module(index, size).contains(self)
    }

    /// The headline role for a device.
    pub fn primary(index: usize, size: usize) -> Role {
        RolePlan::for_module(index, size).primary()
    }
}

/// The bounded local mix assigned to one module in the current roster.
///
/// One box renders the complete song. Two boxes split every lane. Larger groups
/// give every box one rhythm and one tonal lane, so an isolated box remains
/// musical while the group still has distinct responsibilities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RolePlan {
    mask: u8,
    primary: Role,
}

impl RolePlan {
    pub fn for_module(index: usize, size: usize) -> Self {
        let size = size.max(1);
        let index = index % size;
        if size == 1 {
            return Self {
                mask: (1 << ROLES.len()) - 1,
                primary: Role::Pulse,
            };
        }

        let rhythm = if index % 2 == 0 {
            Role::Pulse
        } else {
            Role::Pocket
        };
        let tonal = match index % 3 {
            0 => Role::Low,
            1 => Role::Color,
            _ => Role::Motif,
        };
        let mut mask = rhythm.bit() | tonal.bit();
        if size == 2 && index == 0 {
            mask |= Role::Motif.bit();
        }

        Self {
            mask,
            primary: rhythm,
        }
    }

    pub const fn contains(self, role: Role) -> bool {
        self.mask & role.bit() != 0
    }

    pub const fn mask(self) -> u8 {
        self.mask
    }

    pub const fn primary(self) -> Role {
        self.primary
    }

    /// Fixed makeup gain for this local subset. Sparse drum/tonal pairings need
    /// more drive than kick/bass pairings to carry a room at the same volume.
    /// The engine applies this before its bounded soft saturator.
    pub const fn output_trim(self) -> f32 {
        if self.contains(Role::Pulse) && self.contains(Role::Pocket) {
            1.2
        } else if self.contains(Role::Pulse) {
            if self.contains(Role::Motif)
                && !self.contains(Role::Low)
                && !self.contains(Role::Color)
            {
                1.8
            } else {
                1.4
            }
        } else if self.contains(Role::Low) {
            2.0
        } else if self.contains(Role::Motif) {
            7.0
        } else {
            5.5
        }
    }
}

/// Generator parameters the composer reads. Features nudge these; the generators
/// clamp and derive all pitch from the shared scale, so any combination is safe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Params {
    pub kick_variant: u8,
    pub drum_fill: u8,
    pub hat_density: u8, // 0 sparse, 1 normal, 2 double
    pub open_hats: bool,
    pub ghosts: bool,
    pub half_time: bool,
    pub swing_extra: u8, // added swing percent
    pub bass_busy: bool,
    pub bass_walk: bool,
    pub bass_shape: u8,
    pub sub_bass: bool,
    pub rich_chords: bool,
    pub keys_sparse: bool,
    pub keys_shape: u8,
    pub lead_on: bool,
    pub lead_busy: bool,
    pub lead_shape: u8,
    pub dust: u8, // 0..3
    pub texture_shape: u8,
    pub reharm: u32, // progression seed offset
}

impl Params {
    fn base(seed: u64) -> Self {
        Self {
            kick_variant: 0,
            drum_fill: 0,
            hat_density: 1,
            open_hats: false,
            ghosts: false,
            half_time: false,
            swing_extra: (seed % 6) as u8,
            bass_busy: false,
            bass_walk: false,
            bass_shape: (seed as u8) & 1,
            sub_bass: false,
            rich_chords: false,
            keys_sparse: false,
            keys_shape: ((seed >> 3) as u8) & 1,
            lead_on: true,
            lead_busy: false,
            lead_shape: ((seed >> 5) as u8) % 4,
            dust: 1,
            texture_shape: ((seed >> 7) as u8) & 1,
            reharm: 0,
        }
    }
}

/// One composable variation. Each is a pure, always-safe delta on `Params`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Feature {
    DoubleHats,
    SparseHats,
    OpenHats,
    Ghosts,
    KickA,
    KickB,
    HalfTime,
    DrumFill,
    SwingHard,
    Walk,
    BassSkip,
    SubBass,
    BusyBass,
    RichChords,
    KeyStabs,
    SparseKeys,
    LeadIn,
    BusyLead,
    NewMotif,
    Dusty,
    PadPulse,
    Reharm,
}

// Only advertise variations that the sample-loop renderer currently makes
// audible. Keeping dormant synthesis-era parameters out of rotation prevents
// a phrase boundary from promising a change that cannot be heard.
const CATALOG: [Feature; 17] = [
    Feature::DoubleHats,
    Feature::SparseHats,
    Feature::OpenHats,
    Feature::Ghosts,
    Feature::KickA,
    Feature::KickB,
    Feature::HalfTime,
    Feature::DrumFill,
    Feature::SwingHard,
    Feature::Walk,
    Feature::BassSkip,
    Feature::SubBass,
    Feature::BusyBass,
    Feature::RichChords,
    Feature::SparseKeys,
    Feature::BusyLead,
    Feature::PadPulse,
];

impl Feature {
    fn apply(self, p: &mut Params) {
        match self {
            Feature::DoubleHats => p.hat_density = 2,
            Feature::SparseHats => p.hat_density = 0,
            Feature::OpenHats => p.open_hats = true,
            Feature::Ghosts => p.ghosts = true,
            Feature::KickA => p.kick_variant = 1,
            Feature::KickB => p.kick_variant = 2,
            Feature::HalfTime => p.half_time = true,
            Feature::DrumFill => p.drum_fill = p.drum_fill.wrapping_add(1),
            Feature::SwingHard => p.swing_extra = 10,
            Feature::Walk => {
                p.bass_walk = true;
                p.bass_busy = true;
            }
            Feature::BassSkip => p.bass_shape = p.bass_shape.wrapping_add(1),
            Feature::SubBass => p.sub_bass = true,
            Feature::BusyBass => p.bass_busy = true,
            Feature::RichChords => p.rich_chords = true,
            Feature::KeyStabs => p.keys_shape = p.keys_shape.wrapping_add(1),
            Feature::SparseKeys => p.keys_sparse = true,
            Feature::LeadIn => p.lead_on = true,
            Feature::BusyLead => {
                p.lead_on = true;
                p.lead_busy = true;
            }
            Feature::NewMotif => {
                p.lead_on = true;
                p.lead_shape = p.lead_shape.wrapping_add(1);
            }
            Feature::Dusty => p.dust = 3,
            Feature::PadPulse => p.texture_shape = p.texture_shape.wrapping_add(1),
            Feature::Reharm => p.reharm = p.reharm.wrapping_add(1),
        }
    }

    pub fn role(self) -> Role {
        match self {
            Feature::KickA | Feature::KickB | Feature::HalfTime | Feature::SubBass => Role::Pulse,
            Feature::DoubleHats
            | Feature::SparseHats
            | Feature::OpenHats
            | Feature::Ghosts
            | Feature::DrumFill
            | Feature::SwingHard => Role::Pocket,
            Feature::Walk | Feature::BassSkip | Feature::BusyBass => Role::Low,
            Feature::RichChords
            | Feature::KeyStabs
            | Feature::SparseKeys
            | Feature::Dusty
            | Feature::PadPulse
            | Feature::Reharm => Role::Color,
            Feature::LeadIn | Feature::BusyLead | Feature::NewMotif => Role::Motif,
        }
    }

    /// Stable ABI code used by the hardware display and browser telemetry.
    pub const fn code(self) -> u8 {
        self as u8 + 1
    }
}

/// Bars per arrangement phrase (one turn).
pub const BARS_PER_PHRASE: i64 = 8;
// Two phrase cards can overlap. This keeps the mesh's collaborative selection
// audible without allowing every instrument lane to become busy at once.
const WINDOW: usize = 2;

/// The resolved arrangement at a given phrase.
#[derive(Clone, Copy, Debug)]
pub struct Arrangement {
    pub params: Params,
    pub selector: NodeId,
    /// The only lane allowed to add a foreground flourish this phrase.
    pub spotlight: Role,
    pub incoming: Feature,
    fingerprint: u32,
}

impl Arrangement {
    pub fn at(seed: u64, roster: &[NodeId], phrase: i64) -> Self {
        let mut params = Params::base(seed);
        let start = (phrase - WINDOW as i64 + 1).max(0);
        let mut fingerprint = (seed as u32) ^ (params.reharm);
        let mut selector = selector_for(roster, phrase);
        let mut spotlight = pick(seed, phrase.max(0), selector).role();
        for p in start..=phrase {
            let sel = selector_for(roster, p);
            let feature = pick(seed, p, sel);
            feature.apply(&mut params);
            fingerprint = fingerprint
                .wrapping_mul(31)
                .wrapping_add(feature.code() as u32)
                .wrapping_add(sel as u32);
            if p == phrase {
                selector = sel;
                spotlight = feature.role();
            }
        }
        let incoming = pick(seed, phrase + 1, selector_for(roster, phrase + 1));
        fingerprint ^= params.reharm.wrapping_mul(2654435761);
        Self {
            params,
            selector,
            spotlight,
            incoming,
            fingerprint,
        }
    }

    /// Codename for the next phrase, so screens can preview what's coming.
    pub fn next_codename(seed: u64, roster: &[NodeId], phrase: i64) -> Codename {
        Arrangement::at(seed, roster, phrase + 1).codename()
    }

    /// A coined, non-descriptive name that hashes the active combination.
    pub fn codename(&self) -> Codename {
        Codename::coin(self.fingerprint)
    }
}

fn selector_for(roster: &[NodeId], phrase: i64) -> NodeId {
    if roster.is_empty() {
        return 0;
    }
    roster[phrase.rem_euclid(roster.len() as i64) as usize]
}

fn pick(seed: u64, phrase: i64, selector: NodeId) -> Feature {
    let h = splitmix(
        seed ^ (phrase as u64).wrapping_mul(0x9e37_79b9) ^ selector.wrapping_mul(0x85eb_ca6b),
    );
    CATALOG[(h % CATALOG.len() as u64) as usize]
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Shared 64-bit avalanche mixer, reused by the kit selector so all deterministic
/// choices draw from the same hash family.
pub(crate) fn mix64(x: u64) -> u64 {
    splitmix(x)
}

/// A short pronounceable codename, e.g. "Toluma".
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Codename {
    bytes: [u8; 10],
    len: u8,
}

impl Codename {
    pub fn coin(hash: u32) -> Self {
        const CONS: &[u8; 16] = b"bdfgklmnprstvzjh";
        const VOWS: &[u8; 8] = b"aeiouyao";
        let mut bytes = [0u8; 10];
        let mut h = splitmix(hash as u64);
        let syllables = 2 + (h & 1) as usize; // 2 or 3
        let mut len = 0;
        for _ in 0..syllables {
            bytes[len] = CONS[(h & 15) as usize];
            h >>= 4;
            bytes[len + 1] = VOWS[(h & 7) as usize];
            h >>= 3;
            len += 2;
        }
        bytes[0] = bytes[0].to_ascii_uppercase();
        Self {
            bytes,
            len: len as u8,
        }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or("")
    }
}

impl Default for Codename {
    fn default() -> Self {
        Codename::coin(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_spread_across_devices() {
        assert_eq!(RolePlan::for_module(0, 1).mask().count_ones(), 5);

        for size in 2..=10 {
            let mut covered = 0;
            for index in 0..size {
                let plan = RolePlan::for_module(index, size);
                let count = plan.mask().count_ones();
                assert!(
                    (2..=3).contains(&count),
                    "{size} boxes, module {index}: {count} roles"
                );
                assert!(plan.contains(Role::Pulse) || plan.contains(Role::Pocket));
                assert!(
                    plan.contains(Role::Low)
                        || plan.contains(Role::Color)
                        || plan.contains(Role::Motif)
                );
                assert_ne!(count, ROLES.len() as u32);
                assert!((1.0..=7.0).contains(&plan.output_trim()));
                covered |= plan.mask();
            }
            assert_eq!(
                covered.count_ones(),
                5,
                "{size} boxes did not cover every role"
            );
        }

        assert_eq!(RolePlan::for_module(0, 2).mask().count_ones(), 3);
        assert_eq!(RolePlan::for_module(1, 2).mask().count_ones(), 2);
    }

    #[test]
    fn arrangement_is_deterministic_and_evolves() {
        let roster = [1u64, 2, 3];
        let a = Arrangement::at(99, &roster, 4);
        let b = Arrangement::at(99, &roster, 4);
        assert_eq!(a.params, b.params);
        assert_eq!(a.spotlight, b.spotlight);
        // A later phrase generally differs.
        let c = Arrangement::at(99, &roster, 9);
        assert_ne!(a.codename(), c.codename());
    }

    #[test]
    fn exactly_one_lane_owns_each_phrase_spotlight() {
        let roster = [1u64, 2, 3];
        for seed in 0..64 {
            for phrase in 0..64 {
                let arrangement = Arrangement::at(seed, &roster, phrase);
                let selector = selector_for(&roster, phrase);
                assert_eq!(arrangement.spotlight, pick(seed, phrase, selector).role());
            }
        }
    }

    #[test]
    fn selector_vectors_are_architecture_width_independent() {
        let roster = [1u64, 2, 3];
        let walk = Arrangement::at(0, &roster, 4);
        assert_eq!(walk.selector, 2);
        assert_eq!(walk.spotlight, Role::Low);
        assert!(walk.params.bass_walk);

        let sparse = Arrangement::at(2, &roster, 18);
        assert_eq!(sparse.spotlight, Role::Pocket);
        assert_eq!(sparse.params.hat_density, 0);
    }

    #[test]
    fn listening_profiles_resolve_to_their_named_spotlights() {
        let roster = [1u64, 2, 3];
        for (seed, phrase) in [(0, 10), (1, 12), (2, 21)] {
            let value = Arrangement::at(seed, &roster, phrase);
            assert_eq!(value.spotlight, Role::Pulse);
            assert!(value.params.half_time);
        }
        for (seed, phrase) in [(3, 8), (4, 58), (5, 6)] {
            let value = Arrangement::at(seed, &roster, phrase);
            assert_eq!(value.spotlight, Role::Pocket);
            assert_eq!(value.params.hat_density, 2);
        }
        for (seed, phrase) in [(6, 13), (7, 24), (8, 13)] {
            let value = Arrangement::at(seed, &roster, phrase);
            assert_eq!(value.spotlight, Role::Low);
            assert!(value.params.bass_walk);
        }
        for (seed, phrase) in [(12, 17), (10, 14), (11, 14)] {
            let value = Arrangement::at(seed, &roster, phrase);
            assert_eq!(value.spotlight, Role::Pocket);
            assert_eq!(value.params.hat_density, 0);
        }
    }

    #[test]
    fn arrangement_has_a_bounded_variation_budget() {
        let roster = [1u64, 2, 3, 4];
        for seed in 0..64 {
            for phrase in 0..64 {
                let actual = Arrangement::at(seed, &roster, phrase).params;
                let mut candidates = [Params::base(seed); CATALOG.len() * CATALOG.len()];
                let mut len = 0;
                for first in CATALOG {
                    for second in CATALOG {
                        let mut params = Params::base(seed);
                        first.apply(&mut params);
                        second.apply(&mut params);
                        candidates[len] = params;
                        len += 1;
                    }
                }
                assert!(
                    phrase == 0 || candidates[..len].contains(&actual),
                    "phrase {phrase} exceeded two active feature cards"
                );
            }
        }
    }

    #[test]
    fn every_selectable_feature_changes_rendered_parameters() {
        for feature in CATALOG {
            let before = Params::base(2);
            let mut after = before;
            feature.apply(&mut after);
            assert_ne!(before, after, "{feature:?} is not an audible variation");
        }
    }

    #[test]
    fn codename_is_pronounceable_and_stable() {
        let n = Codename::coin(12345);
        assert_eq!(n, Codename::coin(12345));
        let s = n.as_str();
        assert!(s.len() >= 4 && s.is_ascii());
    }
}
