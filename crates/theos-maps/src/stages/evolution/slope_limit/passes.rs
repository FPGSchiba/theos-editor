//! The passes of `slope_limit` over a worklist (spec §5.9 "Slope limit").
//!
//! A pass visits the cells from the highest to the lowest by their heights when it starts (ties
//! by index), and each moves half of its excess to its steepest neighbour. Whether a cell moves
//! depends only on its own height and its eight neighbours'. A cell that did not move when it
//! was last visited, and around which nothing has moved since, does not move now either, so a
//! pass needs to visit only the other cells (the *dirty* ones), in the same order: the result is
//! the same, bit for bit, as visiting every cell, while a pass touches a part of the map only
//! (Task 27b: a quarter of the cells on average on the 384² budget recipe).
//!
//! A move changes two heights, so the cells around both become dirty. Those still ahead in the
//! running pass join it at their place in the order; those behind it wait for the next pass.

use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

use super::{Books, Limits, MAX_PASSES, NEIGHBOURS, TOLERANCE};
use crate::grid::Grid;
use crate::kernels::num::{OrdF64, key};

/// Runs passes over `height` until it is stable or [`MAX_PASSES`] passes have run. Returns the
/// number of passes and whether the surface is stable: no cell that is not fixed exceeds its
/// critical slope by more than [`TOLERANCE`].
pub(super) fn relax(
    limits: &Limits<'_>,
    height: &mut Grid<f64>,
    books: &mut Books<'_>,
) -> (u32, bool) {
    let mut work = Worklist::new(height.len());
    let mut passes = 0;
    while work.violated(limits, height) {
        if passes == MAX_PASSES {
            return (passes, false);
        }
        work.pass(limits, height, books);
        passes += 1;
    }
    (passes, true)
}

/// A cell's place in a pass: higher first, then the lower index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Place {
    /// The cell's height when the pass started.
    height: OrdF64,
    /// The cell, reversed so that the lower index comes first.
    cell: Reverse<usize>,
}

/// The cells the next pass visits, and the bookkeeping of the running one.
struct Worklist {
    /// The dirty cells, each once.
    dirty: Vec<usize>,
    /// Whether a cell is in `dirty`.
    is_dirty: Vec<bool>,
    /// The number of the running pass, from 1.
    pass: u32,
    /// The pass in which a cell was last put in the queue.
    queued: Vec<u32>,
    /// The pass in which a cell's height last changed.
    moved: Vec<u32>,
    /// A cell's height when the pass in `moved` started.
    start: Vec<f64>,
}

impl Worklist {
    /// Every cell dirty: the first pass visits them all.
    fn new(cells: usize) -> Self {
        Self {
            dirty: (0..cells).collect(),
            is_dirty: vec![true; cells],
            pass: 0,
            queued: vec![0; cells],
            moved: vec![0; cells],
            start: vec![0.0; cells],
        }
    }

    /// Whether a dirty cell that is not fixed exceeds its critical slope by more than
    /// [`TOLERANCE`]; every other cell did not move at its last visit and nothing moved around
    /// it since.
    fn violated(&self, limits: &Limits<'_>, height: &Grid<f64>) -> bool {
        self.dirty
            .iter()
            .any(|&i| !limits.fixed[i] && limits.excess(height, i) > TOLERANCE)
    }

    /// One pass over the dirty cells and those that become dirty ahead of it.
    fn pass(&mut self, limits: &Limits<'_>, height: &mut Grid<f64>, books: &mut Books<'_>) {
        self.pass += 1;
        let mut queue = BinaryHeap::with_capacity(self.dirty.len());
        for i in std::mem::take(&mut self.dirty) {
            self.is_dirty[i] = false;
            if !limits.fixed[i] {
                self.queued[i] = self.pass;
                queue.push(self.place(height, i));
            }
        }
        while let Some(now) = queue.pop() {
            let Reverse(from) = now.cell;
            let Some((to, moved)) = limits.move_of(height, from) else {
                continue;
            };
            // A fixed neighbour is a sink and keeps its height.
            let changed = if limits.fixed[to] { 1 } else { 2 };
            for &cell in &[from, to][..changed] {
                self.remember(cell, height.as_slice()[cell]);
            }
            limits.shift(height, books, from, to, moved);
            for &cell in &[from, to][..changed] {
                self.mark(limits, height, now, cell, &mut queue);
                for (offset, _) in NEIGHBOURS {
                    if let Some(next) = height.neighbour(cell, offset) {
                        self.mark(limits, height, now, next, &mut queue);
                    }
                }
            }
        }
    }

