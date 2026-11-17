//! Iterative radix-2 FFT.

use super::KernelError;
use super::complex::Complex;
use super::num::to_f64;
use core::f64::consts::PI;

/// The smallest power of two that is at least `n` (`1` for `0`).
#[must_use]
pub fn next_pow2(n: usize) -> usize {
    n.next_power_of_two()
}

/// The `n / 2` twiddles `exp(-+2 pi i k / n)` (conjugated for the inverse), computed with `libm`.
pub(super) fn twiddles(n: usize, inverse: bool) -> Vec<Complex> {
    let sign = if inverse { 1.0 } else { -1.0 };
    let step = sign * 2.0 * PI / to_f64(n);
    (0..n / 2)
        .map(|k| Complex::from_polar(1.0, step * to_f64(k)))
        .collect()
}

fn check_len(len: usize) -> Result<(), KernelError> {
    if len.is_power_of_two() {
        Ok(())
    } else {
        Err(KernelError::NotPowerOfTwo { len })
    }
}

/// Bit-reversal permutation, then butterfly stages of increasing size; no scaling. `data` has a
/// power-of-two length and `twiddles` are [`twiddles`] of that length.
pub(super) fn transform(data: &mut [Complex], twiddles: &[Complex]) {
    let n = data.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            data.swap(i, j);
        }
    }
    let mut size = 2;
    while size <= n {
        let half = size / 2;
        let stride = n / size;
        for block in data.chunks_exact_mut(size) {
            let (lo, hi) = block.split_at_mut(half);
            for (k, (a, b)) in lo.iter_mut().zip(hi.iter_mut()).enumerate() {
                let t = *b * twiddles[k * stride];
                *b = *a - t;
                *a = *a + t;
            }
        }
        size *= 2;
    }
}

fn scale_all(data: &mut [Complex], count: usize) {
    let s = 1.0 / to_f64(count);
    for v in data {
        *v = v.scale(s);
    }
}

/// In-place iterative radix-2 FFT; the inverse is scaled by `1 / n`.
///
/// # Errors
/// [`KernelError::NotPowerOfTwo`] when the length is not a power of two (zero included).
pub fn fft(data: &mut [Complex], inverse: bool) -> Result<(), KernelError> {
    check_len(data.len())?;
    transform(data, &twiddles(data.len(), inverse));
    if inverse {
        scale_all(data, data.len());
    }
    Ok(())
}

