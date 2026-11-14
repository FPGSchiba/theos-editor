//! The simple marks of `inworld`: peaks on the ridges and trees in the forests. 0.3.0 drew a
//! random sample; renders have no random stream, so one cell per block is taken: the one with
//! the highest score, the lowest index on a tie.

use std::collections::BTreeMap;

use crate::grid::Grid;
use crate::kernels::filters::gaussian;
use crate::kernels::num::{to_f64, to_index};
use crate::world::MapFrame;

/// The blur that gives the ridge's local mean, cells (0.3.0).
const RIDGE_BLUR_CELLS: f64 = 8.0;
/// A cell is a ridge when its prominence is within this top share of all cells (0.3.0: 3 %).
const RIDGE_TOP_SHARE: f64 = 0.03;
/// The vegetation density above which a cell is forest (0.3.0).
const FOREST_MIN: f64 = 0.75;
/// The block edge for peaks, cells.
const PEAK_BLOCK: usize = 6;
/// The block edge for trees, cells.
const TREE_BLOCK: usize = 8;

/// Keeps one candidate `(cell index, score)` per `block` x `block` cells: the highest score,
/// the lowest index on a tie; the result is in block order.
fn thin(candidates: impl Iterator<Item = (usize, f64)>, cols: usize, block: usize) -> Vec<usize> {
    let mut best: BTreeMap<(usize, usize), (usize, f64)> = BTreeMap::new();
    for (i, score) in candidates {
        let key = (i / cols / block, i % cols / block);
        let slot = best.entry(key).or_insert((i, score));
        if score.total_cmp(&slot.1).is_gt() {
            *slot = (i, score);
        }
    }
    best.values().map(|&(i, _)| i).collect()
}

/// The centre in metres of the cell with flat index `i`.
fn centre(frame: &MapFrame, i: usize) -> [f64; 2] {
    frame.centre_m(i / frame.cols, i % frame.cols)
}

/// The points in metres where peaks are drawn: known cells in the top share of prominence
/// (the height above its blurred mean), one per block.
pub(crate) fn peaks(height: &Grid<f64>, known: &Grid<bool>, frame: &MapFrame) -> Vec<[f64; 2]> {
    let mean = gaussian(height, RIDGE_BLUR_CELLS * frame.cell_m, frame.cell_m);
    let prominence: Vec<f64> = height
        .as_slice()
        .iter()
        .zip(mean.as_slice())
        .map(|(h, m)| h - m)
        .collect();
    let mut sorted = prominence.clone();
    sorted.sort_by(f64::total_cmp);
    let cut = to_index(to_f64(sorted.len()) * (1.0 - RIDGE_TOP_SHARE), sorted.len());
    let Some(&threshold) = sorted.get(cut) else {
        return Vec::new();
    };
    let candidates = prominence
        .iter()
        .enumerate()
        .filter(|&(i, &p)| p > threshold && p > 0.0 && known.as_slice()[i])
        .map(|(i, &p)| (i, p));
    thin(candidates, frame.cols, PEAK_BLOCK)
        .into_iter()
        .map(|i| centre(frame, i))
        .collect()
}

/// The points in metres where trees are drawn: known cells of vegetation above the forest
/// threshold, one per block.
pub(crate) fn trees(vegetation: &Grid<f64>, known: &Grid<bool>, frame: &MapFrame) -> Vec<[f64; 2]> {
    let candidates = vegetation
        .as_slice()
        .iter()
        .enumerate()
        .filter(|&(i, &v)| v > FOREST_MIN && known.as_slice()[i])
        .map(|(i, &v)| (i, v));
    thin(candidates, frame.cols, TREE_BLOCK)
        .into_iter()
        .map(|i| centre(frame, i))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> MapFrame {
        MapFrame {
            rows: 16,
            cols: 16,
            cell_m: 50.0,
        }
    }

    #[test]
    fn trees_are_one_per_block_on_known_forest_only() {
        let veg = Grid::filled(16, 16, 0.9);
        let mut known = Grid::filled(16, 16, false);
        for r in 0..8 {
            for c in 0..8 {
                known.set(r, c, true);
            }
        }
        let out = trees(&veg, &known, &frame());
        assert_eq!(out, vec![frame().centre_m(0, 0)]);
        let bare = Grid::filled(16, 16, 0.1);
        assert_eq!(trees(&bare, &known, &frame()), Vec::<[f64; 2]>::new());
    }

    #[test]
    fn a_sharp_known_summit_is_a_peak() {
        let mut height = Grid::filled(16, 16, 100.0);
        height.set(4, 4, 900.0);
        let known = Grid::filled(16, 16, true);
        let out = peaks(&height, &known, &frame());
        assert_eq!(out, vec![frame().centre_m(4, 4)]);
        assert_eq!(
            peaks(&height, &Grid::filled(16, 16, false), &frame()),
            Vec::<[f64; 2]>::new()
        );
    }
}
