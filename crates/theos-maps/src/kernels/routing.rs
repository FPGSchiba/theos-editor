//! D8 routing on a surface: the distance to a D8 neighbour, the steepest-descent receiver of
//! every cell, and the outlets of a map, the closed map's sink among them. `landscape_evolution`
//! and `hydrology` share them.

use std::cmp::Ordering;
use std::f64::consts::SQRT_2;

use crate::grid::{D8, Grid};
use crate::kernels::num::{OrdF64, key, to_u32};

/// The distance in metres to the neighbour at [`D8`] offset `k` of a cell of `cell_m` metres:
/// `cell_m` along a row or column (even `k`), `cell_m·√2` along a diagonal (odd `k`).
#[must_use]
pub fn d8_distance(cell_m: f64, k: usize) -> f64 {
    if k % 2 == 1 { cell_m * SQRT_2 } else { cell_m }
}

/// The steepest-descent receiver of every cell of `surface` (Braun & Willett 2013): the D8
/// neighbour with the largest drop ÷ distance, equal slopes going to the lower neighbour index.
/// A cell with no strictly lower neighbour, or a `fixed` cell, receives itself.
#[must_use]
pub fn steepest_receivers(surface: &Grid<f64>, fixed: &Grid<bool>, cell_m: f64) -> Grid<u32> {
    let mut receivers = Grid::filled(surface.rows(), surface.cols(), 0_u32);
    for r in 0..surface.rows() {
        for c in 0..surface.cols() {
            let i = surface.index(r, c);
            let to = if fixed.as_slice()[i] {
                i
            } else {
                steepest(surface, cell_m, r, c)
            };
            receivers.set(r, c, to_u32(to));
        }
    }
    receivers
}

/// The steepest strictly lower neighbour of the cell at `row`, `col`, or the cell itself when
/// there is none.
fn steepest(surface: &Grid<f64>, cell_m: f64, row: usize, col: usize) -> usize {
    let (heights, cell) = (surface.as_slice(), surface.index(row, col));
    let mut best: Option<(usize, f64)> = None;
    for (dir, &offset) in D8.iter().enumerate() {
        let Some(next) = surface.neighbour_at(row, col, offset) else {
            continue;
        };
        let drop = heights[cell] - heights[next];
        if drop <= 0.0 {
            continue;
        }
        let slope = drop / d8_distance(cell_m, dir);
        let steeper = best.is_none_or(|(kept, kept_slope)| {
            match OrdF64(key(slope)).cmp(&OrdF64(key(kept_slope))) {
                Ordering::Greater => true,
                Ordering::Equal => next < kept,
                Ordering::Less => false,
            }
        });
        if steeper {
            best = Some((next, slope));
        }
    }
    best.map_or(cell, |(next, _)| next)
}

/// The lowest of `cells` on `height`, ties going to the lower index; `None` when `cells` is
/// empty.
pub fn lowest(height: &Grid<f64>, cells: impl Iterator<Item = usize>) -> Option<usize> {
    let h = height.as_slice();
    cells.min_by_key(|&i| (OrdF64(key(h[i])), i))
}

/// The outlets of a map: its `fixed` cells in index order or, on a closed map (no fixed cell),
/// its sink, the lowest cell (ties by index); none for an empty grid.
#[must_use]
pub fn outlets(height: &Grid<f64>, fixed: &Grid<bool>) -> Vec<usize> {
    let drained: Vec<usize> = (0..fixed.len()).filter(|&i| fixed.as_slice()[i]).collect();
    if drained.is_empty() {
        lowest(height, 0..height.len()).into_iter().collect()
    } else {
        drained
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::SQRT_2;

    use super::*;

    #[test]
    fn d8_distance_is_the_cell_or_its_diagonal() {
        for k in [0, 2, 4, 6] {
            assert_eq!(
                d8_distance(62.5, k).to_bits(),
                62.5_f64.to_bits(),
                "k = {k}"
            );
        }
        for k in [1, 3, 5, 7] {
            let diagonal = 62.5 * SQRT_2;
            assert_eq!(
                d8_distance(62.5, k).to_bits(),
                diagonal.to_bits(),
                "k = {k}"
            );
        }
    }

    #[test]
    fn outlets_are_the_fixed_cells_or_the_lowest_cell() {
        let height = Grid::from_vec(1, 4, vec![3.0, 1.0, 2.0, 1.0]).unwrap();
        let fixed = Grid::from_vec(1, 4, vec![false, false, true, true]).unwrap();
        assert_eq!(outlets(&height, &fixed), [2, 3]);
        let closed = Grid::filled(1, 4, false);
        assert_eq!(outlets(&height, &closed), [1], "ties go to the lower index");
        let empty = Grid::from_vec(0, 0, Vec::new()).unwrap();
        let none = Grid::from_vec(0, 0, Vec::new()).unwrap();
        assert_eq!(outlets(&empty, &none), Vec::<usize>::new());
        assert_eq!(lowest(&height, [0, 2].into_iter()), Some(2));
        assert_eq!(lowest(&height, std::iter::empty()), None);
    }
}
