//! Spectral 1/f^beta noise and domain-warped coordinates.
//!
//! The noise is a sum of plane waves on a wavenumber lattice that the map fixes, not the grid.
//! Each lattice point's coefficient comes from a counter-based stream keyed by the point, so
//! every grid of the same map sees the same coefficient for every wavenumber it resolves: one key
//! gives one landscape at every grid size, and a finer grid only adds the wavenumbers above the
//! coarser grid's Nyquist (spec §5.8, physics test 13).
//!
//! - **Lattice:** the point `(a, b)` (integers, `a` along columns, `b` along rows) is the
//!   wavenumber `(kx, ky) = (3/4) (a, b)` cycles per map: `kx` per map width, `ky` per map height,
//!   `k = sqrt(kx² + ky²)`. The noise repeats over 4/3 of the map, so opposite edges do not
//!   match. 3/4 is the step that 0.3.0's padding to the next power of two gives at 192, 384 and
//!   768 cells.
//! - **Band:** a grid of `rows x cols` cells holds the points up to its Nyquist wavenumber,
//!   `|kx| <= cols / 2` and `|ky| <= rows / 2`: `|a| <= floor(2 cols / 3)`,
//!   `|b| <= floor(2 rows / 3)`.
//! - **Coefficients:** `Z(a, b)` is a standard complex normal (real and imaginary parts
//!   independent N(0, 1)) drawn by `coefficient` from the call's key, one draw of the caller's
//!   stream. The amplitude is `k^(-beta / 2)` for `kmin <= k <= kmax`, zero elsewhere and at
//!   `k = 0`.
//! - **Field:** the value of cell `(r, c)` is the real part of
//!   `sum over the band of A(k) Z(a, b) e^(2 pi i (kx x + ky y))` at the cell centre
//!   `x = (c + 1/2) / cols`, `y = (r + 1/2) / rows`, evaluated per axis by a chirp-z transform
//!   (`Axis`). The values are then shifted and scaled to zero mean and unit population
//!   variance over the grid (`normalise`).

use std::num::NonZeroUsize;

use super::complex::Complex;
use super::czt::Czt;
use super::num::{canonical, to_f64, unit_from_bits};
use crate::grid::Grid;
use crate::rng::{SplitMix64, StageRng, fnv1a64};

/// The lattice step in cycles per map is `STEP_NUM / STEP_DEN`.
const STEP_NUM: usize = 3;
/// See [`STEP_NUM`].
const STEP_DEN: usize = 4;

/// Fractal 1/f^`beta` noise by spectral synthesis on the map's wavenumber lattice (module
/// docs), zero mean and unit population variance (all zeros when the variance is zero).
///
/// Wavenumbers, `kmin` and `kmax` are in cycles per map. The call takes one draw from `rng`, the
/// key of its coefficients, whatever the grid, `beta`, `kmin` or `kmax`.
#[must_use]
pub fn spectral_noise(
    rows: usize,
    cols: usize,
    rng: &mut StageRng,
    beta: f64,
    kmin: f64,
    kmax: Option<f64>,
) -> Grid<f64> {
    let key = rng.next_u64();
    let mut values = lattice_sum(rows, cols, key, beta, kmin, kmax);
    normalise(&mut values);
    Grid::from_vec(rows, cols, values).unwrap_or_else(|_| Grid::filled(rows, cols, 0.0))
}

/// The largest lattice index an axis of `cells` cells holds: its Nyquist wavenumber,
/// `cells / 2` cycles per map, over the step, rounded down.
fn reach(cells: usize) -> usize {
    cells * STEP_DEN / (2 * STEP_NUM)
}

/// The lattice index `i - origin` as a 64-bit two's complement.
fn signed(i: usize, origin: usize) -> u64 {
    let wide = |n: usize| u64::try_from(n).unwrap_or(u64::MAX);
    wide(i).wrapping_sub(wide(origin))
}

