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

    /// Does device `index` of `size` play this role? Roles are dealt round-robin
    /// so role j goes to device `j % size`; a lone box gets everything.
    pub fn assigned_to(self, index: usize, size: usize) -> bool {
        let size = size.max(1);
        let j = ROLES.iter().position(|r| *r == self).unwrap_or(0);
        j % size == index % size
    }

    /// The headline role for a device (its lowest-index assignment).
    pub fn primary(index: usize, size: usize) -> Role {
        ROLES
            .iter()
            .copied()
            .find(|r| r.assigned_to(index, size))
            .unwrap_or(Role::Pulse)
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

const CATALOG: [Feature; 22] = [
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
    Feature::KeyStabs,
    Feature::SparseKeys,
    Feature::LeadIn,
    Feature::BusyLead,
    Feature::NewMotif,
    Feature::Dusty,
    Feature::PadPulse,
    Feature::Reharm,
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
            Feature::SwingHard => p.swing_extra = 18,
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

    fn code(self) -> u32 {
        CATALOG.iter().position(|f| *f == self).unwrap_or(0) as u32 + 1
    }
}

/// Bars per arrangement phrase (one turn).
pub const BARS_PER_PHRASE: i64 = 8;
const WINDOW: usize = 4;

/// The resolved arrangement at a given phrase.
#[derive(Clone, Copy, Debug)]
pub struct Arrangement {
    pub params: Params,
    pub selector: NodeId,
    pub incoming: Feature,
    fingerprint: u32,
}

impl Arrangement {
    pub fn at(seed: u64, roster: &[NodeId], phrase: i64) -> Self {
        let mut params = Params::base(seed);
        let start = (phrase - WINDOW as i64 + 1).max(0);
        let mut fingerprint = (seed as u32) ^ (params.reharm);
        let mut selector = selector_for(roster, phrase);
        for p in start..=phrase {
            let sel = selector_for(roster, p);
            let feature = pick(seed, p, sel);
            feature.apply(&mut params);
            fingerprint = fingerprint
                .wrapping_mul(31)
                .wrapping_add(feature.code())
                .wrapping_add(sel as u32);
            for role in ROLES {
                let lane = pick_role_lane(seed, p, selector_for_role(roster, role), role);
                lane.apply(&mut params);
                fingerprint = fingerprint
                    .wrapping_mul(17)
                    .wrapping_add(lane.code())
                    .wrapping_add(role.code());
            }
            if p == phrase {
                selector = sel;
            }
        }
        let incoming = pick(seed, phrase + 1, selector_for(roster, phrase + 1));
        fingerprint ^= params.reharm.wrapping_mul(2654435761);
        Self {
            params,
            selector,
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

impl Role {
    fn code(self) -> u32 {
        match self {
            Role::Pulse => 1,
            Role::Pocket => 2,
            Role::Low => 3,
            Role::Color => 4,
            Role::Motif => 5,
        }
    }
}

fn selector_for(roster: &[NodeId], phrase: i64) -> NodeId {
    if roster.is_empty() {
        return 0;
    }
    roster[phrase.rem_euclid(roster.len() as i64) as usize]
}

fn selector_for_role(roster: &[NodeId], role: Role) -> NodeId {
    if roster.is_empty() {
        return 0;
    }
    let role_ix = ROLES.iter().position(|r| *r == role).unwrap_or(0);
    roster[role_ix % roster.len()]
}

fn pick(seed: u64, phrase: i64, selector: NodeId) -> Feature {
    let h = splitmix(
        seed ^ (phrase as u64).wrapping_mul(0x9e37_79b9) ^ selector.wrapping_mul(0x85eb_ca6b),
    );
    CATALOG[(h as usize) % CATALOG.len()]
}

fn pick_role_lane(seed: u64, phrase: i64, selector: NodeId, role: Role) -> Feature {
    let role_catalog: &[Feature] = match role {
        Role::Pulse => &[
            Feature::KickA,
            Feature::KickB,
            Feature::HalfTime,
            Feature::SubBass,
        ],
        Role::Pocket => &[
            Feature::DoubleHats,
            Feature::SparseHats,
            Feature::OpenHats,
            Feature::Ghosts,
            Feature::DrumFill,
            Feature::SwingHard,
        ],
        Role::Low => &[
            Feature::Walk,
            Feature::BassSkip,
            Feature::SubBass,
            Feature::BusyBass,
        ],
        Role::Color => &[
            Feature::RichChords,
            Feature::KeyStabs,
            Feature::SparseKeys,
            Feature::Dusty,
            Feature::PadPulse,
            Feature::Reharm,
        ],
        Role::Motif => &[Feature::LeadIn, Feature::BusyLead, Feature::NewMotif],
    };
    let h = splitmix(
        seed ^ (phrase as u64).wrapping_mul(0x517c_c1b7)
            ^ selector.wrapping_mul(0x27d4_eb2d)
            ^ (role.code() as u64).wrapping_mul(0x1656_67b1),
    );
    role_catalog[(h as usize) % role_catalog.len()]
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
        // Four boxes: every role covered, drums + texture on box 0.
        for role in ROLES {
            let covered = (0..4).any(|i| role.assigned_to(i, 4));
            assert!(covered, "{:?} uncovered with 4 boxes", role);
        }
        // A lone box plays everything.
        for role in ROLES {
            assert!(role.assigned_to(0, 1));
        }
    }

    #[test]
    fn arrangement_is_deterministic_and_evolves() {
        let roster = [1u64, 2, 3];
        let a = Arrangement::at(99, &roster, 4);
        let b = Arrangement::at(99, &roster, 4);
        assert_eq!(a.params, b.params);
        // A later phrase generally differs.
        let c = Arrangement::at(99, &roster, 9);
        assert_ne!(a.codename(), c.codename());
    }

    #[test]
    fn codename_is_pronounceable_and_stable() {
        let n = Codename::coin(12345);
        assert_eq!(n, Codename::coin(12345));
        let s = n.as_str();
        assert!(s.len() >= 4 && s.is_ascii());
    }
}
