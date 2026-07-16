//! Fixed binary catalogue for sample-only composition.
//!
//! The offline content forge writes a header, constant-time kind/root indexes,
//! fixed-size metadata records, then mu-law audio. Parsing borrows directly from
//! flash/WASM read-only memory and never allocates.

use super::sample::Sample;

const HEADER_SIZE: usize = 16;
const KIND_COUNT: usize = 11;
const KIND_TABLE_SIZE: usize = KIND_COUNT * 4;
const ROOT_KIND_COUNT: usize = 3;
const ROOT_VARIANTS: usize = 8;
const ROOT_TABLE_SIZE: usize = ROOT_KIND_COUNT * 128 * ROOT_VARIANTS * 2;
const RECORD_SIZE: usize = 28;
const RECORDS_OFFSET: usize = HEADER_SIZE + KIND_TABLE_SIZE + ROOT_TABLE_SIZE;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementKind {
    Kick = 0,
    Snare = 1,
    Hat = 2,
    DrumLoop = 3,
    BassNote = 4,
    BassLoop = 5,
    LeadNote = 6,
    MelodyLoop = 7,
    KeysNote = 8,
    HarmonyLoop = 9,
    TextureLoop = 10,
}

impl ElementKind {
    fn root_table(self) -> Option<usize> {
        match self {
            Self::BassNote => Some(0),
            Self::LeadNote => Some(1),
            Self::KeysNote => Some(2),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PackedElement {
    pub sample: Sample,
    pub kind: ElementKind,
    pub looped: bool,
    pub bars: u8,
    pub phase: u8,
    pub bpm: u16,
    pub key_class: u8,
    pub mode: u8,
    pub source_hash: u32,
    pub progression_hash: u32,
    pub root_semitone: Option<u8>,
    pub energy: u8,
}

/// One phase-aligned set of stems harvested from the same source performance.
#[derive(Clone, Copy, Debug)]
pub struct LoopScene {
    pub source_hash: u32,
    pub drums: [Option<PackedElement>; 4],
    pub bass: Option<PackedElement>,
    pub melody: Option<PackedElement>,
    pub harmony: Option<PackedElement>,
    pub texture: Option<PackedElement>,
}

#[derive(Clone, Copy, Debug)]
pub struct PackedCatalog {
    bytes: &'static [u8],
}

impl PackedCatalog {
    pub const fn new(bytes: &'static [u8]) -> Self {
        Self { bytes }
    }

    pub fn is_valid(&self) -> bool {
        if self.bytes.len() < RECORDS_OFFSET || &self.bytes[..4] != b"LFPK" {
            return false;
        }
        let count = self.len();
        let data_offset = self.u32(12) as usize;
        self.bytes[4] == 2
            && self.bytes[5] == 1
            && data_offset == RECORDS_OFFSET + count * RECORD_SIZE
            && data_offset <= self.bytes.len()
    }

    pub fn len(&self) -> usize {
        self.u16(6) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Number of elements available for a role, read from the fixed kind index.
    pub fn len_for_kind(&self, kind: ElementKind) -> usize {
        let table = HEADER_SIZE + kind as usize * 4;
        self.u16(table + 2) as usize
    }

    pub fn choose(&'static self, kind: ElementKind, selector: u64) -> Option<PackedElement> {
        let table = HEADER_SIZE + kind as usize * 4;
        let start = self.u16(table) as usize;
        let count = self.len_for_kind(kind);
        if count == 0 {
            return None;
        }
        self.element(start + selector as usize % count)
    }

    /// Select a coherent source and resolve all of its aligned loop stems.
    /// This scans fixed metadata only when a device starts or changes seed.
    pub fn loop_scene(&'static self, selector: u64) -> Option<LoopScene> {
        let melodies = self.len_for_kind(ElementKind::MelodyLoop);
        let harmonies = self.len_for_kind(ElementKind::HarmonyLoop);
        let anchor_count = melodies + harmonies;
        if anchor_count == 0 {
            return None;
        }
        let selected = selector as usize % anchor_count;
        let anchor = if selected < melodies {
            self.choose(ElementKind::MelodyLoop, selected as u64)?
        } else {
            self.choose(ElementKind::HarmonyLoop, (selected - melodies) as u64)?
        };
        let source_hash = anchor.source_hash;
        let mut drums = [None; 4];
        for (phase, slot) in drums.iter_mut().enumerate() {
            *slot = self.matching_loop(ElementKind::DrumLoop, source_hash, phase as u8);
        }
        Some(LoopScene {
            source_hash,
            drums,
            bass: self.matching_loop(ElementKind::BassLoop, source_hash, 0),
            melody: self.matching_loop(ElementKind::MelodyLoop, source_hash, 0),
            harmony: self.matching_loop(ElementKind::HarmonyLoop, source_hash, 0),
            texture: self.matching_loop(ElementKind::TextureLoop, source_hash, 0),
        })
    }

    pub fn nearest_note(
        &'static self,
        kind: ElementKind,
        target_semitone: u8,
        selector: u64,
    ) -> Option<PackedElement> {
        let table = kind.root_table()?;
        let variant = selector as usize % ROOT_VARIANTS;
        let offset = HEADER_SIZE
            + KIND_TABLE_SIZE
            + ((table * 128 + target_semitone.min(127) as usize) * ROOT_VARIANTS + variant) * 2;
        let index = self.u16(offset);
        if index == u16::MAX {
            None
        } else {
            self.element(index as usize)
        }
    }

    pub fn element(&'static self, index: usize) -> Option<PackedElement> {
        if index >= self.len() {
            return None;
        }
        let base = RECORDS_OFFSET + index * RECORD_SIZE;
        let kind = match self.bytes[base] {
            0 => ElementKind::Kick,
            1 => ElementKind::Snare,
            2 => ElementKind::Hat,
            3 => ElementKind::DrumLoop,
            4 => ElementKind::BassNote,
            5 => ElementKind::BassLoop,
            6 => ElementKind::LeadNote,
            7 => ElementKind::MelodyLoop,
            8 => ElementKind::KeysNote,
            9 => ElementKind::HarmonyLoop,
            10 => ElementKind::TextureLoop,
            _ => return None,
        };
        let offset = self.u32(base + 20) as usize;
        let length = self.u32(base + 24) as usize;
        let data = self.bytes.get(offset..offset.checked_add(length)?)?;
        let gain = self.u16(base + 18) as f32 / 32_768.0;
        let root = self.bytes[base + 16] as i8;
        Some(PackedElement {
            sample: Sample::mulaw(data, self.u32(8), gain),
            kind,
            looped: self.bytes[base + 1] & 1 != 0,
            bars: self.bytes[base + 2],
            phase: self.bytes[base + 3],
            bpm: self.u16(base + 4),
            key_class: self.bytes[base + 6],
            mode: self.bytes[base + 7],
            source_hash: self.u32(base + 8),
            progression_hash: self.u32(base + 12),
            root_semitone: (root >= 0).then_some(root as u8),
            energy: self.bytes[base + 17],
        })
    }

    fn matching_loop(
        &'static self,
        kind: ElementKind,
        source_hash: u32,
        phase: u8,
    ) -> Option<PackedElement> {
        let table = HEADER_SIZE + kind as usize * 4;
        let start = self.u16(table) as usize;
        let count = self.len_for_kind(kind);
        (start..start + count)
            .filter_map(|index| self.element(index))
            .find(|element| element.source_hash == source_hash && element.phase == phase)
    }

    fn u16(&self, offset: usize) -> u16 {
        let Some(bytes) = self.bytes.get(offset..offset + 2) else {
            return 0;
        };
        u16::from_le_bytes([bytes[0], bytes[1]])
    }

    fn u32(&self, offset: usize) -> u32 {
        let Some(bytes) = self.bytes.get(offset..offset + 4) else {
            return 0;
        };
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
}

pub static AI_CATALOG: PackedCatalog =
    PackedCatalog::new(include_bytes!("../../../../assets/content/catalog.pack"));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::sample::render_sample;

    #[test]
    fn shipped_catalog_is_valid_and_substantial() {
        assert!(AI_CATALOG.is_valid());
        assert!(AI_CATALOG.len() >= 100);
        assert!(AI_CATALOG.len_for_kind(ElementKind::Kick) >= 30);
        assert!(AI_CATALOG.len_for_kind(ElementKind::Snare) >= 20);
        assert!(AI_CATALOG.len_for_kind(ElementKind::Hat) >= 30);
        assert!(AI_CATALOG.len_for_kind(ElementKind::BassNote) >= 50);
        assert!(AI_CATALOG.len_for_kind(ElementKind::LeadNote) >= 20);
        assert!(AI_CATALOG.len_for_kind(ElementKind::KeysNote) >= ROOT_VARIANTS);
        assert!(AI_CATALOG.choose(ElementKind::Kick, 42).is_some());
        assert!(AI_CATALOG
            .nearest_note(ElementKind::LeadNote, 60, 0)
            .is_some());
        assert!(AI_CATALOG
            .nearest_note(ElementKind::KeysNote, 60, 0)
            .is_some());
    }

    #[test]
    fn selectors_are_stable() {
        let a = AI_CATALOG.choose(ElementKind::Hat, 987).unwrap();
        let b = AI_CATALOG.choose(ElementKind::Hat, 987).unwrap();
        assert_eq!(a.sample.len(), b.sample.len());
        assert_eq!(a.energy, b.energy);
    }

    #[test]
    fn pitched_note_tables_offer_real_variation() {
        for kind in [ElementKind::BassNote, ElementKind::LeadNote] {
            let first = AI_CATALOG.nearest_note(kind, 60, 0).unwrap();
            let varied = (1..ROOT_VARIANTS as u64).any(|selector| {
                let candidate = AI_CATALOG.nearest_note(kind, 60, selector).unwrap();
                candidate.source_hash != first.source_hash
                    || candidate.root_semitone != first.root_semitone
                    || candidate.sample.len() != first.sample.len()
            });
            assert!(varied, "{kind:?} root table repeats one element");
        }
    }

    #[test]
    fn loop_scenes_never_mix_source_performances() {
        for selector in 0..16 {
            let scene = AI_CATALOG.loop_scene(selector).unwrap();
            assert!(scene.drums.into_iter().all(|element| element.is_some()));
            assert!(scene.bass.is_some());
            assert!(scene.texture.is_some());
            assert!(scene.melody.is_some() || scene.harmony.is_some());
            let elements = scene.drums.into_iter().chain([
                scene.bass,
                scene.melody,
                scene.harmony,
                scene.texture,
            ]);
            assert!(elements
                .flatten()
                .all(|element| element.source_hash == scene.source_hash));
        }
    }

    #[test]
    fn shipped_drum_attacks_are_aligned() {
        for kind in [ElementKind::Kick, ElementKind::Snare, ElementKind::Hat] {
            for selector in 0..AI_CATALOG.len_for_kind(kind) {
                let sample = AI_CATALOG.choose(kind, selector as u64).unwrap().sample;
                let peak = (0..sample.len())
                    .map(|frame| {
                        render_sample(&sample, frame as f32 / sample.sample_rate() as f32).abs()
                    })
                    .fold(0.0_f32, f32::max);
                let threshold = peak * 0.08;
                let onset = (0..sample.len())
                    .find(|frame| {
                        render_sample(&sample, *frame as f32 / sample.sample_rate() as f32).abs()
                            >= threshold
                    })
                    .unwrap();
                assert!(
                    onset * 800 <= sample.sample_rate() as usize,
                    "{kind:?} selector {selector} starts at frame {onset}"
                );
            }
        }
    }

    #[test]
    fn shipped_loop_seams_are_quiet() {
        use crate::music::sample::render_sample;

        for kind in [
            ElementKind::DrumLoop,
            ElementKind::BassLoop,
            ElementKind::MelodyLoop,
            ElementKind::HarmonyLoop,
            ElementKind::TextureLoop,
        ] {
            for selector in 0..AI_CATALOG.len_for_kind(kind) {
                let sample = AI_CATALOG.choose(kind, selector as u64).unwrap().sample;
                let first = render_sample(&sample, 0.0).abs();
                let last = render_sample(
                    &sample,
                    (sample.len() - 1) as f32 / sample.sample_rate() as f32,
                )
                .abs();
                assert!(
                    first < 0.002 && last < 0.002,
                    "{kind:?} seam {first}/{last}"
                );
            }
        }
    }
}
