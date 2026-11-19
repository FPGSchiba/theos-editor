//! The exact threshold limiter of `slope_limit` (spec §5.9 "Slope limit"): one sweep of a
//! priority queue over the cells, lowest first, a shortest-path form of a slope-limited fill.
//!
//! **The surface it finds.** A cell `i` that is not fixed may stand at most `S_c,i × d_ij` above
//! each D8 neighbour `j`, with `S_c,i` its critical slope and `d_ij` the D8 distance. The highest
//! surface at or below the heights `h` that keeps this rule is the largest `L` with
//!
//! ```text
//! L_i = h_i                                          for a fixed cell,
//! L_i = min(h_i, min over j of (L_j + S_c,i × d_ij))  for every other cell.
//! ```
//!
//! **One sweep finds it.** Read as a graph whose step onto a cell `n` that is not fixed costs
//! `S_c,n × d` (no step enters a fixed cell), `L_i` is the lowest `h_s + Σ cost` over the paths
//! from any cell `s` to `i`: a shortest path with every cell a source at its own height. That is
//! Dijkstra's algorithm with many sources. The queue starts with every cell at its height and
//! pops the lowest (ties by cell index); a popped cell's level is final, and it offers each
//! neighbour `n` that is not fixed the level `L + S_c,n × d`, which lowers `n` when it is below
//! `n`'s level so far. A lowered cell is queued again at its new level; its older entry is
//! skipped when it comes up (lazy deletion). Each cell is final once, so the sweep takes
//! `O(n log n)` for `n` cells.
//!
//! **Why it is exact, also in floating point.** The costs are not negative, and rounding
//! `L + cost` to the nearest float never gives less than `L`, so a cell popped later can never
//! lower one popped earlier. Every cell therefore ends at `min(h_i, min_j fl(L_j + cost))`, the
//! rule as the floats compute it, and at the highest such level: any surface at or below `h`
//! that keeps the rule lies at or below every path's sum, step by step. The result does not
//! depend on the order of ties; the order by cell index only makes the sweep itself the same on
//! every run.

use crate::kernels::routing::d8_distance;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::grid::{D8, Grid};
use crate::kernels::num::{OrdF64, key};

/// The highest surface at or below `height` on which no cell that is not `fixed` stands more
/// than its `critical` slope × the D8 distance above a neighbour; fixed cells keep their height.
/// `critical` and `fixed` hold one entry per cell, the critical slopes are not negative, and
/// `cell_m` is the side of a cell in metres.
pub(super) fn limited(
    height: &Grid<f64>,
    critical: &[f64],
    fixed: &[bool],
    cell_m: f64,
) -> Grid<f64> {
    let distance: [f64; 8] = std::array::from_fn(|k| d8_distance(cell_m, k));
    let mut level = height.clone();
    let mut done = vec![false; level.len()];
    let mut queue: BinaryHeap<Reverse<(OrdF64, usize)>> = level
        .as_slice()
        .iter()
        .enumerate()
        .map(|(i, &h)| Reverse((OrdF64(key(h)), i)))
        .collect();
    while let Some(Reverse((_, i))) = queue.pop() {
        if done[i] {
            continue;
        }
        done[i] = true;
        let at = level.as_slice()[i];
        for (k, &offset) in D8.iter().enumerate() {
            let Some(n) = level.neighbour(i, offset) else {
                continue;
            };
            if done[n] || fixed[n] {
                continue;
            }
            let offer = at + critical[n] * distance[k];
            if offer < level.as_slice()[n] {
                level.as_mut_slice()[n] = offer;
                queue.push(Reverse((OrdF64(key(offer)), n)));
            }
        }
    }
    level
}
