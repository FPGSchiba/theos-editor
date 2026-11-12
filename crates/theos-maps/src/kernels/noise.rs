//! Spectral 1/f^beta noise and domain-warped coordinates.

use super::complex::Complex;
use super::fft::{fft2, next_pow2};
use super::num::{canonical, to_f64};
use crate::grid::Grid;
use crate::rng::StageRng;

/// The signed frequency index of `i` in a length-`n` axis: `i` below `n / 2`, else `i - n`.
fn signed_freq(i: usize, n: usize) -> f64 {
    if i < n / 2 { to_f64(i) } else { -to_f64(n - i) }
}

/// Fractal 1/f^`beta` noise by spectral synthesis, zero mean and unit population variance
/// (all zeros when the variance is zero).
///
/// The field is synthesised on the padded power-of-two grid and cropped to `rows x cols`.
/// Wavenumbers `k` are in cycles per map; the amplitude is `k^(-beta / 2)` for
/// `kmin <= k <= kmax` and zero elsewhere and at `k == 0`. The complex coefficients are drawn
/// from `rng` in row-major order over the padded grid, real part first, for every cell, so the
/// stream consumed does not depend on `beta`, `kmin` or `kmax`.
#[must_use]
pub fn spectral_noise(
    rows: usize,
    cols: usize,
    rng: &mut StageRng,
    beta: f64,
    kmin: f64,
    kmax: Option<f64>,
) -> Grid<f64> {
    let (p, q) = (next_pow2(rows), next_pow2(cols));
    let mut spectrum = Vec::with_capacity(p * q);
    for i in 0..p {
        let ky = signed_freq(i, p) * to_f64(rows) / to_f64(p);
        for j in 0..q {
            let kx = signed_freq(j, q) * to_f64(cols) / to_f64(q);
            let k = (kx * kx + ky * ky).sqrt();
            let on = k != 0.0 && k >= kmin && kmax.is_none_or(|top| k <= top);
            let amplitude = if on { libm::pow(k, -beta / 2.0) } else { 0.0 };
            let re = rng.normal();
            let im = rng.normal();
            spectrum.push(Complex::new(amplitude * re, amplitude * im));
        }
    }
    // Both lengths are powers of two by construction, so this cannot fail.
    if fft2(&mut spectrum, p, q, true).is_err() {
        return Grid::filled(rows, cols, 0.0);
    }
    let mut values = Vec::with_capacity(rows * cols);
    for r in 0..rows {
        values.extend(spectrum[r * q..r * q + cols].iter().map(|v| v.re));
    }
    normalise(&mut values);
    Grid::from_vec(rows, cols, values).unwrap_or_else(|_| Grid::filled(rows, cols, 0.0))
}

/// Subtracts the mean and divides by the population standard deviation (zeros when it is 0).
fn normalise(values: &mut [f64]) {
    if values.is_empty() {
        return;
    }
    let n = to_f64(values.len());
    let mean = values.iter().sum::<f64>() / n;
    let var = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n;
    let std = var.sqrt();
    for v in values {
        *v = if std > 0.0 {
            canonical((*v - mean) / std)
        } else {
            0.0
        };
    }
}

