//! The timbre catalogue and the vibe layer.
//!
//! Two kinds of content live here, both pure data:
//!
//! - **Instrument presets** — `const Patch` values (`RHODES_WARM`, `BASS_SUB`,
//!   `KICK_BOOMBAP`, …). Adding a new instrument to the world is adding a `const`
//!   and dropping it into [`ALL_PATCHES`]; the synth engine never changes.
//! - **Kits** — a `Kit` is a *vibe*: coherent pitched instruments, an embedded
//!   drum bank, and the tape/vinyl `Tone` that glues them together.
//!
//! Everything is selected deterministically from the shared seed, so every box in
//! the mesh renders the same vibe with the same instruments.

use super::patch::{AmpEnv, Fm, Lfo, Noiseband, Partial, Patch, PitchEnv};
use super::sample_bank::{DrumBank, ACOUSTIC_DRUMS};

// ---------------------------------------------------------------------------
// Electric pianos / keys — the defining lofi timbre.
// ---------------------------------------------------------------------------

/// Classic mellow Rhodes: soft tine bark that folds quickly into a warm sine,
/// slow tremolo, a hint of drive for body.
pub const RHODES_WARM: Patch = Patch {
    partials: &[
        Partial {
            ratio: 1.0,
            level: 1.0,
            decay: 1.1,
        },
        Partial {
            ratio: 2.0,
            level: 0.16,
            decay: 0.35,
        },
    ],
    fm: Some(Fm {
        ratio: 1.0,
        index: 1.5,
        index_decay: 0.13,
        floor: 0.16,
    }),
    tremolo: Lfo {
        rate_hz: 5.2,
        depth: 0.12,
        delay: 0.0,
    },
    amp: AmpEnv {
        attack: 0.004,
        release: 1.0,
        sustain: 0.0,
    },
    gain: 1.0,
    drive: 0.5,
    ..Patch::EMPTY
};

/// Brighter bell-Rhodes: more FM bite, a shimmering upper partial. Cuts through
/// busier arrangements.
pub const RHODES_BELL: Patch = Patch {
    partials: &[
        Partial {
            ratio: 1.0,
            level: 1.0,
            decay: 0.9,
        },
        Partial {
            ratio: 3.0,
            level: 0.12,
            decay: 0.4,
        },
    ],
    fm: Some(Fm {
        ratio: 2.0,
        index: 2.2,
        index_decay: 0.1,
        floor: 0.2,
    }),
    tremolo: Lfo {
        rate_hz: 6.0,
        depth: 0.08,
        delay: 0.0,
    },
    amp: AmpEnv {
        attack: 0.003,
        release: 0.9,
        sustain: 0.0,
    },
    gain: 0.95,
    drive: 0.35,
    ..Patch::EMPTY
};

/// Wurlitzer-ish reediness: barkier, shorter, a touch of grit.
pub const WURLI: Patch = Patch {
    partials: &[Partial {
        ratio: 1.0,
        level: 1.0,
        decay: 0.6,
    }],
    fm: Some(Fm {
        ratio: 1.0,
        index: 2.6,
        index_decay: 0.08,
        floor: 0.35,
    }),
    tremolo: Lfo {
        rate_hz: 4.6,
        depth: 0.16,
        delay: 0.0,
    },
    amp: AmpEnv {
        attack: 0.003,
        release: 0.6,
        sustain: 0.0,
    },
    gain: 0.95,
    drive: 0.8,
    ..Patch::EMPTY
};

// ---------------------------------------------------------------------------
// Bass.
// ---------------------------------------------------------------------------

/// Round upright/electric bass: fundamental plus a soft second harmonic.
pub const BASS_ROUND: Patch = Patch {
    partials: &[
        Partial {
            ratio: 1.0,
            level: 1.0,
            decay: 0.55,
        },
        Partial {
            ratio: 2.0,
            level: 0.18,
            decay: 0.35,
        },
    ],
    amp: AmpEnv {
        attack: 0.006,
        release: 0.55,
        sustain: 0.0,
    },
    gain: 1.0,
    drive: 0.3,
    ..Patch::EMPTY
};

/// Deep sub bass: almost pure fundamental, long, felt more than heard.
pub const BASS_SUB: Patch = Patch {
    partials: &[
        Partial {
            ratio: 1.0,
            level: 1.0,
            decay: 0.7,
        },
        Partial {
            ratio: 2.0,
            level: 0.08,
            decay: 0.3,
        },
    ],
    amp: AmpEnv {
        attack: 0.01,
        release: 0.7,
        sustain: 0.0,
    },
    gain: 1.05,
    drive: 0.2,
    ..Patch::EMPTY
};

