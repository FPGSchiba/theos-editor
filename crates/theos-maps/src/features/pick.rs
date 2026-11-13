//! Siting helpers: the interior of a frame and greedy picking by score (0.3.0's `_interior` and
//! `_pick`).

use crate::grid::Grid;
use crate::kernels::num::{to_f64, to_index};
use crate::world::MapFrame;

/// The cells at least `edge_margin` of the shorter side away from every edge: the margin is
/// `round(edge_margin x shorter side in cells)` cells on each side, halves to even.
#[must_use]
pub fn interior(frame: &MapFrame, edge_margin: f64) -> Grid<bool> {
    let shorter = to_f64(frame.rows.min(frame.cols));
    let margin = to_index((edge_margin * shorter).round_ties_even(), usize::MAX);
    let inside = |i: usize, len: usize| i >= margin && i < len.saturating_sub(margin);
    let mut out = Grid::filled(frame.rows, frame.cols, false);
    for (i, cell) in out.as_mut_slice().iter_mut().enumerate() {
        *cell = inside(i / frame.cols, frame.rows) && inside(i % frame.cols, frame.cols);
    }
    out
}

/// Sets every score within `sep_m` (strictly) of the point `(x, y)` in metres to 0.
fn clear_near(score: &mut Grid<f64>, frame: &MapFrame, at: [f64; 2], sep_m: f64) {
    let limit = sep_m * sep_m;
    for i in 0..score.len() {
        let (r, c) = score.cell(i);
        let [x, y] = frame.centre_m(r, c);
        let (dx, dy) = (x - at[0], y - at[1]);
        if dx * dx + dy * dy < limit {
            score.as_mut_slice()[i] = 0.0;
        }
    }
}

