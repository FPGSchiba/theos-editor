//! Flow helpers shared by the routing processes: D8 distances and steepest-descent receivers.

use std::cmp::Ordering;
use std::f64::consts::SQRT_2;

use crate::grid::{D8, Grid};
use crate::kernels::num::{OrdF64, key};

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
pub(crate) fn steepest_receivers(
    surface: &Grid<f64>,
    fixed: &Grid<bool>,
    cell_m: f64,
) -> Grid<u32> {
    let mut receivers = Grid::filled(surface.rows(), surface.cols(), 0_u32);
    for r in 0..surface.rows() {
        for c in 0..surface.cols() {
            let i = surface.index(r, c);
            let to = if fixed.as_slice()[i] {
                i
            } else {
                steepest(surface, cell_m, r, c)
            };
            receivers.set(r, c, cell_u32(to));
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

/// A cell index as stored in a `u32` raster; grids hold fewer than `u32::MAX` cells.
pub(crate) fn cell_u32(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}

/// A cell index read from a `u32` raster.
pub(crate) fn cell_index(c: u32) -> usize {
    usize::try_from(c).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use std::f64::consts::SQRT_2;

    use super::d8_distance;

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
}
