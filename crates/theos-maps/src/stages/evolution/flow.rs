//! The flow graph of the routing processes: the world value `weights` and the flow accumulated
//! over it. The D8 distances and the steepest-descent receivers are in
//! [`crate::kernels::routing`].

use std::any::Any;

use serde::Serialize;

use crate::grid::Grid;
use crate::kernels::num::to_usize;
use crate::world::{ValueType, WorldValue};

/// The world value `weights`: where each cell's water goes, as a compressed-row graph. Cell `i`
/// sends `weight[e]` of its flow to cell `to[e]`, over `dist[e]` metres, for `e` in
/// `start[i]..start[i + 1]`. A cell that receives itself (an outlet, or the closed map's sink)
/// has no edge. The weights of a cell with edges sum to 1.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct FlowWeights {
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
            (Some(&from), Some(&until)) => to_usize(from)..to_usize(until),
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
pub(crate) fn accumulate(stack: &[u32], w: &FlowWeights, input: &Grid<f64>) -> Grid<f64> {
    let mut out = input.clone();
    for &cell in stack.iter().rev() {
        let i = to_usize(cell);
        let carried = out.as_slice()[i];
        for e in w.edges(i) {
            out.as_mut_slice()[to_usize(w.to[e])] += carried * w.weight[e];
        }
    }
    out
}

/// The flow that leaves the map: [`accumulate`]d flow summed over the `outlets`, in cell order.
/// The closed map's sink is not an outlet; its water stays on the map.
#[must_use]
#[cfg(test)]
pub(crate) fn outflow(
    stack: &[u32],
    w: &FlowWeights,
    input: &Grid<f64>,
    outlets: &Grid<bool>,
) -> f64 {
    accumulate(stack, w, input)
        .as_slice()
        .iter()
        .zip(outlets.as_slice())
        .filter(|&(_, &outlet)| outlet)
        .fold(0.0, |sum, (flow, _)| sum + flow)
}

#[cfg(test)]
mod tests {
    use super::{FlowWeights, accumulate, outflow};
    use crate::grid::Grid;

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
