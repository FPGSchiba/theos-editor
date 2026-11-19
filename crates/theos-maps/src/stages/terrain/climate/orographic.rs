//! Orographic precipitation by the linear theory of Smith & Barstad (2004), FFT solution.
//!
//! A port of 0.3.0's `orographic_precip`; rows run north to south, columns west to east, and
//! the wind direction is meteorological (the direction it blows from).

use super::{ClimateParams, SECONDS_PER_YEAR};
use crate::grid::Grid;
use crate::kernels::complex::Complex;
use crate::kernels::fft::{fft2, next_pow2};
use crate::kernels::num::{min, sign, to_f64};
use crate::world::MapFrame;
use core::f64::consts::PI;

/// The padded length of an axis of `n` cells: a power of two of at least `1.5 n`.
fn padded_len(n: usize) -> usize {
    next_pow2((3 * n).div_ceil(2))
}

/// `line` with `before` and `after` values added as numpy's `linear_ramp` to 0 does it: the pad
/// nearest to the edge is `edge * (width - 1) / width`, the farthest is 0.
fn ramp_line(line: &[f64], before: usize, after: usize) -> Vec<f64> {
    let (Some(&first), Some(&last)) = (line.first(), line.last()) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(before + line.len() + after);
    out.extend((0..before).map(|k| first * to_f64(k) / to_f64(before)));
    out.extend_from_slice(line);
    out.extend((0..after).map(|j| last * to_f64(after - 1 - j) / to_f64(after)));
    out
}

/// `height` centred in a `P x Q` array (powers of two of at least 1.5 times each side), ramped
/// to 0 as numpy's `pad(mode="linear_ramp")` does: rows first, then the columns of the result.
/// Returns `(P, Q, row-major data)`.
pub(super) fn padded(height: &Grid<f64>) -> (usize, usize, Vec<f64>) {
    let (rows, cols) = (height.rows(), height.cols());
    let (big_rows, big_cols) = (padded_len(rows), padded_len(cols));
    let (top, left) = ((big_rows - rows) / 2, (big_cols - cols) / 2);
    let by_col: Vec<Vec<f64>> = (0..cols)
        .map(|c| {
            let column: Vec<f64> = (0..rows).map(|r| height.get(r, c)).collect();
            ramp_line(&column, top, big_rows - rows - top)
        })
        .collect();
    let mut data = Vec::with_capacity(big_rows * big_cols);
    for r in 0..big_rows {
        let row: Vec<f64> = by_col.iter().map(|col| col[r]).collect();
        data.extend(ramp_line(&row, left, big_cols - cols - left));
    }
    (big_rows, big_cols, data)
}

/// numpy `fftfreq(n)` times `n`: the signed index of bin `i` of `n` (a power of two).
fn signed_index(i: usize, n: usize) -> f64 {
    if i < n / 2 { to_f64(i) } else { -to_f64(n - i) }
}

/// The orographic part of the precipitation in mm/yr, negative in rain shadows.
///
/// Smith & Barstad 2004 with 0.3.0's branch rule for the vertical wavenumber `m`: real
/// `sign(sigma) sqrt|m2|` where `m2 >= 0`, imaginary `i sqrt|m2|` where it is negative.
#[must_use]
pub fn orographic(height: &Grid<f64>, frame: &MapFrame, p: &ClimateParams) -> Grid<f64> {
    let (rows, cols) = (height.rows(), height.cols());
    let theta = p.wind_from_deg * PI / 180.0;
    let u = -p.wind_speed * libm::sin(theta);
    let v = p.wind_speed * libm::cos(theta);
    let floor = height
        .as_slice()
        .iter()
        .fold(f64::INFINITY, |a, &h| min(a, h));
    let (big_rows, big_cols, data) = padded(&height.map(|h| h - floor));
    let mut spectrum: Vec<Complex> = data.into_iter().map(|h| Complex::new(h, 0.0)).collect();
    let flat = || Grid::filled(rows, cols, 0.0);
    if fft2(&mut spectrum, big_rows, big_cols, false).is_err() {
        // Unreachable: the padded sides are powers of two and `spectrum` has their product.
        return flat();
    }
    let wavenumber =
        |i: usize, n: usize| 2.0 * PI * signed_index(i, n) / (to_f64(n) * frame.cell_m);
    for (idx, cell) in spectrum.iter_mut().enumerate() {
        let kx = wavenumber(idx % big_cols, big_cols);
        let ky = wavenumber(idx / big_cols, big_rows);
        *cell = response(p, u * kx + v * ky, kx * kx + ky * ky) * *cell;
    }
    if fft2(&mut spectrum, big_rows, big_cols, true).is_err() {
        return flat();
    }
    let (top, left) = ((big_rows - rows) / 2, (big_cols - cols) / 2);
    let mut out = Vec::with_capacity(rows * cols);
    for r in 0..rows {
        for c in 0..cols {
            out.push(spectrum[(top + r) * big_cols + left + c].re * SECONDS_PER_YEAR);
        }
    }
    Grid::from_vec(rows, cols, out).unwrap_or_else(|_| flat())
}

