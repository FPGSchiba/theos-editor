//! Priority-flood with epsilon (Barnes, Lehman and Mulla 2014).
//!
//! Starting from the seeds, the lowest queued cell is popped and each unvisited neighbour gets
//! `max(own height, popped + epsilon)`. Every non-seed cell then has a neighbour at least
//! `epsilon` lower, so water drains to a seed. Ties pop by cell index. [`priority_flood_where`]
//! is the same flood in place, over the part of a surface its caller lets it enter.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::grid::{D8, Grid};
use crate::kernels::num::{OrdF64, key, max};

/// The result of [`priority_flood`].
#[derive(Debug, Clone, PartialEq)]
pub struct Flooded {
    /// The surface with every pit filled and every flat sloped by `epsilon`.
    pub surface: Grid<f64>,
    /// The cells in the order they were popped.
    pub order: Vec<u32>,
}

/// Floods `height` from `seeds` (cell indices; those off the grid are ignored). With no valid
/// seed the surface comes back unchanged and `order` is empty. `order` holds `u32` cell numbers,
/// so grids of more than `u32::MAX` cells are not supported.
#[must_use]
pub fn priority_flood(height: &Grid<f64>, seeds: &[usize], epsilon: f64) -> Flooded {
    let mut surface = height.clone();
    let mut visited = vec![false; height.len()];
    let order = priority_flood_where(&mut surface, &mut visited, seeds, epsilon, |_, _| true);
    Flooded { surface, order }
}