/// The coefficient `Z(a, b)` of the lattice point `(a, b)` under `key`, counter-based: a
/// [`SplitMix64`] seeded with `key ^ fnv1a64(bytes)`, where `bytes` are `a` then `b` as 64-bit
/// two's complement, little-endian, gives `u1` and `u2` from its first two outputs (top 53 bits).
/// Box-Muller on both outputs: radius `sqrt(-2 ln(1 - u1))`, angle `2 pi u2`, so the real and
/// imaginary parts are independent N(0, 1).
fn coefficient(key: u64, a: u64, b: u64) -> Complex {
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&a.to_le_bytes());
    bytes[8..].copy_from_slice(&b.to_le_bytes());
    let mut mix = SplitMix64::new(key ^ fnv1a64(&bytes));
    let u1 = 1.0 - unit_from_bits(mix.next());
    let u2 = unit_from_bits(mix.next());
    let radius = (-2.0 * libm::log(u1)).sqrt();
    Complex::from_polar(radius, 2.0 * core::f64::consts::PI * u2)
}

/// The plane waves of one axis of `cells` cells, at its cell centres.
///
/// `sum(y)[m] = sum over j <= 2 reach of y[j] e^(2 pi i (3/4) (j - reach) (m + 1/2) / cells)`,
/// where `y[j]` belongs to the lattice index `j - reach`. With `d = 8 cells` the angle is
/// `3 (j - reach) (2m + 1) / d` turns, `= 6 j m / d + 3 j / d - 3 reach (2m + 1) / d`: a chirp-z
/// transform at the step `6 / d` between the factors `e^(2 pi i 3 j / d)` and
/// `e^(-2 pi i 3 reach (2m + 1) / d)`, every angle reduced in integers.
struct Axis {
    czt: Czt,
    pre: Vec<Complex>,
    post: Vec<Complex>,
}

impl Axis {
    /// The axis of `cells` cells; `None` for no cells.
    fn new(cells: usize) -> Option<Self> {
        let reach = reach(cells);
        let lattice = 2 * reach + 1;
        let d = NonZeroUsize::new(2 * STEP_DEN * cells)?;
        Some(Self {
            czt: Czt::new(lattice, cells, 2 * STEP_NUM, d),
            pre: (0..lattice)
                .map(|j| Complex::from_turns(STEP_NUM * j, d))
                .collect(),
            post: (0..cells)
                .map(|m| Complex::from_turns(STEP_NUM * reach * (2 * m + 1), d).conj())
                .collect(),
        })
    }

    /// Writes `sum(y)` into `out` (both of the planned lengths).
    fn sum(&self, y: &[Complex], out: &mut [Complex]) {
        let shifted: Vec<Complex> = y.iter().zip(&self.pre).map(|(v, p)| *v * *p).collect();
        // `shifted` has the planned length, so the transform cannot refuse it.
        let waves = self.czt.apply(&shifted).unwrap_or_default();
        for (slot, (v, p)) in out.iter_mut().zip(waves.iter().zip(&self.post)) {
            *slot = *v * *p;
        }
    }
}

