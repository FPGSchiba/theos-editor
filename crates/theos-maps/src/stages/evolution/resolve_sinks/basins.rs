//! Steps 1 and 2 of `resolve_sinks`: the outlets, and the basin every receiver chain ends in.

use crate::grid::Grid;
use crate::kernels::num::to_usize;
use crate::kernels::routing::outlets;

/// The basin of every outlet: the root of the basin tree.
pub(super) const ROOT: usize = 0;

/// The basin of every cell.
pub(super) struct Basins {
    /// The basin of each cell: [`ROOT`] where the receiver chain ends at an outlet, `1..count`
    /// for the closed basins, numbered in the order of their pits' cell indices.
    pub of: Vec<usize>,
    /// The number of basins, the root included.
    pub count: usize,
}

/// The first cell whose receiver is neither the cell itself nor a lower D8 neighbour, with that
/// receiver. When there is none, every receiver chain descends and ends at a cell that receives
/// itself.
pub(super) fn bad_receiver(height: &Grid<f64>, receivers: &Grid<u32>) -> Option<(usize, u32)> {
    let h = height.as_slice();
    let lower_neighbour = |i: usize, j: usize| {
        let near = |(ri, ci): (usize, usize), (rj, cj): (usize, usize)| {
            ri.abs_diff(rj) <= 1 && ci.abs_diff(cj) <= 1
        };
        j < h.len() && near(height.cell(i), height.cell(j)) && h[j] < h[i]
    };
    receivers
        .as_slice()
        .iter()
        .enumerate()
        .find(|&(i, &r)| {
            let j = to_usize(r);
            j != i && !lower_neighbour(i, j)
        })
        .map(|(i, &r)| (i, r))
}

/// The outlets are the `fixed` cells, or the lowest cell (ties by index) when none is fixed,
/// the map's sink; they all belong to [`ROOT`]. Every other cell belongs to the basin of the pit
/// (a cell receiving itself) its receiver chain ends in. `receivers` must have no
/// [`bad_receiver`], so that every chain ends.
pub(super) fn basins(height: &Grid<f64>, receivers: &Grid<u32>, fixed: &Grid<bool>) -> Basins {
    const UNSET: usize = usize::MAX;
    let mut of = vec![UNSET; height.len()];
    for outlet in outlets(height, fixed) {
        of[outlet] = ROOT;
    }
    let to = |i: usize| to_usize(receivers.as_slice()[i]);
    let mut count = ROOT + 1;
    for (i, basin) in of.iter_mut().enumerate() {
        if *basin == UNSET && to(i) == i {
            *basin = count;
            count += 1;
        }
    }
    let mut path = Vec::new();
    for i in 0..of.len() {
        let mut at = i;
        while of[at] == UNSET {
            path.push(at);
            at = to(at);
        }
        let basin = of[at];
        for cell in path.drain(..) {
            of[cell] = basin;
        }
    }
    Basins { of, count }
}
