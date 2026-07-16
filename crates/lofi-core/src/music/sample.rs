//! Flash-resident sample playback for the embedded audio path.
//!
//! Samples are mono, headerless G.711 mu-law at a declared source rate. Mu-law
//! keeps one byte per source frame while preserving quiet tails better than
//! linear 8-bit PCM. Playback is stateless and indexed from note age, so clock
//! correction or block boundaries cannot leave a voice cursor out of sync.

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    data: &'static [u8],
    sample_rate: u32,
    gain: f32,
}

impl Sample {
    pub const fn mulaw(data: &'static [u8], sample_rate: u32, gain: f32) -> Self {
        Self {
            data,
            sample_rate,
            gain,
        }
    }

    pub const fn len(&self) -> usize {
        self.data.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

/// Render one linearly interpolated sample `age` seconds after its onset.
#[inline]
pub fn render_sample(sample: &Sample, age: f32) -> f32 {
    if age < 0.0 || sample.data.is_empty() {
        return 0.0;
    }
    let position = age * sample.sample_rate as f32;
    let index = position as usize;
    let Some(&a) = sample.data.get(index) else {
        return 0.0;
    };
    let fraction = position - index as f32;
    let b = sample.data.get(index + 1).copied().unwrap_or(a);
    let a = decode_mulaw(a);
    (a + (decode_mulaw(b) - a) * fraction) * sample.gain
}

#[inline]
fn decode_mulaw(encoded: u8) -> f32 {
    const BIAS: i32 = 0x84;

    let code = !encoded;
    let exponent = ((code >> 4) & 0x07) as u32;
    let mantissa = (code & 0x0f) as i32;
    let magnitude = (((mantissa << 3) + BIAS) << exponent) - BIAS;
    let pcm = if code & 0x80 != 0 {
        -magnitude
    } else {
        magnitude
    };
    pcm as f32 / 32_768.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_silence_decodes_to_zero() {
        assert_eq!(decode_mulaw(0xff), 0.0);
        assert_eq!(decode_mulaw(0x7f), 0.0);
    }

    #[test]
    fn playback_interpolates_and_stops() {
        static DATA: [u8; 3] = [0xff, 0x80, 0xff];
        let sample = Sample::mulaw(&DATA, 2, 0.5);
        assert_eq!(render_sample(&sample, -0.1), 0.0);
        assert!(render_sample(&sample, 0.5).abs() > 0.4);
        assert!(render_sample(&sample, 0.25).abs() > 0.2);
        assert_eq!(render_sample(&sample, 2.0), 0.0);
    }

    #[test]
    fn descriptor_reports_flash_cost() {
        let sample = Sample::mulaw(&[0xff; 32], 22_050, 1.0);
        assert_eq!(sample.len(), 32);
        assert_eq!(sample.sample_rate(), 22_050);
        assert!(!sample.is_empty());
    }
}