/// [`priority_flood`] in place over part of `surface`: the flood enters an unvisited neighbour
/// `j` of a cell popped at `level` only when `enter(j, level)` holds, so the caller decides how
/// far it spreads. Seeds start at their value in `surface`; seeds off the grid or already
/// visited are ignored. `visited` holds one flag per cell; the flood marks the seeds and every
/// cell it enters, so floods over disjoint parts can share it. Returns the cells in the order
/// they were popped.
pub fn priority_flood_where(
    surface: &mut Grid<f64>,
    visited: &mut [bool],
    seeds: &[usize],
    epsilon: f64,
    mut enter: impl FnMut(usize, f64) -> bool,
) -> Vec<u32> {
    let mut heap: BinaryHeap<Reverse<(OrdF64, usize)>> = BinaryHeap::new();
    for &s in seeds.iter().filter(|&&s| s < surface.len()) {
        if !visited[s] {
            visited[s] = true;
            heap.push(Reverse((OrdF64(key(surface.as_slice()[s])), s)));
        }
    }
    let mut order = Vec::with_capacity(heap.len());
    while let Some(Reverse((OrdF64(level), i))) = heap.pop() {
        order.push(u32::try_from(i).unwrap_or(u32::MAX));
        for &offset in &D8 {
            let Some(j) = surface.neighbour(i, offset) else {
                continue;
            };
            if visited[j] || !enter(j, level) {
                continue;
            }
            visited[j] = true;
            let raised = max(surface.as_slice()[j], level + epsilon);
            surface.as_mut_slice()[j] = raised;
            heap.push(Reverse((OrdF64(key(raised)), j)));
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::D8;
    use crate::rng::StageRng;

    fn random_surface(rows: usize, cols: usize, name: &str) -> Grid<f64> {
        let mut rng = StageRng::for_key(1, name);
        let data = (0..rows * cols).map(|_| rng.range(0.0, 100.0)).collect();
        Grid::from_vec(rows, cols, data).unwrap()
    }

    fn edge_cells(rows: usize, cols: usize) -> Vec<usize> {
        (0..rows * cols)
            .filter(|&i| {
                let (r, c) = (i / cols, i % cols);
                r == 0 || c == 0 || r + 1 == rows || c + 1 == cols
            })
            .collect()
    }

    #[test]
    fn the_flooded_surface_has_no_interior_pit() {
        let eps = 1e-4;
        for k in 0..5 {
            let height = random_surface(32, 32, &format!("flood-{k}"));
            let seeds = edge_cells(32, 32);
            let flooded = priority_flood(&height, &seeds, eps);
            for i in 0..height.len() {
                let (own, h) = (flooded.surface.as_slice()[i], height.as_slice()[i]);
                assert!(own >= h);
                if seeds.contains(&i) {
                    assert_eq!(own.to_bits(), h.to_bits());
                    continue;
                }
                let drains = D8.iter().any(|&d| {
                    height
                        .neighbour(i, d)
                        .is_some_and(|j| flooded.surface.as_slice()[j] + eps <= own)
                });
                assert!(drains, "cell {i} is a pit");
            }
        }
    }

    #[test]
    fn flood_order_starts_at_the_seeds() {
        let height = random_surface(16, 16, "flood-order");
        let seeds = [200, 17, 5];
        let flooded = priority_flood(&height, &seeds, 1e-3);
        let lowest = seeds
            .iter()
            .copied()
            .min_by(|&a, &b| {
                height.as_slice()[a]
                    .total_cmp(&height.as_slice()[b])
                    .then(a.cmp(&b))
            })
            .unwrap();
        assert_eq!(flooded.order[0] as usize, lowest);
        let mut sorted = flooded.order.clone();
        sorted.sort_unstable();
        assert!(sorted.iter().enumerate().all(|(i, &v)| v as usize == i));
    }

    #[test]
    fn a_pit_is_raised_and_ties_pop_by_index() {
        let height = Grid::from_vec(1, 5, vec![0.0, 5.0, 1.0, 5.0, 0.0]).unwrap();
        let flooded = priority_flood(&height, &[0, 4], 0.5);
        assert!((flooded.surface.get(0, 2) - 5.5).abs() < 1e-12);
        assert_eq!(flooded.order, vec![0, 4, 1, 3, 2]);
    }

    #[test]
    fn a_flood_where_enters_only_the_cells_it_is_allowed_to() {
        // Seeded at cell 0 raised to 4; a cell is entered only when the flood would raise it,
        // so the wall at cell 4 and the low cell 5 behind it stay out.
        let height = Grid::from_vec(1, 6, vec![0.0, 2.0, 1.0, 3.0, 9.0, 0.5]).unwrap();
        let mut surface = height.clone();
        surface.as_mut_slice()[0] = 4.0;
        let mut visited = vec![false; 6];
        let order = priority_flood_where(&mut surface, &mut visited, &[0], 0.5, |j, level| {
            height.as_slice()[j] < level + 0.5
        });
        assert_eq!(order, vec![0, 1, 2, 3]);
        let lifted: Vec<f64> = surface.as_slice().to_vec();
        let expected: [f64; 6] = [4.0, 4.5, 5.0, 5.5, 9.0, 0.5];
        for (got, want) in lifted.iter().zip(expected) {
            assert_eq!(got.to_bits(), want.to_bits(), "{lifted:?}");
        }
        assert_eq!(visited, [true, true, true, true, false, false]);
        // A second flood shares `visited`: it never enters a cell the first one took.
        let again = priority_flood_where(&mut surface, &mut visited, &[5, 3], 0.5, |_, _| true);
        assert_eq!(again, vec![5, 4]);
        assert_eq!(surface.as_slice()[4].to_bits(), 9.0_f64.to_bits());
        assert!(visited.iter().all(|&v| v));
    }

    #[test]
    fn priority_flood_is_the_flood_where_every_cell_may_be_entered() {
        let height = random_surface(16, 16, "flood-where");
        let seeds = edge_cells(16, 16);
        let flooded = priority_flood(&height, &seeds, 1e-3);
        let mut surface = height.clone();
        let mut visited = vec![false; height.len()];
        let order = priority_flood_where(&mut surface, &mut visited, &seeds, 1e-3, |_, _| true);
        assert_eq!(order, flooded.order);
        assert_eq!(surface, flooded.surface);
    }

    #[test]
    fn no_seeds_leaves_the_surface_alone() {
        let height = random_surface(4, 4, "flood-empty");
        let flooded = priority_flood(&height, &[], 0.1);
        assert_eq!(flooded.surface, height);
        assert_eq!(flooded.order, Vec::<u32>::new());
        let skipped = priority_flood(&height, &[999], 0.1);
        assert_eq!(skipped.order, Vec::<u32>::new());
    }
}
