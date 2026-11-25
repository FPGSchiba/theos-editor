//! Nearest-seed labelling and Delaunay edges, both through spade's Delaunay triangulation.
//!
//! Points are inserted in index order. A point equal to an earlier one gets that vertex back from
//! spade, and the vertex keeps the lowest index of its points. When a query is exactly
//! equidistant from two vertices, the answer is the one spade's walk reaches; that choice is
//! deterministic for a given insertion order, and no further tie-break by index is applied.
//!
//! The work is sized by the number of points, which a recipe count sets (a million provinces),
//! so building the triangulation and querying it per cell both call `cancel` every 4,096
//! inserts or queries.

use spade::{DelaunayTriangulation, Point2, Triangulation};

use super::{KernelError, cancel_point};
use crate::grid::Grid;

/// A triangulated point set that answers nearest-point queries with original indices.
pub(super) struct PointIndex {
    triangulation: DelaunayTriangulation<Point2<f64>>,
    /// For every vertex (by its handle index) the lowest index of the points that sit there.
    first_index: Vec<usize>,
}

impl PointIndex {
    /// Triangulates `points`, calling `cancel` every [`CANCEL_EVERY`](super::CANCEL_EVERY)
    /// inserts.
    ///
    /// # Errors
    /// [`KernelError::Empty`] for no points, [`KernelError::InvalidPoint`] for a point spade
    /// refuses, [`KernelError::Cancelled`] when `cancel` returns `true`.
    pub(super) fn new(points: &[[f64; 2]], cancel: &dyn Fn() -> bool) -> Result<Self, KernelError> {
        if points.is_empty() {
            return Err(KernelError::Empty);
        }
        let mut triangulation = DelaunayTriangulation::<Point2<f64>>::new();
        let mut first_index: Vec<usize> = Vec::new();
        for (index, p) in points.iter().enumerate() {
            cancel_point(index, cancel)?;
            let handle = triangulation
                .insert(Point2::new(p[0], p[1]))
                .map_err(|_| KernelError::InvalidPoint { index })?;
            if handle.index() == first_index.len() {
                first_index.push(index);
            }
        }
        Ok(Self {
            triangulation,
            first_index,
        })
    }

    /// The original index of the point nearest to `q`; `None` for a non-finite query.
    pub(super) fn nearest(&self, q: [f64; 2]) -> Option<usize> {
        if !(q[0].is_finite() && q[1].is_finite()) {
            return None;
        }
        let vertex = self
            .triangulation
            .nearest_neighbor(Point2::new(q[0], q[1]))?;
        self.first_index.get(vertex.fix().index()).copied()
    }

