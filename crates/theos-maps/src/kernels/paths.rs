//! Dijkstra's shortest paths over a grid, from one or many sources.
//!
//! The queue is a binary heap ordered by `(total_cmp(distance), cell index)`, so cells at equal
//! distance pop in index order. A relaxation updates a cell only on a strictly smaller distance,
//! which makes `prev` deterministic. Stale queue entries are skipped (lazy deletion).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::grid::{D8, D16, Grid};
use crate::kernels::KernelError;
use crate::kernels::num::{OrdF64, key, to_u32, to_usize};

/// How many pops pass between two calls of the cancel callback.
const CANCEL_EVERY: usize = 4096;

/// Which neighbour table a search walks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Directions {
    /// The eight king moves ([`D8`]).
    Eight,
    /// The king moves and the eight knight moves ([`D16`]).
    Sixteen,
}

/// The result of [`shortest_paths`].
///
/// Internal to the generator: `dist` holds `f64::INFINITY` for unreachable cells, so callers must
/// never store it as a layer.
#[derive(Debug, Clone, PartialEq)]
pub struct PathTree {
    /// Distance of every cell from its nearest source; `f64::INFINITY` when unreachable.
    pub dist: Vec<f64>,
    /// The cell before this one on its shortest path; `u32::MAX` for sources and unreachable
    /// cells.
    pub prev: Vec<u32>,
}

/// Shortest paths from `sources` over a `rows` x `cols` grid.
///
/// `cost(from, to)` is the length of one step, or `None` when the step is forbidden; the kernel
/// assumes nothing about it except that it is not negative. `extra(from)` lists additional edges
/// out of a cell (such as bridges) as `(to, length)`; targets off the grid are ignored. Sources
/// off the grid are ignored. `cancel` is called every 4,096 pops.
///
/// # Errors
/// [`KernelError::Cancelled`] when `cancel` returns `true`; [`KernelError::TooMany`] when the
/// grid has more cells than a `u32` numbers.
pub fn shortest_paths(
    rows: usize,
    cols: usize,
    sources: &[usize],
    dirs: Directions,
    cost: &mut dyn FnMut(usize, usize) -> Option<f64>,
    extra: &mut dyn FnMut(usize) -> Vec<(usize, f64)>,
    cancel: &dyn Fn() -> bool,
) -> Result<PathTree, KernelError> {
    let n = rows * cols;
    u32::try_from(n).map_err(|_| KernelError::TooMany { len: n })?;
    let table: &[(isize, isize)] = match dirs {
        Directions::Eight => &D8,
        Directions::Sixteen => &D16,
    };
    let shape = Grid::filled(rows, cols, ());
    let mut dist = vec![f64::INFINITY; n];
    let mut prev = vec![u32::MAX; n];
    let mut heap: BinaryHeap<Reverse<(OrdF64, usize)>> = BinaryHeap::new();
    for &s in sources.iter().filter(|&&s| s < n) {
        if dist[s] > 0.0 {
            dist[s] = 0.0;
            heap.push(Reverse((OrdF64(0.0), s)));
        }
    }
    let mut pops = 0_usize;
    while let Some(Reverse((OrdF64(d), i))) = heap.pop() {
        pops += 1;
        if pops.is_multiple_of(CANCEL_EVERY) && cancel() {
            return Err(KernelError::Cancelled);
        }
        if d > dist[i] {
            continue;
        }
        let from = to_u32(i);
        let mut relax = |j: usize, step: f64| {
            let nd = key(d + step);
            if nd < dist[j] {
                dist[j] = nd;
                prev[j] = from;
                heap.push(Reverse((OrdF64(nd), j)));
            }
        };
        for &offset in table {
            if let Some(j) = shape.neighbour(i, offset)
                && let Some(step) = cost(i, j)
            {
                relax(j, step);
            }
        }
        for (j, step) in extra(i) {
            if j < n {
                relax(j, step);
            }
        }
    }
    Ok(PathTree { dist, prev })
}