/// In-place 2D FFT of a row-major `rows x cols` array: every row first, then every column,
/// forward and inverse alike. The inverse is scaled by `1 / (rows * cols)`.
///
/// # Errors
/// [`KernelError::Shape`] when `data.len() != rows * cols`; [`KernelError::NotPowerOfTwo`] when
/// `rows` or `cols` is not a power of two.
pub fn fft2(
    data: &mut [Complex],
    rows: usize,
    cols: usize,
    inverse: bool,
) -> Result<(), KernelError> {
    if data.len() != rows * cols {
        return Err(KernelError::Shape {
            rows,
            cols,
            len: data.len(),
        });
    }
    check_len(rows)?;
    check_len(cols)?;
    let row_twiddles = twiddles(cols, inverse);
    for row in data.chunks_exact_mut(cols) {
        transform(row, &row_twiddles);
    }
    let col_twiddles = twiddles(rows, inverse);
    let mut column = vec![Complex::ZERO; rows];
    for c in 0..cols {
        for (r, slot) in column.iter_mut().enumerate() {
            *slot = data[r * cols + c];
        }
        transform(&mut column, &col_twiddles);
        for (r, value) in column.iter().enumerate() {
            data[r * cols + c] = *value;
        }
    }
    if inverse {
        scale_all(data, rows * cols);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::num::{max, to_f64};
    use crate::rng::StageRng;

    fn random(n: usize, key: &str) -> Vec<Complex> {
        let mut rng = StageRng::for_key(1, key);
        (0..n)
            .map(|_| Complex::new(rng.normal(), rng.normal()))
            .collect()
    }

    fn max_diff(a: &[Complex], b: &[Complex]) -> f64 {
        a.iter().zip(b).fold(0.0, |acc, (x, y)| {
            max(acc, max((x.re - y.re).abs(), (x.im - y.im).abs()))
        })
    }

    fn dft(x: &[Complex]) -> Vec<Complex> {
        let n = x.len();
        (0..n)
            .map(|k| {
                let mut sum = Complex::ZERO;
                for (j, &v) in x.iter().enumerate() {
                    let angle = -2.0 * core::f64::consts::PI * to_f64(k * j % n) / to_f64(n);
                    sum = sum + v * Complex::from_polar(1.0, angle);
                }
                sum
            })
            .collect()
    }

    #[test]
    fn fft_matches_the_direct_dft() {
        let input = random(8, "t");
        let mut out = input.clone();
        fft(&mut out, false).unwrap();
        assert!(max_diff(&out, &dft(&input)) < 1e-12);
    }

    #[test]
    fn inverse_fft_restores_the_input() {
        let input = random(64, "inv");
        let mut data = input.clone();
        fft(&mut data, false).unwrap();
        fft(&mut data, true).unwrap();
        assert!(max_diff(&data, &input) < 1e-12);

        let input = random(16 * 32, "inv2");
        let mut data = input.clone();
        fft2(&mut data, 16, 32, false).unwrap();
        fft2(&mut data, 16, 32, true).unwrap();
        assert!(max_diff(&data, &input) < 1e-12);
    }

    #[test]
    fn fft2_matches_the_direct_2d_dft() {
        let (rows, cols) = (4, 8);
        let input = random(rows * cols, "2d");
        let mut out = input.clone();
        fft2(&mut out, rows, cols, false).unwrap();
        let tau = 2.0 * core::f64::consts::PI;
        for u in 0..rows {
            for v in 0..cols {
                let mut sum = Complex::ZERO;
                for r in 0..rows {
                    for c in 0..cols {
                        let phase = to_f64(u * r) / to_f64(rows) + to_f64(v * c) / to_f64(cols);
                        sum = sum + input[r * cols + c] * Complex::from_polar(1.0, -tau * phase);
                    }
                }
                let got = out[u * cols + v];
                assert!((got.re - sum.re).abs() < 1e-11 && (got.im - sum.im).abs() < 1e-11);
            }
        }
    }

    #[test]
    fn fft_of_an_impulse_is_flat() {
        let mut data = vec![Complex::ZERO; 16];
        data[0] = Complex::new(1.0, 0.0);
        fft(&mut data, false).unwrap();
        assert!(
            data.iter()
                .all(|v| (v.re - 1.0).abs() < 1e-15 && v.im.abs() < 1e-15)
        );
    }

    #[test]
    fn a_length_that_is_not_a_power_of_two_is_an_error() {
        let mut data = vec![Complex::ZERO; 6];
        assert_eq!(
            fft(&mut data, false),
            Err(KernelError::NotPowerOfTwo { len: 6 })
        );
        assert!(fft(&mut [], false).is_err());
        let mut data = vec![Complex::ZERO; 12];
        assert!(fft2(&mut data, 3, 4, false).is_err());
        let mut data = vec![Complex::ZERO; 15];
        assert_eq!(
            fft2(&mut data, 4, 4, false),
            Err(KernelError::Shape {
                rows: 4,
                cols: 4,
                len: 15
            })
        );
    }

    #[test]
    fn next_pow2_rounds_up() {
        assert_eq!(
            [0, 1, 2, 3, 37, 64, 65].map(next_pow2),
            [1, 1, 2, 4, 64, 64, 128]
        );
    }

    #[test]
    fn fft_sizes_the_recipes_use_round_trip() {
        for side in [64_usize, 512, 1024] {
            let input = random(side * side, "sizes");
            let mut data = input.clone();
            fft2(&mut data, side, side, false).unwrap();
            fft2(&mut data, side, side, true).unwrap();
            assert!(max_diff(&data, &input) < 1e-10, "{side}");
        }
    }
}
