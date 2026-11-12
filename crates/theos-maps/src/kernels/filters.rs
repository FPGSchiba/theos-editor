//! Gaussian, uniform and maximum filters with scipy's `ndimage` semantics (`mode='reflect'`).

use crate::grid::Grid;

use super::num::{max, to_f64, to_index};
use super::reflect::{map_lines, reflect_offset};

/// Kernel radius scipy uses: `int(truncate * sigma + 0.5)` with `truncate = 4.0`.
fn gaussian_radius(sigma_cells: f64) -> usize {
    to_index(4.0 * sigma_cells + 0.5, usize::MAX)
}

/// Normalised half kernel `w[0..=radius]` of `exp(-0.5 x^2 / sigma^2)`.
fn gaussian_weights(sigma_cells: f64, radius: usize) -> Vec<f64> {
    let var = sigma_cells * sigma_cells;
    let mut w: Vec<f64> = (0..=radius)
        .map(|x| {
            let x = to_f64(x);
            libm::exp(-0.5 / var * (x * x))
        })
        .collect();
    let total = w[0] + 2.0 * w[1..].iter().sum::<f64>();
    for v in &mut w {
        *v /= total;
    }
    w
}

/// scipy `gaussian_filter(g, sigma_m / cell_m, mode='reflect', truncate=4.0)`.
///
/// Separable: axis 0, then axis 1. A sigma of zero (or less, or NaN) returns the input unchanged.
#[must_use]
pub fn gaussian(g: &Grid<f64>, sigma_m: f64, cell_m: f64) -> Grid<f64> {
    let sigma = sigma_m / cell_m;
    if sigma.is_nan() || sigma <= 0.0 {
        return g.clone();
    }
    let radius = gaussian_radius(sigma);
    let w = gaussian_weights(sigma, radius);
    let pass = |line: &[f64], out: &mut [f64]| {
        let n = line.len();
        for (i, o) in out.iter_mut().enumerate() {
            let mut acc = line[i] * w[0];
            for (k, wk) in w.iter().enumerate().skip(1) {
                let k = isize::try_from(k).unwrap_or(isize::MAX);
                acc += (line[reflect_offset(i, -k, n)] + line[reflect_offset(i, k, n)]) * wk;
            }
            *o = acc;
        }
    };
    map_lines(&map_lines(g, 0, pass), 1, pass)
}

/// The window offsets of scipy's size-`s` filters with origin 0: `-(s/2) ..= s-1-(s/2)`.
fn window(size: usize) -> impl Iterator<Item = isize> {
    let size = isize::try_from(size).unwrap_or(isize::MAX);
    let lo = -(size / 2);
    lo..(lo + size)
}

/// Reduces a window along axis 0, then axis 1, and maps the result with `finish`.
fn separable(
    g: &Grid<f64>,
    size: usize,
    reduce: impl Fn(f64, f64) -> f64,
    finish: impl Fn(f64) -> f64,
) -> Grid<f64> {
    let pass = |line: &[f64], out: &mut [f64]| {
        let n = line.len();
        for (i, o) in out.iter_mut().enumerate() {
            let mut cells = window(size).map(|k| line[reflect_offset(i, k, n)]);
            let first = cells.next().unwrap_or(0.0);
            *o = finish(cells.fold(first, &reduce));
        }
    };
    map_lines(&map_lines(g, 0, pass), 1, pass)
}

/// scipy `uniform_filter(g, size, mode='reflect')`: the mean of the `size x size` window.
#[must_use]
pub fn uniform(g: &Grid<f64>, size: usize) -> Grid<f64> {
    let count = to_f64(size.max(1));
    separable(g, size, |a, b| a + b, |s| s / count)
}

/// scipy `maximum_filter(g, size, mode='reflect')`: the maximum of the `size x size` window.
#[must_use]
pub fn maximum(g: &Grid<f64>, size: usize) -> Grid<f64> {
    separable(g, size, max, |m| m)
}

#[cfg(test)]
mod tests {
    use super::super::test_data::*;
    use super::*;
    use crate::grid::Grid;

    fn grid(rows: usize, cols: usize, v: &[f64]) -> Grid<f64> {
        Grid::from_vec(rows, cols, v.to_vec()).unwrap()
    }

    #[test]
    fn gaussian_matches_scipy() {
        let ramp: Vec<f64> = (0..35).map(f64::from).collect();
        let out = gaussian(&grid(5, 7, &ramp), 1.0, 1.0);
        assert_close(out.as_slice(), &GAUSS_ARANGE, 1e-12);
        assert_close(
            gaussian(&grid(5, 7, &H), 1.0, 1.0).as_slice(),
            &GAUSS_H_1,
            1e-12,
        );
        // sigma 0.6 cells (1.2 m on 2 m cells): radius int(4 * 0.6 + 0.5) = 2
        assert_close(
            gaussian(&grid(5, 7, &H), 1.2, 2.0).as_slice(),
            &GAUSS_H_06,
            1e-12,
        );
    }

    #[test]
    fn gaussian_wider_than_the_map_reflects_and_stays_finite() {
        let out = gaussian(&grid(4, 4, &S), 10.0, 1.0);
        assert!(out.as_slice().iter().all(|v| v.is_finite()));
        assert_close(out.as_slice(), &GAUSS_S_10, 1e-12);
    }

    #[test]
    fn gaussian_with_zero_sigma_is_the_identity() {
        let g = grid(5, 7, &H);
        assert_eq!(gaussian(&g, 0.0, 1.0), g);
    }

    #[test]
    fn uniform_matches_scipy_for_odd_and_even_sizes() {
        let g = grid(5, 7, &H);
        assert_close(uniform(&g, 3).as_slice(), &UNIFORM_3, 1e-12);
        assert_close(uniform(&g, 4).as_slice(), &UNIFORM_4, 1e-12);
    }

    #[test]
    fn maximum_matches_scipy() {
        let g = grid(5, 7, &H);
        assert_close(maximum(&g, 3).as_slice(), &MAX_3, 1e-12);
        assert_close(maximum(&g, 4).as_slice(), &MAX_4, 1e-12);
    }
}