/// Fingered bass with a little more bite and a faster note — sits forward.
pub const BASS_PLUCK: Patch = Patch {
    partials: &[
        Partial {
            ratio: 1.0,
            level: 1.0,
            decay: 0.4,
        },
        Partial {
            ratio: 2.0,
            level: 0.28,
            decay: 0.22,
        },
        Partial {
            ratio: 3.0,
            level: 0.1,
            decay: 0.15,
        },
    ],
    fm: Some(Fm {
        ratio: 1.0,
        index: 0.6,
        index_decay: 0.05,
        floor: 0.0,
    }),
    amp: AmpEnv {
        attack: 0.004,
        release: 0.45,
        sustain: 0.0,
    },
    gain: 0.95,
    drive: 0.5,
    ..Patch::EMPTY
};

// ---------------------------------------------------------------------------
// Leads / motifs.
// ---------------------------------------------------------------------------

/// Soft music-box: pure sine, gentle vibrato that fades in.
pub const LEAD_MUSICBOX: Patch = Patch {
    partials: &[
        Partial {
            ratio: 1.0,
            level: 1.0,
            decay: 0.5,
        },
        Partial {
            ratio: 2.0,
            level: 0.12,
            decay: 0.3,
        },
    ],
    vibrato: Lfo {
        rate_hz: 5.5,
        depth: 0.006,
        delay: 0.12,
    },
    amp: AmpEnv {
        attack: 0.01,
        release: 0.5,
        sustain: 0.0,
    },
    gain: 1.0,
    ..Patch::EMPTY
};

/// Breathy sine-flute: a whisper of noise on the attack, slow vibrato.
pub const LEAD_FLUTE: Patch = Patch {
    partials: &[Partial {
        ratio: 1.0,
        level: 1.0,
        decay: 0.9,
    }],
    noise: Some(Noiseband {
        level: 0.05,
        decay: 0.06,
        tilt: 1.0,
    }),
    vibrato: Lfo {
        rate_hz: 5.0,
        depth: 0.01,
        delay: 0.2,
    },
    amp: AmpEnv {
        attack: 0.03,
        release: 0.8,
        sustain: 0.2,
    },
    gain: 0.9,
    ..Patch::EMPTY
};

/// Muted-guitar pluck: brighter partials, quick decay, a little FM twang.
pub const LEAD_GUITAR: Patch = Patch {
    partials: &[
        Partial {
            ratio: 1.0,
            level: 1.0,
            decay: 0.35,
        },
        Partial {
            ratio: 2.0,
            level: 0.3,
            decay: 0.2,
        },
        Partial {
            ratio: 3.0,
            level: 0.14,
            decay: 0.12,
        },
    ],
    fm: Some(Fm {
        ratio: 1.0,
        index: 1.0,
        index_decay: 0.04,
        floor: 0.0,
    }),
    vibrato: Lfo {
        rate_hz: 6.0,
        depth: 0.004,
        delay: 0.08,
    },
    amp: AmpEnv {
        attack: 0.004,
        release: 0.35,
        sustain: 0.0,
    },
    gain: 0.85,
    drive: 0.4,
    ..Patch::EMPTY
};

// ---------------------------------------------------------------------------
// Pads / texture.
// ---------------------------------------------------------------------------

/// Warm analog pad: two slightly-detuned saws-of-sine, slow swell, long tail.
pub const PAD_WARM: Patch = Patch {
    partials: &[
        Partial {
            ratio: 1.0,
            level: 1.0,
            decay: 3.0,
        },
        Partial {
            ratio: 1.004,
            level: 1.0,
            decay: 3.0,
        },
        Partial {
            ratio: 2.0,
            level: 0.2,
            decay: 2.0,
        },
    ],
    amp: AmpEnv {
        attack: 0.25,
        release: 2.6,
        sustain: 0.0,
    },
    gain: 0.5,
    ..Patch::EMPTY
};

/// Glassy pad: brighter, wider detune, a shimmer up top.
pub const PAD_GLASS: Patch = Patch {
    partials: &[
        Partial {
            ratio: 1.0,
            level: 1.0,
            decay: 3.2,
        },
        Partial {
            ratio: 1.007,
            level: 0.9,
            decay: 3.2,
        },
        Partial {
            ratio: 3.0,
            level: 0.14,
            decay: 1.6,
        },
        Partial {
            ratio: 4.0,
            level: 0.08,
            decay: 1.2,
        },
    ],
    amp: AmpEnv {
        attack: 0.4,
        release: 3.0,
        sustain: 0.0,
    },
    gain: 0.42,
    ..Patch::EMPTY
};

