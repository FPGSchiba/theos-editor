//! Steps 3 and 4 of `resolve_sinks`: the lowest pass between every two neighbouring basins, and
//! their minimum spanning tree oriented to the root (Cordonnier, Bovy & Braun 2019).

use std::collections::VecDeque;

use super::basins::{Basins, ROOT};
use crate::grid::{D8, Grid};
use crate::kernels::mst::kruskal;
use crate::kernels::num::{OrdF64, key, max};

/// The D8 offsets E, SE, S and SW: taken from every cell, they reach each pair of neighbours
/// once, the second cell having the larger index.
const FORWARD: [(isize, isize); 4] = [D8[2], D8[3], D8[4], D8[5]];

/// The lowest pass between two basins.
struct Pass {
    /// The two basins, the smaller number first.
    basins: (usize, usize),
    /// `max(h_a, h_b)` over the pair of cells.
    level: f64,
    /// The pair of cells, the smaller index first.
    cells: (usize, usize),
}

/// How a closed basin drains: from its cell `a` over the pass at `level` to the cell `b` of its
/// parent basin in the tree.
#[derive(Debug, Clone, Copy)]
pub(super) struct Link {
    /// The closed basin.
    pub basin: usize,
    /// The basin's cell on the pass.
    pub a: usize,
    /// The parent's cell on the pass.
    pub b: usize,
    /// The pass height `max(h_a, h_b)`.
    pub level: f64,
}

/// The link of every closed basin, parents before children: breadth-first from the root over
/// the minimum spanning tree of the basin graph, the children of a basin in basin order.
pub(super) fn links(height: &Grid<f64>, basins: &Basins) -> Vec<Link> {
    let passes = passes(height, basins);
    let edges: Vec<(usize, usize, f64)> = passes
        .iter()
        .map(|p| (p.basins.0, p.basins.1, p.level))
        .collect();
    let mut tree: Vec<Vec<usize>> = vec![Vec::new(); basins.count];
    for e in kruskal(basins.count, &edges) {
        let (x, y) = passes[e].basins;
        tree[x].push(e);
        tree[y].push(e);
    }
    let mut links = Vec::with_capacity(basins.count);
    let mut reached = vec![false; basins.count];
    reached[ROOT] = true;
    let mut queue = VecDeque::from([ROOT]);
    while let Some(parent) = queue.pop_front() {
        let mut children: Vec<(usize, usize)> = tree[parent]
            .iter()
            .map(|&e| {
                let (x, y) = passes[e].basins;
                (if x == parent { y } else { x }, e)
            })
            .filter(|&(child, _)| !reached[child])
            .collect();
        children.sort_unstable();
        for (child, e) in children {
            reached[child] = true;
            let (i, j) = passes[e].cells;
            let (a, b) = if basins.of[i] == child {
                (i, j)
            } else {
                (j, i)
            };
            let level = passes[e].level;
            links.push(Link {
                basin: child,
                a,
                b,
                level,
            });
            queue.push_back(child);
        }
    }
    links
}

/// The lowest pass of every pair of neighbouring basins, ties by the smaller pair of cells.
///
/// One scan over the D8 pairs keeps each basin pair's lowest pass under its larger basin, which
/// is never the root: a closed basin has few neighbours, so the lookup stays short, and no list
/// of every boundary pair is built or sorted.
fn passes(height: &Grid<f64>, basins: &Basins) -> Vec<Pass> {
    let (heights, of) = (height.as_slice(), &basins.of);
    let mut lowest: Vec<Vec<Pass>> = (0..basins.count).map(|_| Vec::new()).collect();
    for row in 0..height.rows() {
        for col in 0..height.cols() {
            let here = height.index(row, col);
            for &offset in &FORWARD {
                let Some(there) = height.neighbour_at(row, col, offset) else {
                    continue;
                };
                let (from, to) = (of[here], of[there]);
                if from == to {
                    continue;
                }
                let pass = Pass {
                    basins: (from.min(to), from.max(to)),
                    level: max(heights[here], heights[there]),
                    cells: (here, there),
                };
                let kept = &mut lowest[pass.basins.1];
                match kept.iter_mut().find(|known| known.basins == pass.basins) {
                    Some(known) => {
                        if order(&pass) < order(known) {
                            *known = pass;
                        }
                    }
                    None => kept.push(pass),
                }
            }
        }
    }
    lowest.into_iter().flatten().collect()
}

/// The order of passes between the same two basins: the lower first, then the smaller cells.
fn order(pass: &Pass) -> (OrdF64, (usize, usize)) {
    (OrdF64(key(pass.level)), pass.cells)
}
