//! Streaming resampler that downsamples audio to the transcription rate.
//!
//! For integer ratios (e.g. 48kHz -> 16kHz) it applies an anti-aliasing
//! low-pass FIR before decimating, which avoids the high-frequency aliasing
//! that plain "take every Nth sample" decimation produces. State (filter
//! history) is kept across chunks, so create one `Resampler` per stream.

use std::f32::consts::PI;

/// Number of FIR taps. Odd-ish length is fine; 63 gives a good speech-band
/// roll-off without being expensive.
const FIR_TAPS: usize = 63;

/// Stateful resampler from `in_rate` to `out_rate`.
pub struct Resampler {
    mode: Mode,
}

enum Mode {
    /// Rates match; pass samples through untouched.
    Passthrough,
    /// Integer downsample with anti-aliasing.
    Decimate(FirDecimator),
    /// Non-integer ratio fallback: plain decimation (no anti-alias).
    LinearDecimate { ratio: f32 },
}

impl Resampler {
    pub fn new(in_rate: u32, out_rate: u32) -> Self {
        if in_rate == out_rate || in_rate == 0 || out_rate == 0 {
            return Self { mode: Mode::Passthrough };
        }
        if in_rate.is_multiple_of(out_rate) {
            let factor = (in_rate / out_rate) as usize;
            // Cut off just below the output Nyquist to leave a transition band.
            let cutoff_norm = 0.45 * out_rate as f32 / in_rate as f32;
            let taps = design_lowpass(FIR_TAPS, cutoff_norm);
            Self {
                mode: Mode::Decimate(FirDecimator::new(taps, factor)),
            }
        } else {
            Self {
                mode: Mode::LinearDecimate {
                    ratio: in_rate as f32 / out_rate as f32,
                },
            }
        }
    }

    /// Resample one chunk. Output length is roughly `input.len() * out / in`.
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        match &mut self.mode {
            Mode::Passthrough => input.to_vec(),
            Mode::Decimate(fir) => fir.process(input),
            Mode::LinearDecimate { ratio } => linear_decimate(input, *ratio),
        }
    }
}

/// Streaming FIR low-pass followed by integer decimation.
struct FirDecimator {
    taps: Vec<f32>,
    factor: usize,
    /// Pending input samples; `buf[cursor..]` is not yet fully consumed.
    buf: Vec<f32>,
    cursor: usize,
}

impl FirDecimator {
    fn new(taps: Vec<f32>, factor: usize) -> Self {
        Self {
            taps,
            factor,
            buf: Vec::new(),
            cursor: 0,
        }
    }

    fn process(&mut self, input: &[f32]) -> Vec<f32> {
        self.buf.extend_from_slice(input);

        let ntaps = self.taps.len();
        let mut out = Vec::with_capacity(self.buf.len() / self.factor + 1);

        // Emit one output per full FIR window, stepping the cursor by `factor`.
        while self.cursor + ntaps <= self.buf.len() {
            let window = &self.buf[self.cursor..self.cursor + ntaps];
            let mut acc = 0.0;
            for (h, &x) in self.taps.iter().zip(window) {
                acc += h * x;
            }
            out.push(acc);
            self.cursor += self.factor;
        }

        // Drop samples that can no longer contribute to a future window.
        if self.cursor > 0 {
            self.buf.drain(..self.cursor);
            self.cursor = 0;
        }

        out
    }
}

/// Plain decimation by a fractional ratio (no anti-aliasing). Fallback only.
fn linear_decimate(input: &[f32], ratio: f32) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }
    let out_len = (input.len() as f32 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let idx = (i as f32 * ratio) as usize;
        if idx < input.len() {
            out.push(input[idx]);
        } else {
            break;
        }
    }
    out
}

/// Windowed-sinc low-pass FIR, normalized to unity DC gain.
/// `cutoff_norm` is the cutoff in cycles/sample (0..0.5).
fn design_lowpass(ntaps: usize, cutoff_norm: f32) -> Vec<f32> {
    let m = (ntaps - 1) as f32;
    let mut taps = Vec::with_capacity(ntaps);

    for n in 0..ntaps {
        let x = n as f32 - m / 2.0;
        let sinc = if x.abs() < 1e-6 {
            2.0 * cutoff_norm
        } else {
            (2.0 * PI * cutoff_norm * x).sin() / (PI * x)
        };
        // Hamming window.
        let w = 0.54 - 0.46 * (2.0 * PI * n as f32 / m).cos();
        taps.push(sinc * w);
    }

    let sum: f32 = taps.iter().sum();
    if sum.abs() > f32::EPSILON {
        for t in &mut taps {
            *t /= sum;
        }
    }
    taps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_when_rates_match() {
        let mut r = Resampler::new(16000, 16000);
        assert_eq!(r.process(&[1.0, 2.0, 3.0]), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn decimates_by_integer_factor() {
        let mut r = Resampler::new(48000, 16000);
        // 3:1 ratio. The FIR holds back up to ~FIR_TAPS samples as filter
        // context, so the first chunk yields slightly fewer than input/3.
        let input = vec![0.5; 6000];
        let out = r.process(&input);
        let expected = 6000 / 3;
        assert!(
            out.len() <= expected && out.len() + FIR_TAPS >= expected,
            "len={}",
            out.len()
        );
    }

    #[test]
    fn preserves_dc_level() {
        let mut r = Resampler::new(48000, 16000);
        let out = r.process(&vec![1.0; 6000]);
        // DC gain is unity; interior samples should sit near 1.0.
        let mid = out[out.len() / 2];
        assert!((mid - 1.0).abs() < 0.01, "mid={mid}");
    }

    #[test]
    fn attenuates_above_output_nyquist() {
        // 12kHz tone at 48kHz is above the 8kHz output Nyquist -> must be cut.
        let mut r = Resampler::new(48000, 16000);
        let n = 9000;
        let tone: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * 12000.0 * i as f32 / 48000.0).sin())
            .collect();
        let out = r.process(&tone);
        let peak = out[out.len() / 4..].iter().fold(0.0_f32, |m, &x| m.max(x.abs()));
        assert!(peak < 0.2, "high-freq tone not attenuated: peak={peak}");
    }
}
