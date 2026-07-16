use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

use claxon::FlacReader;

const TARGET_RATE: u32 = 22_050;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let input = args
        .next()
        .ok_or("usage: lofi-sample-packer INPUT.flac OUTPUT.ulaw")?;
    let output = args
        .next()
        .ok_or("usage: lofi-sample-packer INPUT.flac OUTPUT.ulaw")?;
    if args.next().is_some() {
        return Err("usage: lofi-sample-packer INPUT.flac OUTPUT.ulaw".into());
    }

    let mut reader = FlacReader::open(&input)?;
    let info = reader.streaminfo();
    let channels = info.channels as usize;
    if channels == 0 {
        return Err("source has no channels".into());
    }
    let scale = (1_u64 << (info.bits_per_sample - 1)) as f64;
    let decoded = reader.samples().collect::<Result<Vec<i32>, _>>()?;
    let mono = decoded
        .chunks_exact(channels)
        .map(|frame| {
            frame
                .iter()
                .map(|&sample| sample as f64 / scale)
                .sum::<f64>()
                / channels as f64
        })
        .collect::<Vec<_>>();

    let output_frames = mono.len() as u64 * TARGET_RATE as u64 / info.sample_rate as u64;
    let mut encoded = Vec::with_capacity(output_frames as usize);
    for frame in 0..output_frames {
        let position = frame as f64 * info.sample_rate as f64 / TARGET_RATE as f64;
        let index = position as usize;
        let fraction = position - index as f64;
        let a = mono[index];
        let b = mono.get(index + 1).copied().unwrap_or(a);
        let pcm = ((a + (b - a) * fraction).clamp(-1.0, 1.0) * 32_767.0) as i16;
        encoded.push(encode_mulaw(pcm));
    }

    fs::write(&output, &encoded)?;
    println!(
        "{}: {} Hz/{} ch -> {}: {} bytes at {} Hz mu-law",
        Path::new(&input).display(),
        info.sample_rate,
        channels,
        Path::new(&output).display(),
        encoded.len(),
        TARGET_RATE,
    );
    Ok(())
}

fn encode_mulaw(pcm: i16) -> u8 {
    const BIAS: i32 = 0x84;
    const CLIP: i32 = 32_635;

    let mut magnitude = pcm as i32;
    let sign = if magnitude < 0 {
        magnitude = -magnitude;
        0x80
    } else {
        0
    };
    magnitude = magnitude.min(CLIP) + BIAS;

    let mut exponent = 7_u8;
    let mut mask = 0x4000_i32;
    while exponent > 0 && magnitude & mask == 0 {
        exponent -= 1;
        mask >>= 1;
    }
    let mantissa = ((magnitude >> (exponent + 3)) & 0x0f) as u8;
    !(sign | (exponent << 4) | mantissa)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mulaw_silence_has_canonical_code() {
        assert_eq!(encode_mulaw(0), 0xff);
    }

    #[test]
    fn mulaw_preserves_sign() {
        assert_ne!(encode_mulaw(12_000) & 0x80, encode_mulaw(-12_000) & 0x80);
    }
}