/// Domain-warped coordinates: the cell-centre fractions `((c + 0.5) / cols, (r + 0.5) / rows)`
/// plus `strength` times a [`spectral_noise`] field (x from `rng_x`, y from `rng_y`; `kmin = 0`,
/// no `kmax`). Returns `(x, y)`.
#[must_use]
pub fn warped_coords(
    rows: usize,
    cols: usize,
    rng_x: &mut StageRng,
    rng_y: &mut StageRng,
    strength: f64,
    beta: f64,
) -> (Grid<f64>, Grid<f64>) {
    let noise_x = spectral_noise(rows, cols, rng_x, beta, 0.0, None);
    let noise_y = spectral_noise(rows, cols, rng_y, beta, 0.0, None);
    let mut x = Grid::filled(rows, cols, 0.0);
    let mut y = Grid::filled(rows, cols, 0.0);
    for r in 0..rows {
        for c in 0..cols {
            let fx = (to_f64(c) + 0.5) / to_f64(cols);
            let fy = (to_f64(r) + 0.5) / to_f64(rows);
            x.set(r, c, fx + strength * noise_x.get(r, c));
            y.set(r, c, fy + strength * noise_y.get(r, c));
        }
    }
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(g: &Grid<f64>) -> (f64, f64) {
        let n = to_f64(g.len());
        let mean = g.as_slice().iter().sum::<f64>() / n;
        let var = g
            .as_slice()
            .iter()
            .map(|v| (v - mean) * (v - mean))
            .sum::<f64>()
            / n;
        (mean, var)
    }

    #[test]
    fn noise_is_zero_mean_unit_variance() {
        let mut rng = StageRng::for_key(1, "n");
        let g = spectral_noise(64, 64, &mut rng, 3.0, 0.0, None);
        let (mean, var) = stats(&g);
        assert!(mean.abs() < 1e-12, "{mean}");
        assert!((var - 1.0).abs() < 1e-9, "{var}");
    }

    #[test]
    fn noise_is_repeatable_for_a_key() {
        let a = spectral_noise(32, 32, &mut StageRng::for_key(2, "n"), 3.0, 1.0, Some(8.0));
        let b = spectral_noise(32, 32, &mut StageRng::for_key(2, "n"), 3.0, 1.0, Some(8.0));
        let c = spectral_noise(32, 32, &mut StageRng::for_key(3, "n"), 3.0, 1.0, Some(8.0));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn noise_on_odd_shapes_is_cropped_and_finite() {
        for (rows, cols) in [(37, 64), (64, 5), (1, 1)] {
            let mut rng = StageRng::for_key(4, "odd");
            let g = spectral_noise(rows, cols, &mut rng, 3.0, 0.0, None);
            assert_eq!((g.rows(), g.cols()), (rows, cols));
            assert!(g.as_slice().iter().all(|v| v.is_finite()));
            if (rows, cols) == (1, 1) {
                assert!(g.as_slice().iter().all(|v| v.to_bits() == 0));
            } else {
                let (mean, var) = stats(&g);
                assert!(mean.abs() < 1e-12 && (var - 1.0).abs() < 1e-9);
            }
        }
    }

    fn mean_laplacian(g: &Grid<f64>) -> f64 {
        let mut sum = 0.0;
        for r in 1..g.rows() - 1 {
            for c in 1..g.cols() - 1 {
                let l = g.get(r - 1, c) + g.get(r + 1, c) + g.get(r, c - 1) + g.get(r, c + 1)
                    - 4.0 * g.get(r, c);
                sum += l.abs();
            }
        }
        sum / to_f64((g.rows() - 2) * (g.cols() - 2))
    }

    #[test]
    fn higher_beta_is_smoother() {
        let rough = spectral_noise(64, 64, &mut StageRng::for_key(5, "b"), 2.0, 0.0, None);
        let smooth = spectral_noise(64, 64, &mut StageRng::for_key(5, "b"), 4.0, 0.0, None);
        assert!(mean_laplacian(&smooth) < mean_laplacian(&rough));
    }

    #[test]
    fn warped_coords_start_at_cell_centres() {
        let mut rx = StageRng::for_key(6, "x");
        let mut ry = StageRng::for_key(6, "y");
        let (x, y) = warped_coords(8, 16, &mut rx, &mut ry, 0.0, 3.5);
        assert!((x.get(3, 5) - 5.5 / 16.0).abs() < 1e-15);
        assert!((y.get(3, 5) - 3.5 / 8.0).abs() < 1e-15);
        let mut rx = StageRng::for_key(6, "x");
        let mut ry = StageRng::for_key(6, "y");
        let (xw, yw) = warped_coords(8, 16, &mut rx, &mut ry, 0.05, 3.5);
        assert_ne!(x, xw);
        assert_ne!(y, yw);
        assert!((xw.get(3, 5) - x.get(3, 5)).abs() < 0.05 * 6.0);
    }
}
