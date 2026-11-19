//! The slope supply that passes dry catchments without being laid down, when the recipe switches
//! it on (`fluvial`'s `supply_passes_dry_cells`, off by default; Task 27h).
//!
//! **The loop it stops.** Where a cell's catchment is dry, the law lays down nearly all of the
//! slope supply that passes through it: a headwater cell keeps `c/(1 + c)` of its own supply,
//! with `c = G·area/Q` at 100 to 200 on the dry patches of the 384² budget recipe. That supply is
//! what `slope_limit` cut from the cell because it stood above its rock's critical slope. Laid
//! back on the cell it stands there again, `slope_limit` cuts it again in the same step, and the
//! supply pending on the cell grows by each step's new cut until the climate turns wetter
//! (Task 27g: to about 1.7 km per cell). The material never leaves the cell.
//!
//! **What the switch does.** A cell is *dry* when its catchment's mean runoff weight `Q/A` is
//! below G, `Q < G·A`, with `A` the plain drainage area: there Yuan et al.'s condition
//! `G/p̂ ≤ 1` fails, and a dry catchment keeps about G/p̄ to 1 of its supply. A locally wet cell
//! can count as dry; with uniform runoff every cell is dry when G > 1 and none is when G ≤ 1.
//! The slope supply passes dry cells without being laid down, as debris runs out down dry
//! gullies, and joins the sediment at the first cell that is not dry, at a routed depression
//! (its fill) or at an outlet. [`passing`] gives, per cell, the supply volume `P_i` that passes
//! it, and the cell's deposition term leaves it out (`solve`). Only where the supply is laid
//! down changes; what the cells erode is laid down as before, and the books close the volume
//! balance as before.

use crate::stages::evolution::flow::{FlowWeights, cell_index};

/// Per cell, the volume of slope supply (m³) that passes it without being laid down: on a cell
/// with an edge where `dry(i)` holds, its own `supply` plus what reaches it from such cells
/// upstream, split like its water; 0 on every other cell, where what reaches it joins the
/// sediment that the law lays down. One walk in reverse `stack` order, donors first.
pub(super) fn passing(
    stack: &[u32],
    weights: &FlowWeights,
    supply: &[f64],
    dry: impl Fn(usize) -> bool,
) -> Vec<f64> {
    let mut reaching = vec![0.0; supply.len()];
    let mut through = vec![0.0; supply.len()];
    for &cell in stack.iter().rev() {
        let i = cell_index(cell);
        let edges = weights.edges(i);
        if edges.is_empty() || !dry(i) {
            continue;
        }
        let passes = reaching[i] + supply[i];
        through[i] = passes;
        for e in edges {
            reaching[cell_index(weights.to[e])] += passes * weights.weight[e];
        }
    }
    through
}

#[cfg(test)]
mod tests {
    use super::passing;
    use crate::stages::evolution::flow::FlowWeights;

    /// A 1 x 6 strip: cell 0 is the outlet; 1 → 0; 2 → 1; 3 → 2; 4 sends 1/4 to 3 and 3/4 to 2;
    /// 5 → 4.
    fn strip() -> FlowWeights {
        FlowWeights {
            start: vec![0, 0, 1, 2, 3, 5, 6],
            to: vec![0, 1, 2, 3, 2, 4],
            weight: vec![1.0, 1.0, 1.0, 0.25, 0.75, 1.0],
            dist: vec![1.0; 6],
        }
    }

    const STACK: [u32; 6] = [0, 1, 2, 3, 4, 5];

    #[test]
    fn the_supply_passes_dry_cells_and_stops_at_the_first_that_is_not() {
        // Cells 3, 4 and 5 are dry. Cell 5 passes its 4; cell 4 passes 4 + 2, a quarter to cell 3
        // and three quarters to cell 2; cell 3 passes 1.5 + 1. Cell 2 is not dry: nothing passes
        // it, and cells 1 and 0 see no passing supply either.
        let supply = [8.0, 16.0, 32.0, 1.0, 2.0, 4.0];
        let through = passing(&STACK, &strip(), &supply, |i| i >= 3);
        assert_eq!(through, [0.0, 0.0, 0.0, 2.5, 6.0, 4.0]);
        // A dry cell without an edge (the outlet) passes nothing on: the supply leaves there.
        let through = passing(&STACK, &strip(), &supply, |_| true);
        assert_eq!(through, [0.0, 55.0, 39.0, 2.5, 6.0, 4.0]);
        // No dry cell, nothing passes.
        let through = passing(&STACK, &strip(), &supply, |_| false);
        assert_eq!(through, [0.0; 6]);
    }
}
