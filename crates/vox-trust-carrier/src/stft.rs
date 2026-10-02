//! Short-time Fourier transform with square-root Hann windows (perfect reconstruction at a
//! 50 % hop), and synthesis of the watermark as a *difference* signal so that audio outside
//! the modified tiles is returned bit-exact.

use std::f32::consts::PI;
use std::sync::Arc;

use realfft::num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};

use crate::{Params, FRAME, HOP};

pub(crate) type Spectrum = Vec<Complex<f32>>;

pub(crate) struct Stft {
    window: Vec<f32>,
    forward: Arc<dyn RealToComplex<f32>>,
    inverse: Arc<dyn ComplexToReal<f32>>,
}

impl Stft {
    pub(crate) fn new() -> Stft {
        let mut planner = RealFftPlanner::<f32>::new();
        // sqrt(periodic Hann) = sin(pi n / N); analysis x synthesis sums to 1 at hop N/2.
        let window = (0..FRAME)
            .map(|n| (PI * n as f32 / FRAME as f32).sin())
            .collect();
        Stft {
            window,
            forward: planner.plan_fft_forward(FRAME),
            inverse: planner.plan_fft_inverse(FRAME),
        }
    }

    /// Frames starting at `shift`, `shift + HOP`, ... that fit entirely in `samples`.
    pub(crate) fn analyse(&mut self, samples: &[f32], shift: usize) -> Vec<Spectrum> {
        self.analyse_hop(samples, shift, HOP as f64)
    }

    /// Frames starting at `shift + round(k * hop)`: a fractional hop undoes a tempo change
    /// (a hop of `HOP / 1.01` re-aligns audio played 1 % faster) without moving any
    /// frequency.
    pub(crate) fn analyse_hop(&mut self, samples: &[f32], shift: usize, hop: f64) -> Vec<Spectrum> {
        let mut frames = Vec::new();
        let mut input = self.forward.make_input_vec();
        let mut k = 0usize;
        loop {
            let start = shift + (k as f64 * hop).round() as usize;
            if start + FRAME > samples.len() {
                break;
            }
            for (i, v) in input.iter_mut().enumerate() {
                *v = samples[start + i] * self.window[i];
            }
            let mut out = self.forward.make_output_vec();
            self.forward
                .process(&mut input, &mut out)
                .expect("buffer sizes come from the plan");
            frames.push(out);
            k += 1;
        }
        frames
    }

    /// The time-domain change produced by multiplying each tile of each frame (grid starting
    /// at sample 0) by its gain, `gains_db[frame * subbands + subband]`.
    pub(crate) fn synthesise_gain_delta(
        &mut self,
        frames: &[Spectrum],
        gains_db: &[f32],
        p: &Params,
        len: usize,
    ) -> Vec<f32> {
        let sb = p.subbands();
        let mut delta = vec![0f32; len];
        let mut spec = self.inverse.make_input_vec();
        let mut out = self.inverse.make_output_vec();
        let scale = 1.0 / FRAME as f32;
        for (f, frame) in frames.iter().enumerate() {
            let gains = &gains_db[f * sb..(f + 1) * sb];
            if gains.iter().all(|&g| g == 0.0) {
                continue;
            }
            spec.iter_mut().for_each(|c| *c = Complex::new(0.0, 0.0));
            for (q, &g) in gains.iter().enumerate() {
                let factor = 10f32.powf(g / 20.0) - 1.0;
                for b in 0..p.tile_bins {
                    let k = p.lo_bin + q * p.tile_bins + b;
                    spec[k] = frame[k] * factor;
                }
            }
            self.inverse
                .process(&mut spec, &mut out)
                .expect("DC and Nyquist bins are zero");
            let start = f * HOP;
            for (i, &v) in out.iter().enumerate() {
                delta[start + i] += v * scale * self.window[i];
            }
        }
        delta
    }
}

/// A power floor 100 dB below the loudest bin, so silent bins have a finite log level.
pub(crate) fn power_floor(frames: &[Spectrum]) -> f32 {
    let peak = frames
        .iter()
        .flat_map(|f| f.iter())
        .map(|c| c.norm_sqr())
        .fold(0f32, f32::max);
    (peak * 1e-10).max(1e-20)
}