/// The cells from a source to `target`, both included; `None` when `target` is off the grid or
/// unreachable.
#[must_use]
pub fn path_to(tree: &PathTree, target: usize) -> Option<Vec<usize>> {
    if tree.dist.get(target)?.is_infinite() {
        return None;
    }
    let mut path = vec![target];
    let mut at = target;
    while let Some(&p) = tree.prev.get(at) {
        if p == u32::MAX {
            break;
        }
        at = to_usize(p);
        path.push(at);
    }
    path.reverse();
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{D8, D16};
    use crate::rng::StageRng;
    use std::cell::Cell;

    fn length(cols: usize, a: usize, b: usize) -> f64 {
        let dr = (a / cols).abs_diff(b / cols);
        let dc = (a % cols).abs_diff(b % cols);
        crate::kernels::num::to_f64(dr * dr + dc * dc).sqrt()
    }

    fn no_extra(_: usize) -> Vec<(usize, f64)> {
        Vec::new()
    }

    fn bellman_ford(
        rows: usize,
        cols: usize,
        source: usize,
        table: &[(isize, isize)],
        field: &[f64],
    ) -> Vec<f64> {
        let shape = Grid::filled(rows, cols, ());
        let cells = rows * cols;
        let mut dist = vec![f64::INFINITY; cells];
        dist[source] = 0.0;
        for _ in 0..cells {
            let mut changed = false;
            for from in 0..cells {
                for &offset in table {
                    let Some(to) = shape.neighbour(from, offset) else {
                        continue;
                    };
                    let step = field[to] * length(cols, from, to);
                    if dist[from] + step < dist[to] {
                        dist[to] = dist[from] + step;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        dist
    }

    #[test]
    fn dijkstra_matches_bellman_ford() {
        let (rows, cols) = (7, 9);
        let mut rng = StageRng::for_key(1, "paths-dijkstra");
        let field: Vec<f64> = (0..rows * cols).map(|_| rng.range(0.1, 5.0)).collect();
        for (dirs, table) in [
            (Directions::Eight, &D8[..]),
            (Directions::Sixteen, &D16[..]),
        ] {
            let mut cost = |a: usize, b: usize| Some(field[b] * length(cols, a, b));
            let tree = shortest_paths(rows, cols, &[10], dirs, &mut cost, &mut no_extra, &|| false)
                .unwrap();
            let reference = bellman_ford(rows, cols, 10, table, &field);
            for (got, want) in tree.dist.iter().zip(&reference) {
                assert!((got - want).abs() < 1e-9, "{got} vs {want}");
            }
            let path = path_to(&tree, 62).unwrap();
            assert_eq!((path[0], *path.last().unwrap()), (10, 62));
        }
    }

    #[test]
    fn knight_steps_cost_their_own_length() {
        let mut cost = |a: usize, b: usize| Some(length(5, a, b));
        let target = 5 + 2; // one row down, two columns right of cell 0
        let eight = shortest_paths(
            5,
            5,
            &[0],
            Directions::Eight,
            &mut cost,
            &mut no_extra,
            &|| false,
        )
        .unwrap();
        let sixteen = shortest_paths(
            5,
            5,
            &[0],
            Directions::Sixteen,
            &mut cost,
            &mut no_extra,
            &|| false,
        )
        .unwrap();
        assert!((eight.dist[target] - (1.0 + 2.0_f64.sqrt())).abs() < 1e-12);
        assert!((sixteen.dist[target] - 5.0_f64.sqrt()).abs() < 1e-12);
        assert_eq!(path_to(&sixteen, target).unwrap(), vec![0, target]);
    }

    #[test]
    fn equal_paths_break_ties_by_index() {
        let run = || {
            let mut cost = |_: usize, _: usize| Some(1.0);
            shortest_paths(
                3,
                3,
                &[0],
                Directions::Eight,
                &mut cost,
                &mut no_extra,
                &|| false,
            )
            .unwrap()
        };
        let (a, b) = (run(), run());
        assert_eq!(a.prev, b.prev);
        // Cell 5 is reached at distance 2 from cells 1 and 4; cell 1 pops first.
        assert_eq!(a.prev[5], 1);
        assert_eq!(a.prev[0], u32::MAX);
    }

    #[test]
    fn cancel_is_checked_every_4096_pops() {
        let calls = Cell::new(0_usize);
        let mut cost = |_: usize, _: usize| Some(1.0);
        let cancel = || {
            calls.set(calls.get() + 1);
            false
        };
        shortest_paths(
            1,
            8192,
            &[0],
            Directions::Eight,
            &mut cost,
            &mut no_extra,
            &cancel,
        )
        .unwrap();
        assert_eq!(calls.get(), 2);
        let err = shortest_paths(
            1,
            8192,
            &[0],
            Directions::Eight,
            &mut cost,
            &mut no_extra,
            &|| true,
        );
        assert_eq!(err.unwrap_err(), KernelError::Cancelled);
    }

    #[test]
    fn forbidden_steps_extra_edges_and_unreachable_cells() {
        // A wall of forbidden cells in column 2 splits a 3 x 5 grid; a bridge crosses it.
        let wall = |b: usize| b % 5 == 2;
        let mut cost = |_: usize, b: usize| if wall(b) { None } else { Some(1.0) };
        let tree = shortest_paths(
            3,
            5,
            &[0],
            Directions::Eight,
            &mut cost,
            &mut no_extra,
            &|| false,
        )
        .unwrap();
        assert!(tree.dist[4].is_infinite());
        assert_eq!(path_to(&tree, 4), None);
        assert_eq!(path_to(&tree, 99), None);
        let mut extra = |i: usize| {
            if i == 1 {
                vec![(3, 0.5), (999, 1.0)]
            } else {
                Vec::new()
            }
        };
        let tree = shortest_paths(
            3,
            5,
            &[0],
            Directions::Eight,
            &mut cost,
            &mut extra,
            &|| false,
        )
        .unwrap();
        assert!((tree.dist[3] - 1.5).abs() < 1e-12);
        assert_eq!(path_to(&tree, 3).unwrap(), vec![0, 1, 3]);
    }

    #[test]
    fn several_sources_reach_the_nearest() {
        let mut cost = |_: usize, _: usize| Some(1.0);
        let tree = shortest_paths(
            1,
            9,
            &[0, 8, 0, 100],
            Directions::Eight,
            &mut cost,
            &mut no_extra,
            &|| false,
        )
        .unwrap();
        assert!((tree.dist[3] - 3.0).abs() < 1e-12);
        assert!((tree.dist[5] - 3.0).abs() < 1e-12);
        assert_eq!(path_to(&tree, 6).unwrap(), vec![8, 7, 6]);
    }
}
