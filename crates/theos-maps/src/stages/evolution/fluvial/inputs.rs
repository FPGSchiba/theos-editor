//! The checks that `fluvial`'s routed inputs fit the grid. Values (`weights`, `depressions`) are
//! not shape-checked by the world, so these cheap guards keep every index on its slice.

use crate::stages::evolution::flow::{FlowWeights, cell_index};
use crate::stages::evolution::resolve_sinks::Depressions;

/// Why `weights` and `stack` do not fit a grid of `cells` cells, or `None` when they do.
pub(super) fn routing_misfit(weights: &FlowWeights, stack: &[u32], cells: usize) -> Option<String> {
    if weights.cells() != cells {
        return Some(format!(
            "the flow weights cover {} cells, but the grid has {cells}",
            weights.cells()
        ));
    }
    let edges = weights.to.len();
    if weights.weight.len() != edges || weights.dist.len() != edges {
        return Some(format!(
            "the flow weights hold {edges} targets, {} shares and {} distances",
            weights.weight.len(),
            weights.dist.len()
        ));
    }
    if let Some(start) = weights.start.iter().find(|&&s| cell_index(s) > edges) {
        return Some(format!(
            "the flow weights start an edge list at {start}, past their {edges} edges"
        ));
    }
    if let Some(to) = weights.to.iter().find(|&&t| cell_index(t) >= cells) {
        return Some(format!(
            "a flow weight sends water to cell {to}, outside the grid of {cells} cells"
        ));
    }
    if let Some(cell) = stack.iter().find(|&&c| cell_index(c) >= cells) {
        return Some(format!(
            "the stack names cell {cell}, outside the grid of {cells} cells"
        ));
    }
    None
}

/// Why `depressions` do not fit a grid of `cells` cells, or `None` when they do: the labels
/// cover the grid and name only depressions that spill; every spill cell lies on the grid, no
/// two depressions spill from one cell, and every spill level is finite.
pub(super) fn depressions_misfit(depressions: &Depressions, cells: usize) -> Option<String> {
    let label = depressions.label.as_slice();
    if label.len() != cells {
        return Some(format!(
            "the depressions label {} cells, but the grid has {cells}",
            label.len()
        ));
    }
    let count = depressions.spill.len();
    if let Some((cell, n)) = label
        .iter()
        .enumerate()
        .find(|&(_, &n)| cell_index(n) > count)
    {
        let spill = if count == 1 {
            "depression spills"
        } else {
            "depressions spill"
        };
        return Some(format!(
            "cell {cell} lies in depression {n}, but only {count} {spill}"
        ));
    }
    let mut spilling = vec![0; cells];
    for (n, point) in (1..).zip(&depressions.spill) {
        let at = cell_index(point.cell);
        let Some(earlier) = spilling.get_mut(at) else {
            return Some(format!(
                "depression {n} spills from cell {at}, outside the grid of {cells} cells"
            ));
        };
        if *earlier > 0 {
            return Some(format!(
                "depressions {earlier} and {n} both spill from cell {at}"
            ));
        }
        *earlier = n;
        if !point.level.is_finite() {
            return Some(format!(
                "depression {n} has the spill level {}",
                point.level
            ));
        }
    }
    None
}