// ---------------------------------------------------------------------------
// Drums. Drum "pitch" is passed at render time (see `Kit::kick_hz`/`snare_hz`).
// ---------------------------------------------------------------------------

/// Boom-bap kick: a tuned sine with a fast pitch drop into a low thud, plus a
/// short click for the attack.
pub const KICK_BOOMBAP: Patch = Patch {
    partials: &[Partial {
        ratio: 1.0,
        level: 1.0,
        decay: 0.14,
    }],
    pitch_env: Some(PitchEnv {
        amount: 1.6,
        decay: 0.03,
    }),
    noise: Some(Noiseband {
        level: 0.22,
        decay: 0.002,
        tilt: 1.0,
    }),
    amp: AmpEnv {
        attack: 0.0006,
        release: 0.14,
        sustain: 0.0,
    },
    gain: 1.0,
    drive: 0.4,
    ..Patch::EMPTY
};

/// Soft round kick: less click, longer body, deeper drop. Good under pads.
pub const KICK_SOFT: Patch = Patch {
    partials: &[Partial {
        ratio: 1.0,
        level: 1.0,
        decay: 0.2,
    }],
    pitch_env: Some(PitchEnv {
        amount: 1.2,
        decay: 0.045,
    }),
    noise: Some(Noiseband {
        level: 0.08,
        decay: 0.0015,
        tilt: 1.0,
    }),
    amp: AmpEnv {
        attack: 0.001,
        release: 0.2,
        sustain: 0.0,
    },
    gain: 1.0,
    drive: 0.25,
    ..Patch::EMPTY
};

/// Dusty snare: filtered noise burst over a short tonal body.
pub const SNARE_DUSTY: Patch = Patch {
    partials: &[Partial {
        ratio: 1.0,
        level: 0.4,
        decay: 0.09,
    }],
    noise: Some(Noiseband {
        level: 0.8,
        decay: 0.11,
        tilt: 0.5,
    }),
    amp: AmpEnv {
        attack: 0.001,
        release: 0.11,
        sustain: 0.0,
    },
    gain: 0.75,
    ..Patch::EMPTY
};

/// Tight rimshot-ish snare: brighter, shorter, less body.
pub const SNARE_RIM: Patch = Patch {
    partials: &[Partial {
        ratio: 1.0,
        level: 0.3,
        decay: 0.05,
    }],
    noise: Some(Noiseband {
        level: 0.7,
        decay: 0.06,
        tilt: 0.85,
    }),
    amp: AmpEnv {
        attack: 0.0006,
        release: 0.06,
        sustain: 0.0,
    },
    gain: 0.7,
    ..Patch::EMPTY
};

/// Closed hat: short, bright filtered noise.
pub const HAT_CLOSED: Patch = Patch {
    noise: Some(Noiseband {
        level: 0.5,
        decay: 0.028,
        tilt: 1.0,
    }),
    amp: AmpEnv {
        attack: 0.0005,
        release: 0.028,
        sustain: 0.0,
    },
    gain: 1.0,
    ..Patch::EMPTY
};

/// Looser hat: a bit longer and airier, for open-hat features.
pub const HAT_LOOSE: Patch = Patch {
    noise: Some(Noiseband {
        level: 0.5,
        decay: 0.06,
        tilt: 0.9,
    }),
    amp: AmpEnv {
        attack: 0.0006,
        release: 0.06,
        sustain: 0.0,
    },
    gain: 1.0,
    ..Patch::EMPTY
};

/// Every preset, for catalogue-wide property tests.
pub const ALL_PATCHES: &[&Patch] = &[
    &RHODES_WARM,
    &RHODES_BELL,
    &WURLI,
    &BASS_ROUND,
    &BASS_SUB,
    &BASS_PLUCK,
    &LEAD_MUSICBOX,
    &LEAD_FLUTE,
    &LEAD_GUITAR,
    &PAD_WARM,
    &PAD_GLASS,
    &KICK_BOOMBAP,
    &KICK_SOFT,
    &SNARE_DUSTY,
    &SNARE_RIM,
    &HAT_CLOSED,
    &HAT_LOOSE,
];

