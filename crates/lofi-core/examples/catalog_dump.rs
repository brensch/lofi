//! Dump the shipped catalogue as JSONL for offline symbolic-engine tooling.

use lofi_core::music::{ElementKind, AI_CATALOG};

fn main() {
    for index in 0..AI_CATALOG.len() {
        let Some(element) = AI_CATALOG.element(index) else {
            continue;
        };
        let kind = match element.kind {
            ElementKind::Kick => "kick",
            ElementKind::Snare => "snare",
            ElementKind::Hat => "hat",
            ElementKind::DrumLoop => "drum_loop",
            ElementKind::BassNote => "bass_note",
            ElementKind::BassLoop => "bass_loop",
            ElementKind::LeadNote => "lead_note",
            ElementKind::MelodyLoop => "melody_loop",
            ElementKind::KeysNote => "keys_note",
            ElementKind::HarmonyLoop => "harmony_loop",
            ElementKind::TextureLoop => "texture_loop",
        };
        let root = element
            .root_semitone
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string());
        println!(
            "{{\"index\":{index},\"kind\":\"{kind}\",\"root\":{root},\"bars\":{},\"phase\":{},\
             \"bpm\":{},\"key_class\":{},\"mode\":{},\"source\":{},\"frames\":{},\
             \"rate\":{},\"energy\":{}}}",
            element.bars,
            element.phase,
            element.bpm,
            element.key_class,
            element.mode,
            element.source_hash,
            element.sample.len(),
            element.sample.sample_rate(),
            element.energy
        );
    }
}
