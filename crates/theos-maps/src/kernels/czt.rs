//! The chirp-z transform (Bluestein 1970) on the radix-2 FFT.
//!
//! It computes `out[m] = sum over n < inputs of x[n] w^(n m)`, for `m < outputs`, with
//! `w = e^(2 pi i num / den)`: a DFT of any length, or a lattice of frequencies at any rational
//! step. The spectral noise uses it to sum a wavenumber lattice fixed by the map at the cell
//! centres of any grid (`noise.rs`).
//!
//! Bluestein's identity `n m = (n² + m² − (m − n)²) / 2` turns the sum into a convolution with
//! the chirp `c(k) = w^(k² / 2)`: `out[m] = c(m) · sum over n of (x[n] c(n)) · conj(c(m − n))`.
//! The convolution runs through FFTs of the smallest power of two of at least
//! `inputs + outputs − 1`. The chirp's angle, `num k² / (2 den)` turns, is reduced modulo one turn
//! in integers ([`Complex::from_turns`]), so it keeps full precision for every `k`.

use std::num::NonZeroUsize;

use super::KernelError;
use super::complex::Complex;
use super::fft::{next_pow2, transform, twiddles};
use super::num::to_f64;

/// A chirp-z transform planned for one input length, output length and step.
#[derive(Debug, Clone)]
pub struct Czt {
    inputs: usize,
    outputs: usize,
    /// `c(k)` for `k < max(inputs, outputs)`.
    chirp: Vec<Complex>,
    /// The FFT of `conj(c)` wrapped onto the convolution length, divided by that length (the
    /// inverse FFT's scaling).
    filter: Vec<Complex>,
    forward: Vec<Complex>,
    inverse: Vec<Complex>,
}

impl Czt {
    /// The transform of `inputs` values to `outputs` values with `w = e^(2 pi i num / den)`.
    #[must_use]
    pub fn new(inputs: usize, outputs: usize, num: usize, den: NonZeroUsize) -> Self {
        let len = next_pow2((inputs + outputs).saturating_sub(1));
        // c(k) = e^(2 pi i (num k² mod 2 den) / (2 den)); `twice` cannot be zero.
        let twice = NonZeroUsize::new(2 * den.get()).unwrap_or(den);
        let chirp: Vec<Complex> = (0..inputs.max(outputs))
            .map(|k| {
                let k = k % twice.get();
                Complex::from_turns(num % twice.get() * (k * k % twice.get()), twice)
            })
            .collect();
        let (forward, inverse) = (twiddles(len, false), twiddles(len, true));
        let mut filter = vec![Complex::ZERO; len];
        if inputs > 0 && outputs > 0 {
            for (j, c) in chirp.iter().enumerate().take(outputs) {
                filter[j] = c.conj();
            }
            for (j, c) in chirp.iter().enumerate().take(inputs).skip(1) {
                filter[len - j] = c.conj();
            }
        }
        transform(&mut filter, &forward);
        let scale = 1.0 / to_f64(len);
        for v in &mut filter {
            *v = v.scale(scale);
        }
        Self {
            inputs,
            outputs,
            chirp,
            filter,
            forward,
            inverse,
        }
    }

    /// `out[m] = sum over n of input[n] w^(n m)` for every `m < outputs`.
    ///
    /// # Errors
    /// [`KernelError::Shape`] when `input` does not hold `inputs` values.
    pub fn apply(&self, input: &[Complex]) -> Result<Vec<Complex>, KernelError> {
        if input.len() != self.inputs {
            return Err(KernelError::Shape {
                rows: 1,
                cols: self.inputs,
                len: input.len(),
            });
        }
        if self.inputs == 0 {
            return Ok(vec![Complex::ZERO; self.outputs]);
        }
        let mut work = vec![Complex::ZERO; self.filter.len()];
        for ((slot, x), c) in work.iter_mut().zip(input).zip(&self.chirp) {
            *slot = *x * *c;
        }
        transform(&mut work, &self.forward);
        for (v, f) in work.iter_mut().zip(&self.filter) {
            *v = *v * *f;
        }
        transform(&mut work, &self.inverse);
        Ok(work
            .iter()
            .zip(&self.chirp)
            .take(self.outputs)
            .map(|(v, c)| *v * *c)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::fft::fft;
    use crate::rng::StageRng;

    fn random(n: usize, key: &str) -> Vec<Complex> {
        let mut rng = StageRng::for_key(2, key);
        (0..n)
            .map(|_| Complex::new(rng.normal(), rng.normal()))
            .collect()
    }

    fn den(d: usize) -> NonZeroUsize {
        NonZeroUsize::new(d).unwrap()
    }

    /// The sum the transform stands for, term by term.
    fn direct(x: &[Complex], outputs: usize, num: usize, d: usize) -> Vec<Complex> {
        (0..outputs)
            .map(|m| {
                x.iter().enumerate().fold(Complex::ZERO, |acc, (n, v)| {
                    acc + *v * Complex::from_turns(num * n * m % d, den(d))
                })
            })
            .collect()
    }

    #[test]
    fn czt_matches_the_direct_sum() {
        // The noise's shapes: 513 lattice values to 384 cells at 3/4 cycle per map is
        // num = 6, den = 8 * 384; then a short odd case and the smallest.
        for (inputs, outputs, num, d) in [(513, 384, 6, 3072), (7, 5, 3, 64), (1, 1, 1, 2)] {
            let x = random(inputs, "direct");
            let got = Czt::new(inputs, outputs, num, den(d)).apply(&x).unwrap();
            let want = direct(&x, outputs, num, d);
            let scale: f64 = x.iter().map(|v| v.re.abs() + v.im.abs()).sum();
            for (m, (g, w)) in got.iter().zip(&want).enumerate() {
                let err = (g.re - w.re).abs() + (g.im - w.im).abs();
                assert!(err < 1e-12 * scale, "{inputs} -> {outputs}, m {m}: {err}");
            }
        }
    }

    #[test]
    fn czt_at_the_dft_step_is_the_dft() {
        // w = e^(2 pi i 15/16) = e^(-2 pi i / 16), the forward DFT of 16 values.
        let x = random(16, "dft");
        let got = Czt::new(16, 16, 15, den(16)).apply(&x).unwrap();
        let mut want = x;
        fft(&mut want, false).unwrap();
        for (g, w) in got.iter().zip(&want) {
            assert!((g.re - w.re).abs() < 1e-12 && (g.im - w.im).abs() < 1e-12);
        }
    }

    #[test]
    fn czt_of_an_empty_side_is_zeros_or_nothing() {
        let none = Czt::new(0, 3, 1, den(4)).apply(&[]).unwrap();
        assert_eq!(none, vec![Complex::ZERO; 3]);
        let x = random(3, "empty");
        assert_eq!(Czt::new(3, 0, 1, den(4)).apply(&x).unwrap(), Vec::new());
    }

    #[test]
    fn czt_refuses_a_wrong_input_length() {
        assert_eq!(
            Czt::new(4, 2, 1, den(4)).apply(&random(3, "short")),
            Err(KernelError::Shape {
                rows: 1,
                cols: 4,
                len: 3
            })
        );
    }
}