// ---------------------------------------------------------------------------
// Tone: the tape/vinyl character that binds a kit into one record.
// ---------------------------------------------------------------------------

/// The "glue" processing shared by a vibe: tape pitch instability, master
/// filtering, saturation warmth, and the vinyl air bed. See `music::character`.
#[derive(Clone, Copy, Debug)]
pub struct Tone {
    /// Slow tape wow depth (fractional pitch drift).
    pub wow: f32,
    /// Faster flutter depth.
    pub flutter: f32,
    /// Master lowpass cutoff (Hz) — the "behind a closed door" roll-off.
    pub cutoff_hz: f32,
    /// Master saturation warmth.
    pub drive: f32,
    /// Vinyl hiss/crackle bed level.
    pub air: f32,
}

// ---------------------------------------------------------------------------
// Kits: curated vibes.
// ---------------------------------------------------------------------------

/// A vibe: pitched instruments, sampled drums, and their shared tone.
#[derive(Clone, Copy, Debug)]
pub struct Kit {
    pub name: &'static str,
    pub keys: &'static Patch,
    pub bass: &'static Patch,
    pub lead: &'static Patch,
    pub pad: &'static Patch,
    pub drums: &'static DrumBank,
    pub tone: Tone,
}

/// "Dusty" — warm, tape-saturated boom-bap. The house sound.
pub const KIT_DUSTY: Kit = Kit {
    name: "DUSTY",
    keys: &RHODES_WARM,
    bass: &BASS_ROUND,
    lead: &LEAD_MUSICBOX,
    pad: &PAD_WARM,
    drums: &ACOUSTIC_DRUMS,
    tone: Tone {
        wow: 0.0025,
        flutter: 0.0006,
        cutoff_hz: 6200.0,
        drive: 0.15,
        air: 0.35,
    },
};

/// "Rainy" — soft, dark, and hazy: muted keys, deep sub, long pads.
pub const KIT_RAINY: Kit = Kit {
    name: "RAINY",
    keys: &RHODES_WARM,
    bass: &BASS_SUB,
    lead: &LEAD_FLUTE,
    pad: &PAD_WARM,
    drums: &ACOUSTIC_DRUMS,
    tone: Tone {
        wow: 0.004,
        flutter: 0.001,
        cutoff_hz: 5200.0,
        drive: 0.12,
        air: 0.4,
    },
};

/// "Neon" — brighter and more forward: bell keys, plucky bass, glassy pads.
pub const KIT_NEON: Kit = Kit {
    name: "NEON",
    keys: &RHODES_BELL,
    bass: &BASS_PLUCK,
    lead: &LEAD_GUITAR,
    pad: &PAD_GLASS,
    drums: &ACOUSTIC_DRUMS,
    tone: Tone {
        wow: 0.0018,
        flutter: 0.0009,
        cutoff_hz: 7800.0,
        drive: 0.1,
        air: 0.15,
    },
};

/// "Velvet" — smoky lounge: Wurli, round bass, breathy lead, plenty of wow.
pub const KIT_VELVET: Kit = Kit {
    name: "VELVET",
    keys: &WURLI,
    bass: &BASS_ROUND,
    lead: &LEAD_FLUTE,
    pad: &PAD_GLASS,
    drums: &ACOUSTIC_DRUMS,
    tone: Tone {
        wow: 0.0035,
        flutter: 0.0012,
        cutoff_hz: 6000.0,
        drive: 0.18,
        air: 0.25,
    },
};

/// The vibe catalogue. Add a kit here and the mesh can land on it.
pub const KITS: &[&Kit] = &[&KIT_DUSTY, &KIT_RAINY, &KIT_NEON, &KIT_VELVET];

/// Pick the vibe for a seed. Deterministic, so every box agrees.
pub fn kit_for(seed: u64) -> &'static Kit {
    let h = super::arrangement::mix64(seed ^ 0x7669_6265_6b69_7401);
    KITS[(h as usize) % KITS.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kit_selectable_and_stable() {
        // Deterministic for a given seed.
        for s in 0..64u64 {
            assert_eq!(kit_for(s).name, kit_for(s).name);
        }
        // The whole catalogue is reachable across seeds.
        let mut seen = [false; 4];
        for s in 0..500u64 {
            let name = kit_for(s).name;
            let ix = KITS.iter().position(|k| k.name == name).unwrap();
            seen[ix] = true;
        }
        assert!(seen.iter().all(|&b| b), "some kit never selected: {seen:?}");
    }
}
