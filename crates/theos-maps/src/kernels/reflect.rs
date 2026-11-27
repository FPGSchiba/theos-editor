//! scipy's `reflect` boundary (`d c b a | a b c d | d c b a`) and a helper that maps lines of a
//! grid.

use crate::grid::Grid;

/// The in-range index scipy's `mode='reflect'` reads for index `i` of a line of `n` cells.
///
/// Works for any `i`, however many periods outside the line. `0` for an empty line.
#[must_use]
pub fn reflect_index(i: isize, n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let period = 2 * n;
    let wide = isize::try_from(period).unwrap_or(isize::MAX);
    let m = usize::try_from(i.rem_euclid(wide)).unwrap_or(0);
    if m < n { m } else { period - 1 - m }
}

/// Index `i + k` of a line of `n` cells, reflected.
#[must_use]
pub(crate) fn reflect_offset(i: usize, k: isize, n: usize) -> usize {
    let i = isize::try_from(i).unwrap_or(isize::MAX);
    reflect_index(i.saturating_add(k), n)
}

/// Applies `f(line, out)` to every column (`axis == 0`, lines run down the rows) or every row
/// (`axis == 1`) of `g` and returns the grid of the outputs; `line` and `out` have the length of
/// that axis.
#[must_use]
pub(crate) fn map_lines(g: &Grid<f64>, axis: usize, f: impl Fn(&[f64], &mut [f64])) -> Grid<f64> {
    let (rows, cols) = (g.rows(), g.cols());
    let mut out = Grid::filled(rows, cols, 0.0);
    if axis == 1 {
        for r in 0..rows {
            let span = r * cols..(r + 1) * cols;
            f(&g.as_slice()[span.clone()], &mut out.as_mut_slice()[span]);
        }
    } else {
        let mut line = vec![0.0; rows];
        let mut dst = vec![0.0; rows];
        for c in 0..cols {
            for (r, v) in line.iter_mut().enumerate() {
                *v = g.get(r, c);
            }
            f(&line, &mut dst);
            for (r, v) in dst.iter().enumerate() {
                out.set(r, c, *v);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflect_index_wraps_more_than_one_period() {
        // np.pad(arange(4), 20, mode='symmetric') at -12..=12
        let expected = [
            3, 2, 1, 0, 0, 1, 2, 3, 3, 2, 1, 0, 0, 1, 2, 3, 3, 2, 1, 0, 0, 1, 2, 3, 3,
        ];
        for (k, e) in expected.iter().enumerate() {
            let i = isize::try_from(k).unwrap() - 12;
            assert_eq!(reflect_index(i, 4), *e, "i = {i}");
        }
        assert_eq!(reflect_index(-9, 4), 0);
        assert_eq!(reflect_index(12, 4), 3);
        assert_eq!(reflect_index(5, 0), 0);
    }
}