    /// The place of `cell` in the running pass, by its height when the pass started.
    fn place(&self, height: &Grid<f64>, cell: usize) -> Place {
        let h = if self.moved[cell] == self.pass {
            self.start[cell]
        } else {
            height.as_slice()[cell]
        };
        Place {
            height: OrdF64(key(h)),
            cell: Reverse(cell),
        }
    }

    /// Records the height `before` of `cell` as its height at the start of the running pass,
    /// unless its height already changed in this pass.
    fn remember(&mut self, cell: usize, before: f64) {
        if self.moved[cell] != self.pass {
            self.moved[cell] = self.pass;
            self.start[cell] = before;
        }
    }

    /// Makes `cell` dirty after a move at the place `now`: it joins the running pass when its
    /// place lies ahead, and the next pass otherwise. Fixed cells never move.
    fn mark(
        &mut self,
        limits: &Limits<'_>,
        height: &Grid<f64>,
        now: Place,
        cell: usize,
        queue: &mut BinaryHeap<Place>,
    ) {
        if limits.fixed[cell] {
            return;
        }
        let place = self.place(height, cell);
        if place.cmp(&now) == Ordering::Less {
            if self.queued[cell] != self.pass {
                self.queued[cell] = self.pass;
                queue.push(place);
            }
        } else if !self.is_dirty[cell] {
            self.is_dirty[cell] = true;
            self.dirty.push(cell);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Worklist;
    use crate::grid::Grid;
    use crate::kernels::num::key;
    use crate::stages::evolution::slope_limit::{Books, Limits, NEIGHBOURS};

    /// A 5 x 5 plain at 0 m of 10 m cells with a 100 m tower in the middle (cell 12), critical
    /// slope 0.5, nothing fixed (synthetic).
    fn tower() -> Grid<f64> {
        let mut height = Grid::filled(5, 5, 0.0);
        height.set(2, 2, 100.0);
        height
    }

    /// One pass visiting every cell, highest first, ties by index (spec §5.9).
    fn every_cell(limits: &Limits<'_>, height: &mut Grid<f64>, books: &mut Books<'_>) {
        let mut order: Vec<usize> = (0..height.len()).collect();
        let h = height.as_slice();
        order.sort_by(|&a, &b| key(h[b]).total_cmp(&key(h[a])).then_with(|| a.cmp(&b)));
        for i in order {
            if let Some((to, moved)) = limits.move_of(height, i) {
                limits.shift(height, books, i, to, moved);
            }
        }
    }

    #[test]
    fn a_pass_from_one_dirty_cell_follows_its_moves_like_a_full_pass() {
        let (critical, fixed) = (vec![0.5; 25], vec![false; 25]);
        let limits = Limits {
            critical: &critical,
            fixed: &fixed,
            cell_m: 10.0,
        };
        let run = |sparse: bool| {
            let mut height = tower();
            let (mut erosion, mut deposition) = (Grid::filled(5, 5, 0.0), Grid::filled(5, 5, 0.0));
            let mut books = Books {
                erosion: &mut erosion,
                deposition: &mut deposition,
                removed_m: 0.0,
            };
            // Only the tower is dirty, as in a later pass where the plain was visited before.
            let mut work = Worklist::new(25);
            work.dirty = vec![12];
            work.is_dirty = (0..25).map(|i| i == 12).collect();
            if sparse {
                work.pass(&limits, &mut height, &mut books);
            } else {
                every_cell(&limits, &mut height, &mut books);
            }
            (height, work)
        };
        let (sparse, work) = run(true);
        let (full, _) = run(false);
        let bits = |g: &Grid<f64>| g.as_slice().iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&sparse), bits(&full));
        // The tower spilt north onto cell 7, which then spilt onto cell 2: both lay ahead in
        // the pass and joined it.
        assert!(
            sparse.get(1, 2) > 0.0 && sparse.get(0, 2) > 0.0,
            "{sparse:?}"
        );
        assert_eq!((work.queued[7], work.queued[2]), (1, 1));
        // Every neighbour of the lowered tower is dirty for the next pass or was visited after
        // the move in this one, the diagonal ones included.
        for (offset, _) in NEIGHBOURS {
            let n = sparse.neighbour(12, offset).unwrap();
            assert!(work.is_dirty[n] || work.queued[n] == 1, "neighbour {n}");
        }
        assert!(work.is_dirty[12], "the tower itself is checked again");
    }
}