/// Up to `n` cells `(row, col)` by greedy picking: the highest score first, ties by cell index.
///
/// Cells with a score of 0 or less, or a non-finite one, are never picked. A pick clears the
/// cells within `min_sep_m` of its centre (strictly, as 0.3.0), and the cells within
/// `taken_sep_m` of a point of `taken` (metres) are cleared before the first pick.
#[must_use]
pub fn pick(
    score: &Grid<f64>,
    n: usize,
    min_sep_m: f64,
    taken: &[[f64; 2]],
    taken_sep_m: f64,
    frame: &MapFrame,
) -> Vec<(usize, usize)> {
    let mut left = score.map(|v| if v.is_finite() { v } else { 0.0 });
    for &at in taken {
        clear_near(&mut left, frame, at, taken_sep_m);
    }
    let mut picks = Vec::new();
    while picks.len() < n {
        let mut best: Option<(usize, f64)> = None;
        for (index, &v) in left.as_slice().iter().enumerate() {
            if v > 0.0 && best.is_none_or(|(_, b)| v > b) {
                best = Some((index, v));
            }
        }
        let Some((index, _)) = best else { break };
        let (row, col) = left.cell(index);
        picks.push((row, col));
        left.as_mut_slice()[index] = 0.0;
        clear_near(&mut left, frame, frame.centre_m(row, col), min_sep_m);
    }
    picks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::world::MapFrame;

    const FRAME: MapFrame = MapFrame {
        rows: 10,
        cols: 10,
        cell_m: 100.0,
    };

    fn scores(cells: &[((usize, usize), f64)]) -> Grid<f64> {
        let mut g = Grid::filled(10, 10, 0.0);
        for &((r, c), v) in cells {
            g.set(r, c, v);
        }
        g
    }

    fn dist(a: (usize, usize), b: (usize, usize)) -> f64 {
        let (dr, dc) = (a.0.abs_diff(b.0), a.1.abs_diff(b.1));
        libm::sqrt(crate::kernels::num::to_f64(dr * dr + dc * dc)) * 100.0
    }

    #[test]
    fn pick_keeps_its_spacing_and_ignores_non_positive_scores() {
        let g = scores(&[
            ((2, 2), 5.0),
            ((2, 3), 4.9),
            ((2, 8), 3.0),
            ((8, 5), 2.0),
            ((5, 5), -1.0),
            ((6, 6), 0.0),
            ((7, 7), f64::NAN),
            ((9, 9), f64::INFINITY),
        ]);
        // Infinity is finite-checked like NaN: it counts as 0 (0.3.0 `isfinite`).
        let picks = pick(&g, 10, 350.0, &[], 0.0, &FRAME);
        assert_eq!(picks, [(2, 2), (2, 8), (8, 5)]);
        for (i, a) in picks.iter().enumerate() {
            for b in &picks[i + 1..] {
                assert!(dist(*a, *b) >= 350.0, "{a:?} {b:?}");
            }
        }
        assert_eq!(pick(&g, 2, 350.0, &[], 0.0, &FRAME), [(2, 2), (2, 8)]);
        assert_eq!(pick(&g, 0, 350.0, &[], 0.0, &FRAME), []);
    }

    #[test]
    fn pick_skips_cells_near_taken_points() {
        let g = scores(&[((2, 2), 5.0), ((2, 8), 3.0), ((8, 5), 2.0)]);
        // The centre of cell (2, 8).
        let taken = [FRAME.centre_m(2, 8)];
        let picks = pick(&g, 10, 350.0, &taken, 200.0, &FRAME);
        assert_eq!(picks, [(2, 2), (8, 5)]);
    }

    #[test]
    fn pick_separation_is_strict_and_ties_go_to_the_lowest_index() {
        // Two cells exactly 300 m apart both stand with a separation of 300 m.
        let g = scores(&[((4, 1), 1.0), ((4, 4), 1.0)]);
        assert_eq!(pick(&g, 5, 300.0, &[], 0.0, &FRAME), [(4, 1), (4, 4)]);
        assert_eq!(pick(&g, 5, 300.1, &[], 0.0, &FRAME), [(4, 1)]);
        // Equal scores: the lowest index first.
        let flat = Grid::filled(10, 10, 1.0);
        assert_eq!(pick(&flat, 1, 100.0, &[], 0.0, &FRAME), [(0, 0)]);
        // A zero separation still never picks a cell twice.
        let two = scores(&[((3, 3), 1.0), ((3, 4), 0.5)]);
        assert_eq!(pick(&two, 5, 0.0, &[], 0.0, &FRAME), [(3, 3), (3, 4)]);
    }

    #[test]
    fn interior_excludes_the_margin_share() {
        let frame = MapFrame {
            rows: 64,
            cols: 64,
            cell_m: 100.0,
        };
        let inside = interior(&frame, 0.025);
        // round(0.025 * 64) = 2 cells on each side.
        let count = inside.as_slice().iter().filter(|b| **b).count();
        assert_eq!(count, 60 * 60);
        assert!(!inside.get(1, 30) && inside.get(2, 30) && inside.get(61, 30));
        assert!(!inside.get(62, 30) && !inside.get(30, 1) && !inside.get(30, 62));
        assert!(inside.get(30, 2) && inside.get(30, 61));
    }

    #[test]
    fn interior_uses_the_shorter_side_and_handles_the_extremes() {
        let frame = MapFrame {
            rows: 40,
            cols: 64,
            cell_m: 100.0,
        };
        let inside = interior(&frame, 0.1);
        // 0.1 * 40 = 4 cells on every side, also along the longer one.
        assert!(!inside.get(3, 30) && inside.get(4, 30) && inside.get(35, 30));
        assert!(!inside.get(20, 3) && inside.get(20, 4) && inside.get(20, 59));
        assert!(!inside.get(20, 60));
        let all = interior(&frame, 0.0);
        assert!(all.as_slice().iter().all(|b| *b));
        let none = interior(&frame, 0.5);
        assert!(none.as_slice().iter().all(|b| !*b));
    }
}
