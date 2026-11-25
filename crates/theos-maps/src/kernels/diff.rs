//! Second differences, gradients and slope with scipy's and numpy's semantics.

use crate::grid::Grid;

use super::reflect::{map_lines, reflect_offset};

/// A grid of the shape of `like` over `data` (always the right length here).
fn same_shape(like: &Grid<f64>, data: Vec<f64>) -> Grid<f64> {
    Grid::from_vec(like.rows(), like.cols(), data).unwrap_or_else(|_| like.clone())
}

/// scipy `ndimage.laplace(g, mode='reflect') / cell_m^2`: the sum of the second differences
/// along both axes.
#[must_use]
pub fn laplace(g: &Grid<f64>, cell_m: f64) -> Grid<f64> {
    let second = |line: &[f64], out: &mut [f64]| {
        let n = line.len();
        for (i, o) in out.iter_mut().enumerate() {
            *o = line[i] * -2.0 + (line[reflect_offset(i, -1, n)] + line[reflect_offset(i, 1, n)]);
        }
    };
    let down = map_lines(g, 0, second);
    let across = map_lines(g, 1, second);
    let area = cell_m * cell_m;
    let data = down
        .as_slice()
        .iter()
        .zip(across.as_slice())
        .map(|(a, b)| (a + b) / area)
        .collect();
    same_shape(g, data)
}

/// numpy `gradient` of one line: central differences inside, one-sided at the ends, and zeros
/// for a line shorter than two cells (numpy refuses those).
fn line_gradient(line: &[f64], out: &mut [f64], cell_m: f64) {
    let n = line.len();
    if n < 2 {
        out.fill(0.0);
        return;
    }
    out[0] = (line[1] - line[0]) / cell_m;
    for i in 1..n - 1 {
        out[i] = (line[i + 1] - line[i - 1]) / (2.0 * cell_m);
    }
    out[n - 1] = (line[n - 1] - line[n - 2]) / cell_m;
}

/// numpy `gradient(g, cell_m)` as `(d/drow, d/dcol)`.
#[must_use]
pub fn gradient(g: &Grid<f64>, cell_m: f64) -> (Grid<f64>, Grid<f64>) {
    let pass = |line: &[f64], out: &mut [f64]| line_gradient(line, out, cell_m);
    (map_lines(g, 0, pass), map_lines(g, 1, pass))
}

/// The gradient magnitude `sqrt(gr^2 + gc^2)`.
#[must_use]
pub fn slope(g: &Grid<f64>, cell_m: f64) -> Grid<f64> {
    let (gr, gc) = gradient(g, cell_m);
    let data = gr
        .as_slice()
        .iter()
        .zip(gc.as_slice())
        .map(|(r, c)| (r * r + c * c).sqrt())
        .collect();
    same_shape(g, data)
}

#[cfg(test)]
mod tests {
    use super::super::test_data::*;
    use super::*;
    use crate::grid::Grid;

    fn h() -> Grid<f64> {
        Grid::from_vec(5, 7, H.to_vec()).unwrap()
    }

    #[test]
    fn laplace_matches_scipy() {
        assert_close(laplace(&h(), 2.0).as_slice(), &LAPLACE, 1e-12);
    }

    #[test]
    fn gradient_matches_numpy() {
        let (gr, gc) = gradient(&h(), 2.0);
        assert_close(gr.as_slice(), &GRAD_ROW, 1e-12);
        assert_close(gc.as_slice(), &GRAD_COL, 1e-12);
        assert_close(slope(&h(), 2.0).as_slice(), &SLOPE, 1e-12);
    }

    #[test]
    fn a_single_row_or_column_has_zero_gradient_across_it() {
        let g = Grid::from_vec(1, 3, vec![1.0, 5.0, 2.0]).unwrap();
        let (gr, gc) = gradient(&g, 1.0);
        assert_close(gr.as_slice(), &[0.0; 3], 1e-12);
        assert_close(gc.as_slice(), &[4.0, 0.5, -3.0], 1e-12);
        // by hand with reflect: 1-2+5-... = 4, -7, 3
        let t = Grid::from_vec(3, 1, vec![1.0, 5.0, 2.0]).unwrap();
        assert_close(laplace(&t, 1.0).as_slice(), &[4.0, -7.0, 3.0], 1e-12);
    }
}