/// The real part of the lattice sum at every cell centre, row-major, before [`normalise`]:
/// first along the columns for every lattice row `b`, then along the rows for every column. A
/// lattice row without a coefficient is skipped.
fn lattice_sum(
    rows: usize,
    cols: usize,
    key: u64,
    beta: f64,
    kmin: f64,
    kmax: Option<f64>,
) -> Vec<f64> {
    let (Some(along_x), Some(along_y)) = (Axis::new(cols), Axis::new(rows)) else {
        return Vec::new();
    };
    let (reach_x, reach_y) = (reach(cols), reach(rows));
    let step = to_f64(STEP_NUM) / to_f64(STEP_DEN);
    let mut partial = vec![Complex::ZERO; (2 * reach_y + 1) * cols];
    let mut line = vec![Complex::ZERO; 2 * reach_x + 1];
    for (ib, out) in partial.chunks_exact_mut(cols).enumerate() {
        let db = ib.abs_diff(reach_y);
        let mut any = false;
        for (ia, slot) in line.iter_mut().enumerate() {
            let da = ia.abs_diff(reach_x);
            let k = step * to_f64(da * da + db * db).sqrt();
            let on = da + db > 0 && k >= kmin && kmax.is_none_or(|top| k <= top);
            *slot = if on {
                any = true;
                let z = coefficient(key, signed(ia, reach_x), signed(ib, reach_y));
                z.scale(libm::pow(k, -beta / 2.0))
            } else {
                Complex::ZERO
            };
        }
        if any {
            along_x.sum(&line, out);
        }
    }
    let mut values = vec![0.0; rows * cols];
    let mut column = vec![Complex::ZERO; 2 * reach_y + 1];
    let mut waves = vec![Complex::ZERO; rows];
    for c in 0..cols {
        for (slot, row) in column.iter_mut().zip(partial.chunks_exact(cols)) {
            *slot = row[c];
        }
        along_y.sum(&column, &mut waves);
        for (r, v) in waves.iter().enumerate() {
            values[r * cols + c] = v.re;
        }
    }
    values
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
    use crate::kernels::num::to_f64_i64;

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
    fn coefficients_follow_the_documented_hash() {
        // An independent Python of the documented rule (FNV-1a 64 of a and b as little-endian
        // i64, SplitMix64, top 53 bits, Box-Muller on both outputs), the task-43b scratchpad's
        // `t43b_coef.py`:
        //   coefficient(1, -1, 2)          = 0.4496760737703438, -0.27343984717519293
        //   coefficient(0xDEADBEEF, 5, -3) = 0.7641374117752563, 1.9318543026918396
        let cases = [
            (
                (1, -1, 2),
                (0.449_676_073_770_343_8, -0.273_439_847_175_192_93),
            ),
            (
                (0xDEAD_BEEF, 5, -3),
                (0.764_137_411_775_256_3, 1.931_854_302_691_839_6),
            ),
        ];
        for ((key, a, b), (re, im)) in cases {
            let z = coefficient(key, i64::cast_unsigned(a), i64::cast_unsigned(b));
            assert!(
                (z.re - re).abs() < 1e-15 && (z.im - im).abs() < 1e-15,
                "{z:?}"
            );
        }
    }

    // The synthesis is the lattice sum of plane waves at the cell centres, term by term: on 5 x 7
    // cells the band is |a| <= 4 (kx <= 3 of the 3.5 cycles the columns resolve) and |b| <= 3.
    #[test]
    fn the_field_is_the_lattice_sum_at_the_cell_centres() {
        let (rows, cols, key, beta) = (5, 7, 11, 3.0);
        assert_eq!((reach(cols), reach(rows)), (4, 3));
        let got = lattice_sum(rows, cols, key, beta, 0.0, None);
        let tau = 2.0 * core::f64::consts::PI;
        for r in 0..rows {
            for c in 0..cols {
                let x = (to_f64(c) + 0.5) / to_f64(cols);
                let y = (to_f64(r) + 0.5) / to_f64(rows);
                let mut want = 0.0;
                for a in -4_i64..=4 {
                    for b in -3_i64..=3 {
                        if a == 0 && b == 0 {
                            continue;
                        }
                        let (kx, ky) = (0.75 * to_f64_i64(a), 0.75 * to_f64_i64(b));
                        let z = coefficient(key, a.cast_unsigned(), b.cast_unsigned());
                        let amplitude = libm::pow((kx * kx + ky * ky).sqrt(), -beta / 2.0);
                        let angle = tau * (kx * x + ky * y);
                        want += amplitude * (z.re * libm::cos(angle) - z.im * libm::sin(angle));
                    }
                }
                let at = got[r * cols + c];
                assert!((at - want).abs() < 1e-12, "({r}, {c}): {at} against {want}");
            }
        }
    }

    // kmax below the first lattice ring (3/4 cycle per map) leaves no wave: all zeros.
    #[test]
    fn a_band_below_the_lattice_gives_zeros() {
        let mut rng = StageRng::for_key(1, "low");
        let g = spectral_noise(16, 16, &mut rng, 3.0, 0.0, Some(0.5));
        assert!(g.as_slice().iter().all(|v| v.to_bits() == 0));
        assert!(spectral_noise(0, 4, &mut rng, 3.0, 0.0, None).is_empty());
    }

    /// The means of `g` over blocks of `f x f` cells (the sides are multiples of `f`).
    fn block_mean(g: &Grid<f64>, f: usize) -> Grid<f64> {
        let (rows, cols) = (g.rows() / f, g.cols() / f);
        let mut out = Grid::filled(rows, cols, 0.0);
        for r in 0..rows {
            for c in 0..cols {
                let mut sum = 0.0;
                for i in 0..f {
                    for j in 0..f {
                        sum += g.get(r * f + i, c * f + j);
                    }
                }
                out.set(r, c, sum / to_f64(f * f));
            }
        }
        out
    }

    /// The root-mean-square difference of two grids of one shape.
    fn rms_diff(a: &Grid<f64>, b: &Grid<f64>) -> f64 {
        let sum: f64 = a
            .as_slice()
            .iter()
            .zip(b.as_slice())
            .map(|(x, y)| (x - y) * (x - y))
            .sum();
        (sum / to_f64(a.len())).sqrt()
    }

    /// Pearson's correlation of two grids of one shape.
    fn correlation(a: &Grid<f64>, b: &Grid<f64>) -> f64 {
        let ((ma, va), (mb, vb)) = (stats(a), stats(b));
        let cov = a
            .as_slice()
            .iter()
            .zip(b.as_slice())
            .map(|(x, y)| (x - ma) * (y - mb))
            .sum::<f64>()
            / to_f64(a.len());
        cov / (va * vb).sqrt()
    }

    // The wavenumber lattice is fixed by the map, not the grid (spec §5.8): one key at two grid
    // sizes of one map gives one landscape, the finer grid only adds detail. Seen at blocks of 4
    // coarse cells (the fine grid's blocks cover the same ground), the two differ by less than
    // 0.05 of the unit deviation (rms) and correlate above 0.9995. Measured (task 43b): rms at
    // most 0.029 and r at least 0.99991 over these cases; unrelated fields (the noise before
    // 43b) gave rms 1.20 and r 0.05.
    #[test]
    fn a_key_gives_one_landscape_at_every_grid_size() {
        for ((rows, cols), factor) in [((48, 72), 2), ((64, 64), 3), ((40, 24), 4)] {
            for beta in [2.6, 3.0, 3.4, 4.0] {
                let mut rng = StageRng::for_key(8, "sizes");
                let coarse = spectral_noise(rows, cols, &mut rng, beta, 0.0, None);
                let mut rng = StageRng::for_key(8, "sizes");
                let fine = spectral_noise(rows * factor, cols * factor, &mut rng, beta, 0.0, None);
                let (a, b) = (block_mean(&coarse, 4), block_mean(&fine, 4 * factor));
                let (rms, r) = (rms_diff(&a, &b), correlation(&a, &b));
                let at = format!("{rows}x{cols} x{factor}, beta {beta}: rms {rms}, r {r}");
                assert!(rms < 0.05, "{at}");
                assert!(r > 0.9995, "{at}");
            }
        }
    }

    /// The Hann-windowed periodogram of the square power-of-two grid `g`: per FFT bin, its
    /// wavenumber in cycles per map and its power.
    fn periodogram(g: &Grid<f64>) -> Vec<(f64, f64)> {
        let n = g.rows();
        let hann = |i: usize| {
            0.5 - 0.5 * libm::cos(2.0 * core::f64::consts::PI * (to_f64(i) + 0.5) / to_f64(n))
        };
        let mut data: Vec<Complex> = (0..n * n)
            .map(|i| Complex::new(g.as_slice()[i] * hann(i / n) * hann(i % n), 0.0))
            .collect();
        crate::kernels::fft::fft2(&mut data, n, n, false).unwrap();
        data.iter()
            .enumerate()
            .map(|(i, v)| {
                let (u, w) = (i / n, i % n);
                let (ku, kw) = (to_f64(u.min(n - u)), to_f64(w.min(n - w)));
                ((ku * ku + kw * kw).sqrt(), v.re * v.re + v.im * v.im)
            })
            .collect()
    }

    /// The least-squares slope of log power per mode over log wavenumber: the periodograms of
    /// `fields`, averaged over the bins of each ring of whole cycles per map from `lo` to `hi`.
    fn spectral_slope(fields: &[Grid<f64>], lo: usize, hi: usize) -> f64 {
        let mut power = vec![(0.0, 0.0); hi + 1];
        for g in fields {
            for (k, p) in periodogram(g) {
                let ring = crate::kernels::num::to_index(k + 0.5, hi + 2);
                if (lo..=hi).contains(&ring) {
                    power[ring].0 += p;
                    power[ring].1 += 1.0;
                }
            }
        }
        let points: Vec<(f64, f64)> = (lo..=hi)
            .map(|k| (libm::log(to_f64(k)), libm::log(power[k].0 / power[k].1)))
            .collect();
        let count = to_f64(points.len());
        let mx = points.iter().map(|p| p.0).sum::<f64>() / count;
        let my = points.iter().map(|p| p.1).sum::<f64>() / count;
        let sxy: f64 = points.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
        let sxx: f64 = points.iter().map(|p| (p.0 - mx) * (p.0 - mx)).sum();
        sxy / sxx
    }

    // 1/f^beta: the power per mode falls as k^-beta, in cycles per map.
    #[test]
    fn the_spectrum_falls_as_k_to_the_minus_beta() {
        for beta in [2.0, 2.6, 3.0, 3.4, 4.0] {
            let fields: Vec<Grid<f64>> = (0..8)
                .map(|seed| {
                    let mut rng = StageRng::for_key(seed, "slope");
                    spectral_noise(128, 128, &mut rng, beta, 0.0, None)
                })
                .collect();
            let slope = spectral_slope(&fields, 4, 40);
            assert!((slope + beta).abs() < 0.1, "beta {beta}: slope {slope}");
        }
    }

    // kmin and kmax are in cycles per map: a band from 6 to 12 cycles leaves almost no power
    // below 4 or above 14 cycles per map (the Hann window spreads a ring by up to two cycles).
    #[test]
    fn kmin_and_kmax_bound_the_spectrum_in_cycles_per_map() {
        let mut inside = 0.0;
        let mut outside = 0.0;
        for seed in 0..4 {
            let mut rng = StageRng::for_key(seed, "band");
            let g = spectral_noise(128, 128, &mut rng, 3.0, 6.0, Some(12.0));
            for (k, p) in periodogram(&g) {
                if (6.0..=12.0).contains(&k) {
                    inside += p;
                } else if !(4.0..=14.0).contains(&k) {
                    outside += p;
                }
            }
        }
        assert!(outside < 0.001 * inside, "{outside} against {inside}");
    }

    // A call takes one draw from its stream, at any grid size, so what a stage draws after the
    // noise does not depend on the grid either.
    #[test]
    fn the_noise_takes_one_draw_from_its_stream() {
        for (rows, cols) in [(64, 64), (37, 64), (1, 1)] {
            let mut used = StageRng::for_key(9, "draws");
            let _ = spectral_noise(rows, cols, &mut used, 3.0, 0.0, None);
            let mut skipped = StageRng::for_key(9, "draws");
            skipped.next_u64();
            assert_eq!(used.next_u64(), skipped.next_u64(), "{rows}x{cols}");
        }
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