    /// The original index of the point nearest to every cell, with cell `i` at `(x[i], y[i])`,
    /// in cell order; `cancel` is called every [`CANCEL_EVERY`](super::CANCEL_EVERY) cells.
    ///
    /// # Errors
    /// [`KernelError::Shape`] when the grids differ in shape, [`KernelError::InvalidPoint`] for a
    /// cell coordinate that is not finite, [`KernelError::Cancelled`] when `cancel` returns
    /// `true`.
    pub(super) fn nearest_per_cell(
        &self,
        x: &Grid<f64>,
        y: &Grid<f64>,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<usize>, KernelError> {
        same_shape(x, y)?;
        let mut out = Vec::with_capacity(x.len());
        for (cell, (&qx, &qy)) in x.as_slice().iter().zip(y.as_slice()).enumerate() {
            cancel_point(cell, cancel)?;
            let nearest = self
                .nearest([qx, qy])
                .ok_or(KernelError::InvalidPoint { index: cell })?;
            out.push(nearest);
        }
        Ok(out)
    }
}

/// Checks that two grids have the same shape.
pub(super) fn same_shape<T: Copy, U: Copy>(a: &Grid<T>, b: &Grid<U>) -> Result<(), KernelError> {
    if a.rows() == b.rows() && a.cols() == b.cols() {
        Ok(())
    } else {
        Err(KernelError::Shape {
            rows: a.rows(),
            cols: a.cols(),
            len: b.len(),
        })
    }
}

/// The index of the seed nearest to every cell, with cell `(r, c)` at `(x, y)` from the grids.
/// Duplicate seeds give their lowest index. Ties follow spade's deterministic choice.
///
/// # Errors
/// [`KernelError::Empty`] for no seeds, [`KernelError::Shape`] when the grids differ in shape,
/// [`KernelError::InvalidPoint`] for a seed or a cell coordinate that is not finite,
/// [`KernelError::TooMany`] for more seeds than a `u32` numbers, [`KernelError::Cancelled`]
/// when `cancel` returns `true` (called every 4,096 seeds and every 4,096 cells).
pub fn nearest_seed(
    seeds: &[[f64; 2]],
    x: &Grid<f64>,
    y: &Grid<f64>,
    cancel: &dyn Fn() -> bool,
) -> Result<Grid<u32>, KernelError> {
    same_shape(x, y)?;
    u32::try_from(seeds.len()).map_err(|_| KernelError::TooMany { len: seeds.len() })?;
    let index = PointIndex::new(seeds, cancel)?;
    let labels = index
        .nearest_per_cell(x, y, cancel)?
        .into_iter()
        // The count was checked above, so every index fits.
        .map(|seed| u32::try_from(seed).map_err(|_| KernelError::TooMany { len: seeds.len() }))
        .collect::<Result<Vec<u32>, KernelError>>()?;
    Grid::from_vec(x.rows(), x.cols(), labels).map_err(|_| KernelError::Shape {
        rows: x.rows(),
        cols: x.cols(),
        len: x.len(),
    })
}

/// The edges of the Delaunay triangulation of `points` as index pairs `(a, b)` with `a < b`,
/// sorted. A duplicate point stands for the lowest index of its copies.
///
/// # Errors
/// [`KernelError::InvalidPoint`] for a point spade refuses (NaN, infinity, out of range),
/// [`KernelError::Cancelled`] when `cancel` returns `true` (called every 4,096 points).
pub fn delaunay_edges(
    points: &[[f64; 2]],
    cancel: &dyn Fn() -> bool,
) -> Result<Vec<(usize, usize)>, KernelError> {
    if points.is_empty() {
        return Ok(Vec::new());
    }
    let index = PointIndex::new(points, cancel)?;
    let mut edges: Vec<(usize, usize)> = index
        .triangulation
        .undirected_edges()
        .filter_map(|edge| {
            let [a, b] = edge.vertices();
            let a = index.first_index.get(a.fix().index()).copied()?;
            let b = index.first_index.get(b.fix().index()).copied()?;
            Some(if a < b { (a, b) } else { (b, a) })
        })
        .collect();
    edges.sort_unstable();
    edges.dedup();
    Ok(edges)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lcg(state: &mut u64) -> f64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        crate::kernels::num::unit_from_bits(*state)
    }

    #[test]
    fn nearest_seed_matches_brute_force() -> Result<(), KernelError> {
        let mut s = 7_u64;
        let seeds: Vec<[f64; 2]> = (0..20).map(|_| [lcg(&mut s), lcg(&mut s)]).collect();
        let (rows, cols) = (24, 24);
        let xs: Vec<f64> = (0..rows * cols).map(|_| lcg(&mut s)).collect();
        let ys: Vec<f64> = (0..rows * cols).map(|_| lcg(&mut s)).collect();
        let x = Grid::from_vec(rows, cols, xs.clone()).unwrap();
        let y = Grid::from_vec(rows, cols, ys.clone()).unwrap();
        let labels = nearest_seed(&seeds, &x, &y, &|| false)?;
        for i in 0..rows * cols {
            let d = |k: usize| {
                let (dx, dy) = (seeds[k][0] - xs[i], seeds[k][1] - ys[i]);
                dx * dx + dy * dy
            };
            let mut best = 0;
            for k in 1..seeds.len() {
                if d(k) < d(best) {
                    best = k;
                }
            }
            assert_eq!(labels.as_slice()[i] as usize, best);
        }
        Ok(())
    }

