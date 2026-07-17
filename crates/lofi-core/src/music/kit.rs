//! Deterministic master-tone profiles for the sample catalogue.

/// The shared playback and mix character applied to harvested material.
#[derive(Clone, Copy, Debug)]
pub struct Tone {
    /// Slow tape wow depth as a fractional playback-rate change.
    pub wow: f32,
    /// Faster tape flutter depth as a fractional playback-rate change.
    pub flutter: f32,
    /// Master lowpass cutoff in Hz.
    pub cutoff_hz: f32,
    /// Master saturation amount.
    pub drive: f32,
    /// Vinyl noise-bed level.
    pub air: f32,
}

/// A production profile. All audible instruments come from `catalog.pack`.
#[derive(Clone, Copy, Debug)]
pub struct Kit {
    pub name: &'static str,
    pub tone: Tone,
}

pub const KIT_DUSTY: Kit = Kit {
    name: "DUSTY",
    tone: Tone {
        wow: 0.0025,
        flutter: 0.0006,
        cutoff_hz: 6_200.0,
        drive: 0.15,
        air: 0.35,
    },
};

pub const KIT_RAINY: Kit = Kit {
    name: "RAINY",
    tone: Tone {
        wow: 0.004,
        flutter: 0.001,
        cutoff_hz: 5_200.0,
        drive: 0.12,
        air: 0.4,
    },
};

pub const KIT_NEON: Kit = Kit {
    name: "NEON",
    tone: Tone {
        wow: 0.0018,
        flutter: 0.0009,
        cutoff_hz: 7_800.0,
        drive: 0.1,
        air: 0.15,
    },
};

pub const KIT_VELVET: Kit = Kit {
    name: "VELVET",
    tone: Tone {
        wow: 0.0035,
        flutter: 0.0012,
        cutoff_hz: 6_000.0,
        drive: 0.18,
        air: 0.25,
    },
};

pub const KITS: &[&Kit] = &[&KIT_DUSTY, &KIT_RAINY, &KIT_NEON, &KIT_VELVET];

/// Pick a profile from shared state so every mesh member makes the same choice.
pub fn kit_for(seed: u64) -> &'static Kit {
    let hash = super::arrangement::mix64(seed ^ 0x7669_6265_6b69_7401);
    KITS[(hash % KITS.len() as u64) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kit_is_reachable_and_stable() {
        let mut seen = [false; KITS.len()];
        for seed in 0..500_u64 {
            let kit = kit_for(seed);
            assert_eq!(kit.name, kit_for(seed).name);
            let index = KITS
                .iter()
                .position(|candidate| candidate.name == kit.name)
                .unwrap();
            seen[index] = true;
        }
        assert!(seen.iter().all(|selected| *selected));
    }
}