/// The transfer function from terrain to precipitation at one wavenumber:
/// `cw i sigma / ((1 - i m hw)(1 + i sigma tau_c)(1 + i sigma tau_f))`.
fn response(p: &ClimateParams, sigma: f64, k2: f64) -> Complex {
    let sig2 = sigma * sigma + 1e-30;
    let m2 = (p.nm * p.nm - sig2) / sig2 * k2;
    let root = libm::sqrt(m2.abs());
    let m = if m2 >= 0.0 {
        Complex::new(sign(sigma) * root, 0.0)
    } else {
        Complex::new(0.0, root)
    };
    let i = Complex::new(0.0, 1.0);
    let one = Complex::new(1.0, 0.0);
    let lift = one - (i * m).scale(p.hw);
    let slow_c = one + i.scale(sigma * p.tau_c);
    let slow_f = one + i.scale(sigma * p.tau_f);
    i.scale(p.cw * sigma) / (lift * slow_c * slow_f)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::stages::terrain::climate::ClimateParams;
    use crate::world::MapFrame;

    pub(in crate::stages::terrain::climate) fn hill(n: usize, sigma: f64, top: f64) -> Grid<f64> {
        let mid = to_f64(n) / 2.0;
        let v = (0..n * n)
            .map(|i| {
                let (dr, dc) = (to_f64(i / n) - mid, to_f64(i % n) - mid);
                top * libm::exp(-(dr * dr + dc * dc) / (2.0 * sigma * sigma))
            })
            .collect();
        Grid::from_vec(n, n, v).unwrap()
    }

    fn frame(n: usize) -> MapFrame {
        MapFrame {
            rows: n,
            cols: n,
            cell_m: 1000.0,
        }
    }

    #[test]
    fn smith_barstad_wets_the_windward_side() {
        let h = hill(64, 6.0, 1000.0);
        let oro = orographic(&h, &frame(64), &ClimateParams::default());
        let side = |cols: std::ops::Range<usize>| {
            let mut sum = 0.0;
            let mut n = 0.0;
            for r in 0..64 {
                for c in cols.clone() {
                    sum += oro.get(r, c);
                    n += 1.0;
                }
            }
            sum / n
        };
        assert!(side(0..32) > 0.0, "west {}", side(0..32));
        assert!(side(33..64) < 0.0, "east {}", side(33..64));
    }

    #[test]
    fn wind_from_the_east_flips_the_pattern() {
        let h = hill(64, 6.0, 1000.0);
        let p = ClimateParams {
            wind_from_deg: 90.0,
            ..ClimateParams::default()
        };
        let oro = orographic(&h, &frame(64), &p);
        let west: f64 = (0..64).map(|r| oro.get(r, 10)).sum();
        let east: f64 = (0..64).map(|r| oro.get(r, 54)).sum();
        assert!(east > 0.0 && west < 0.0, "{west} {east}");
    }

    #[test]
    fn a_flat_surface_has_no_orographic_part() {
        let h = Grid::filled(32, 32, 250.0);
        let oro = orographic(&h, &frame(32), &ClimateParams::default());
        assert!(oro.as_slice().iter().all(|v| v.abs() < 1e-6));
    }

    #[test]
    fn a_calm_has_no_orographic_part() {
        let h = hill(32, 4.0, 800.0);
        let p = ClimateParams {
            wind_speed: 0.0,
            ..ClimateParams::default()
        };
        let oro = orographic(&h, &frame(32), &p);
        assert!(oro.as_slice().iter().all(|v| v.abs() < 1e-6));
    }

    #[test]
    fn non_square_odd_grids_work_and_keep_their_shape() {
        let v = (0..21 * 33).map(|i| f64::from(i % 7) * 40.0).collect();
        let h = Grid::from_vec(21, 33, v).unwrap();
        let f = MapFrame {
            rows: 21,
            cols: 33,
            cell_m: 1000.0,
        };
        let oro = orographic(&h, &f, &ClimateParams::default());
        assert_eq!((oro.rows(), oro.cols()), (21, 33));
        assert!(oro.as_slice().iter().all(|v| v.is_finite()));
    }

    #[test]
    fn padding_is_centred_with_numpys_linear_ramp() {
        let h = Grid::filled(4, 4, 2.0);
        let (p, q, big) = padded(&h);
        assert_eq!((p, q), (8, 8));
        let at = |r: usize, c: usize| big[r * 8 + c];
        assert!(at(0, 3).abs() < 1e-12 && at(0, 0).abs() < 1e-12);
        assert!((at(1, 1) - 0.5).abs() < 1e-12);
        assert!((at(3, 1) - 1.0).abs() < 1e-12);
        assert!((at(2, 2) - 2.0).abs() < 1e-12 && (at(5, 5) - 2.0).abs() < 1e-12);
        assert!((at(6, 6) - 0.5).abs() < 1e-12 && at(7, 7).abs() < 1e-12);
    }

    #[test]
    fn frequencies_are_the_signed_fft_indices() {
        let f: Vec<f64> = (0..8).map(|i| signed_index(i, 8)).collect();
        assert_eq!(f, [0.0, 1.0, 2.0, 3.0, -4.0, -3.0, -2.0, -1.0]);
        assert!(sign(-2.0) < 0.0 && sign(3.0) > 0.0 && sign(0.0).abs() < 1e-300);
    }
}
