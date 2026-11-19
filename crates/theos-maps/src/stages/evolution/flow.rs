//! Flow helpers shared by the routing processes: D8 distances and steepest-descent receivers.

use std::cmp::Ordering;
use std::f64::consts::SQRT_2;

use std::any::Any;

use serde::Serialize;

use crate::grid::{D8, Grid};
use crate::kernels::num::{OrdF64, key};
use crate::world::{ValueType, WorldValue};

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

/// The world value `weights`: where each cell's water goes, as a compressed-row graph. Cell `i`
/// sends `weight[e]` of its flow to cell `to[e]`, over `dist[e]` metres, for `e` in
/// `start[i]..start[i + 1]`. A cell that receives itself (an outlet, or the closed map's sink)
/// has no edge. The weights of a cell with edges sum to 1.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlowWeights {
    /// The first edge of each cell; one entry more than there are cells.
    pub start: Vec<u32>,
    /// The receiving cell of each edge.
    pub to: Vec<u32>,
    /// The share of the donor's flow along each edge.
    pub weight: Vec<f64>,
    /// The length of each edge in metres.
    pub dist: Vec<f64>,
}

impl FlowWeights {
    /// The number of cells.
    #[must_use]
    pub fn cells(&self) -> usize {
        self.start.len().saturating_sub(1)
    }

    /// The edges of cell `i` as an index range into `to`, `weight` and `dist`.
    #[must_use]
    pub fn edges(&self, i: usize) -> std::ops::Range<usize> {
        match (self.start.get(i), self.start.get(i + 1)) {
            (Some(&from), Some(&until)) => cell_index(from)..cell_index(until),
            _ => 0..0,
        }
    }
}

impl WorldValue for FlowWeights {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        theos_map_format::json::to_checked_value(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for FlowWeights {
    const NAME: &'static str = "weights";
}

/// The flow that reaches each cell: `input` plus everything its donors pass on, summed in
/// reverse `stack` order (donors before receivers), so `out[i] = input[i] + sum of out[d]·w(d→i)`
/// over the donors `d` of `i`.
#[must_use]
pub fn accumulate(stack: &[u32], w: &FlowWeights, input: &Grid<f64>) -> Grid<f64> {
    let mut out = input.clone();
    for &cell in stack.iter().rev() {
        let i = cell_index(cell);
        let carried = out.as_slice()[i];
        for e in w.edges(i) {
            out.as_mut_slice()[cell_index(w.to[e])] += carried * w.weight[e];
        }
    }
    out
}

/// The flow that leaves the map: [`accumulate`]d flow summed over the `outlets`, in cell order.
/// The closed map's sink is not an outlet; its water stays on the map.
#[must_use]
#[cfg(test)]
pub fn outflow(stack: &[u32], w: &FlowWeights, input: &Grid<f64>, outlets: &Grid<bool>) -> f64 {
    accumulate(stack, w, input)
        .as_slice()
        .iter()
        .zip(outlets.as_slice())
        .filter(|&(_, &outlet)| outlet)
        .fold(0.0, |sum, (flow, _)| sum + flow)
}

#[cfg(test)]
mod tests {
    use std::f64::consts::SQRT_2;

    use super::{FlowWeights, accumulate, d8_distance, outflow};
    use crate::grid::Grid;

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

    /// Cell 0 splits 1/4 to cell 1 and 3/4 to cell 2; cell 1 sends all to cell 2; cell 2 is the
    /// outlet. The stack runs receivers first.
    fn small() -> (Vec<u32>, FlowWeights) {
        let w = FlowWeights {
            start: vec![0, 2, 3, 3],
            to: vec![1, 2, 2],
            weight: vec![0.25, 0.75, 1.0],
            dist: vec![1.0; 3],
        };
        (vec![2, 1, 0], w)
    }

    #[test]
    fn accumulate_adds_what_the_donors_pass_on() {
        let (stack, w) = small();
        let input = Grid::from_vec(1, 3, vec![8.0, 1.0, 0.5]).unwrap();
        assert_eq!(accumulate(&stack, &w, &input).as_slice(), [8.0, 3.0, 9.5]);
        let outlets = Grid::from_vec(1, 3, vec![false, false, true]).unwrap();
        assert!((outflow(&stack, &w, &input, &outlets) - 9.5).abs() < 1e-12);
        assert_eq!(w.cells(), 3);
        assert_eq!(w.edges(7), 0..0);
    }
}
