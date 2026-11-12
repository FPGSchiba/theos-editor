//! Step 5 of `resolve_sinks`: the routing surface, which lifts every depression to its spill
//! level along a priority flood with ε (Barnes, Lehman & Mulla 2014), and the depressions.

use super::basins::Basins;
use super::depressions::{Depressions, SpillPoint};
use super::tree::Link;
use crate::grid::Grid;
use crate::kernels::flood::priority_flood_where;
use crate::kernels::num::max;
use crate::stages::evolution::flow::{cell_index, cell_u32};

/// The rise per flood step on the routing surface, in metres.
const EPSILON: f64 = 1e-6;

/// The routing surface of `height` and its depressions, for the basin `links` in tree order
/// (parents first).
///
/// Each closed basin is flooded from its pass cell `a`, which starts at the pass level `L`, or
/// at the routing level of `b` when that is higher (a depression spilling into a lake lifted
/// above `L`). The flood enters only the basin's own cells that it raises, each to the level
/// it was entered from + ε: the basin's cells below the level that connect to its pit, and any
/// cell just above it next to them, which keeps a way down. The raised cells (with `a` when it
/// is raised) form the basin's depression. Cells outside every depression keep `height`.
pub(super) fn lift(
    height: &Grid<f64>,
    basins: &Basins,
    links: &[Link],
) -> (Grid<f64>, Depressions) {
    let h = height.as_slice();
    let mut surface = height.clone();
    let mut visited = vec![false; h.len()];
    let mut label = Grid::filled(height.rows(), height.cols(), 0_u32);
    let mut spill = Vec::new();
    for link in links {
        let start = max(link.level, surface.as_slice()[link.b]);
        surface.as_mut_slice()[link.a] = start;
        let raises = |j: usize, level: f64| basins.of[j] == link.basin && h[j] < level + EPSILON;
        let order = priority_flood_where(&mut surface, &mut visited, &[link.a], EPSILON, raises);
        let lake: Vec<usize> = order
            .into_iter()
            .map(cell_index)
            .filter(|&c| surface.as_slice()[c] > h[c])
            .collect();
        if lake.is_empty() {
            continue;
        }
        spill.push(SpillPoint {
            cell: cell_u32(link.a),
            to: cell_u32(link.b),
            level: link.level,
        });
        let n = u32::try_from(spill.len()).unwrap_or(u32::MAX);
        for c in lake {
            label.as_mut_slice()[c] = n;
        }
    }
    (surface, Depressions { label, spill })
}
