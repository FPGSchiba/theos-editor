//! The checks that `fluvial`'s routed inputs fit the grid. The value `weights` is not
//! shape-checked by the world, so these cheap guards keep every index on its slice.

use crate::stages::evolution::flow::{FlowWeights, cell_index};

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