    #[test]
    fn nearest_seed_gives_duplicates_their_lowest_index() -> Result<(), KernelError> {
        let seeds = [[0.0, 0.0], [5.0, 5.0], [0.0, 0.0]];
        let x = Grid::from_vec(1, 2, vec![0.1, 4.9]).unwrap();
        let y = Grid::from_vec(1, 2, vec![0.1, 4.9]).unwrap();
        let labels = nearest_seed(&seeds, &x, &y, &|| false)?;
        assert_eq!(labels.as_slice(), &[0, 1]);
        Ok(())
    }

    #[test]
    fn nearest_seed_refuses_bad_input() {
        let one = Grid::filled(1, 1, 0.0);
        let two = Grid::filled(1, 2, 0.0);
        assert_eq!(
            nearest_seed(&[], &one, &one, &|| false),
            Err(KernelError::Empty)
        );
        assert!(matches!(
            nearest_seed(&[[0.0, 0.0]], &one, &two, &|| false),
            Err(KernelError::Shape { .. })
        ));
        assert_eq!(
            nearest_seed(&[[f64::NAN, 0.0]], &one, &one, &|| false),
            Err(KernelError::InvalidPoint { index: 0 })
        );
        let nan = Grid::filled(1, 1, f64::NAN);
        assert_eq!(
            nearest_seed(&[[0.0, 0.0]], &nan, &one, &|| false),
            Err(KernelError::InvalidPoint { index: 0 })
        );
    }

    #[test]
    fn delaunay_of_a_square_and_its_centre_has_eight_edges() -> Result<(), KernelError> {
        let pts = [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [1.0, 1.0]];
        let edges = delaunay_edges(&pts, &|| false)?;
        assert_eq!(
            edges,
            vec![
                (0, 1),
                (0, 3),
                (0, 4),
                (1, 2),
                (1, 4),
                (2, 3),
                (2, 4),
                (3, 4)
            ]
        );
        Ok(())
    }

    #[test]
    fn delaunay_edges_use_the_lowest_index_of_duplicates() -> Result<(), KernelError> {
        let pts = [[0.0, 0.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]];
        assert_eq!(
            delaunay_edges(&pts, &|| false)?,
            vec![(0, 1), (0, 3), (1, 3)]
        );
        assert_eq!(delaunay_edges(&[[0.0, 0.0]], &|| false)?.len(), 0);
        assert_eq!(
            delaunay_edges(&[[0.0, 0.0], [f64::NAN, 1.0]], &|| false),
            Err(KernelError::InvalidPoint { index: 1 })
        );
        Ok(())
    }

    // Final review round 4, finding 3: a million provinces are a million inserts, so the
    // triangulation and the per-cell queries reach a cancel point every 4,096 of them.
    #[test]
    fn cancel_is_checked_every_4096_inserts_and_queries() {
        let mut s = 3_u64;
        let seeds: Vec<[f64; 2]> = (0..8192).map(|_| [lcg(&mut s), lcg(&mut s)]).collect();
        let queries = Grid::filled(1, 4097, 0.5);
        let calls = std::cell::Cell::new(0_usize);
        let count = || {
            calls.set(calls.get() + 1);
            false
        };
        nearest_seed(&seeds, &queries, &queries, &count).unwrap();
        assert_eq!(calls.get(), 4, "inserts 0 and 4096, queries 0 and 4096");
        calls.set(0);
        delaunay_edges(&seeds, &count).unwrap();
        assert_eq!(calls.get(), 2);
        let stop = || true;
        assert_eq!(
            nearest_seed(&seeds, &queries, &queries, &stop),
            Err(KernelError::Cancelled)
        );
        assert_eq!(delaunay_edges(&seeds, &stop), Err(KernelError::Cancelled));
        let index = PointIndex::new(&seeds, &|| false).unwrap();
        let after_first = || calls.replace(calls.get() + 1) > 0;
        calls.set(0);
        assert_eq!(
            index.nearest_per_cell(&queries, &queries, &after_first),
            Err(KernelError::Cancelled)
        );
    }
}
