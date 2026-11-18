//! Step 5 of `resolve_sinks`: the routing surface, which lifts every depression to its spill
//! level along a priority flood with ε (Barnes, Lehman & Mulla 2014), and the depressions.

use super::basins::Basins;
use super::depressions::{Depressions, SpillPoint};
use super::tree::Link;
use crate::grid::Grid;
use crate::kernels::flood::priority_flood_where;
use crate::stages::evolution::flow::{cell_index, cell_u32};

/// The rise per flood step on the routing surface, in metres.
const EPSILON: f64 = 1e-6;

/// The routing surface of `height` and its depressions, for the basin `links` in tree order
/// (parents first).
///
/// Each closed basin is flooded from its pass cell `a`, which starts at the pass level `L`, or
/// at one ε above the routing level of `b` when that is higher (a basin spilling into a lake
/// lifted above `L`). The flood enters only the basin's own cells that it raises, each to the level it was
/// entered from + ε: the basin's cells below the level that connect to its pit, and any cell
/// just above it next to them, which keeps a way down. The raised cells (with `a` when it is
/// raised) form the basin's depression, with its spill point over the pass at `L`. A basin that
/// starts above `L` lies under the lake of `b`, so its raised cells join that lake's depression:
/// one lake fills as one, up to the level it spills at (spec §5.9 "Depression filling"). Cells
/// outside every depression keep `height`.
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
        // Over a lake of `b` standing above the pass, `a` starts one ε above `b`: level with `b`
        // it would sort ahead of `b` and its neighbours by index, and a full fill would leave it
        // a pit.
        let above = surface.as_slice()[link.b];
        let start = if above > link.level {
            above + EPSILON
        } else {
            link.level
        };
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
        // `b` lies in a lake standing above this basin's pass: `start > L` only when the flood
        // lifted `b` above its height, so `b` has a depression.
        let under = label.as_slice()[link.b];
        let n = if start > link.level && under > 0 {
            under
        } else {
            spill.push(SpillPoint {
                cell: cell_u32(link.a),
                to: cell_u32(link.b),
                level: link.level,
            });
            u32::try_from(spill.len()).unwrap_or(u32::MAX)
        };
        for c in lake {
            label.as_mut_slice()[c] = n;
        }
    }
    (surface, Depressions { label, spill })
}
