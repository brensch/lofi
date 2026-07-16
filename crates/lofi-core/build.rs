use std::env;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

const SINE_LEN: usize = 1_024;

fn main() {
    let mut out = String::new();
    writeln!(out, "pub const SINE_LEN: usize = {SINE_LEN};").unwrap();
    writeln!(
        out,
        "#[allow(clippy::approx_constant, clippy::excessive_precision)]"
    )
    .unwrap();
    writeln!(out, "pub static SINE: [f32; SINE_LEN] = [").unwrap();
    for i in 0..SINE_LEN {
        let phase = i as f64 / SINE_LEN as f64 * std::f64::consts::TAU;
        writeln!(out, "    {:.9}f32,", phase.sin()).unwrap();
    }
    writeln!(out, "];\n").unwrap();

    writeln!(out, "#[allow(clippy::excessive_precision)]").unwrap();
    writeln!(out, "pub static MIDI_HZ: [f32; 128] = [").unwrap();
    for note in 0..128 {
        let hz = 440.0 * 2.0f64.powf((note as f64 - 69.0) / 12.0);
        writeln!(out, "    {hz:.9}f32,").unwrap();
    }
    writeln!(out, "];").unwrap();

    let path = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("music_tables.rs");
    fs::write(path, out).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
