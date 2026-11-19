//! Routing one road over the step costs, cheaper where roads already run.

use std::collections::BTreeSet;

use super::{RoadClass, StepCost};
use crate::grid::Grid;
use crate::kernels::KernelError;
use crate::kernels::num::to_u32;
use crate::kernels::paths::{Directions, PathTree, path_to, shortest_paths};

/// Least-cost routing of roads in 16 directions with bridge edges. A step costs
/// `merge_discount` times as much on cells that carry a road (the mean of its two ends' shares),
/// and so does a bridge already built.
#[derive(Debug, Clone, Copy)]
pub struct Router<'a> {
    /// The step and bridge costs.
    pub cost: &'a StepCost<'a>,
    /// The classes of the roads laid so far per cell (the `roads` raster); above 0 is a road.
    pub roads: &'a Grid<u8>,
    /// The bridges built so far, each as `(lower cell, higher cell)` ([`bridge_key`]).
    pub built: &'a BTreeSet<(u32, u32)>,
}

impl Router<'_> {
    /// The least costs of a road of `class` from the nearest of `sources` to every cell.
    ///
    /// # Errors
    /// [`KernelError::Cancelled`] when `cancel` returns `true` (checked every 4,096 cells);
    /// [`KernelError::TooMany`] for a grid of more cells than a `u32` numbers.
    pub fn tree(
        &self,
        sources: &[usize],
        class: RoadClass,
        cancel: &dyn Fn() -> bool,
    ) -> Result<PathTree, KernelError> {
        self.search(sources, class, true, cancel)
    }

    /// The least costs as [`Router::tree`] gives them, but without the merge discount: roads and
    /// built bridges cost what new ones would. A loop is routed this way, so it is a new road.
    ///
    /// # Errors
    /// As [`Router::tree`].
    pub fn tree_fresh(
        &self,
        sources: &[usize],
        class: RoadClass,
        cancel: &dyn Fn() -> bool,
    ) -> Result<PathTree, KernelError> {
        self.search(sources, class, false, cancel)
    }

    /// The shortest-path search, with or without the merge discount.
    fn search(
        &self,
        sources: &[usize],
        class: RoadClass,
        merge: bool,
        cancel: &dyn Fn() -> bool,
    ) -> Result<PathTree, KernelError> {
        let discount = if merge {
            self.cost.params.merge_discount
        } else {
            1.0
        };
        let roads = self.roads.as_slice();
        let share = |i: usize| if roads[i] > 0 { discount } else { 1.0 };
        let mut step = |a: usize, b: usize| {
            self.cost
                .step(a, b, class)
                .map(|c| c * 0.5 * (share(a) + share(b)))
        };
        let mut extra = |a: usize| {
            let mut out = self.cost.bridges(a);
            for (b, c) in &mut out {
                if self.built.contains(&bridge_key(a, *b)) {
                    *c *= discount;
                }
            }
            out
        };
        let grid = self.cost.grid();
        shortest_paths(
            grid.rows(),
            grid.cols(),
            sources,
            Directions::Sixteen,
            &mut step,
            &mut extra,
            cancel,
        )
    }

    /// The cells of the least-cost road of `class` from the nearest of `sources` to `target`,
    /// both included; `None` when no road reaches `target`.
    ///
    /// # Errors
    /// As [`Router::tree`].
    pub fn route(
        &self,
        sources: &[usize],
        target: usize,
        class: RoadClass,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Option<Vec<usize>>, KernelError> {
        Ok(path_to(&self.tree(sources, class, cancel)?, target))
    }

    /// As [`Router::route`], without the merge discount ([`Router::tree_fresh`]).
    ///
    /// # Errors
    /// As [`Router::tree`].
    pub fn route_fresh(
        &self,
        sources: &[usize],
        target: usize,
        class: RoadClass,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Option<Vec<usize>>, KernelError> {
        Ok(path_to(&self.tree_fresh(sources, class, cancel)?, target))
    }
}

/// A bridge between the cells `a` and `b` as `(lower cell, higher cell)`, whichever way it is
/// crossed.
#[must_use]
pub fn bridge_key(a: usize, b: usize) -> (u32, u32) {
    (to_u32(a.min(b)), to_u32(a.max(b)))
}

#[cfg(test)]
mod tests {
    use super::super::cost::tests::Layers;
    use super::super::{RoadsParams, StepCost};
    use super::*;

    const N: usize = 64;

    fn at(r: usize, c: usize) -> usize {
        r * N + c
    }

    #[test]
    fn trunks_merge_on_existing_roads() {
        let layers = Layers::flat(N);
        let mut roads = Grid::filled(N, N, 0_u8);
        for c in 5..59 {
            roads.set(20, c, 4);
        }
        let built = BTreeSet::new();
        // A track parallel to a main road 8 rows away.
        let shared = |discount: f64| {
            let params = RoadsParams {
                merge_discount: discount,
                ..Default::default()
            };
            let cost = StepCost::new(layers.terrain(), &params, 100.0);
            let router = Router {
                cost: &cost,
                roads: &roads,
                built: &built,
            };
            let path = router
                .route(&[at(28, 5)], at(28, 58), RoadClass::Track, &|| false)
                .unwrap()
                .unwrap();
            path.iter().filter(|&&i| roads.as_slice()[i] > 0).count()
        };
        assert!(shared(0.35) >= 30, "{}", shared(0.35));
        assert_eq!(shared(1.0), 0);
    }

    #[test]
    fn fresh_routes_ignore_the_merge_discount() {
        let layers = Layers::flat(N);
        let mut roads = Grid::filled(N, N, 0_u8);
        for c in 5..59 {
            roads.set(20, c, 4);
        }
        let built = BTreeSet::new();
        let params = RoadsParams::default();
        let cost = StepCost::new(layers.terrain(), &params, 100.0);
        let router = Router {
            cost: &cost,
            roads: &roads,
            built: &built,
        };
        let path = router
            .route_fresh(&[at(28, 5)], at(28, 58), RoadClass::Track, &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(path.iter().filter(|&&i| roads.as_slice()[i] > 0).count(), 0);
    }

    #[test]
    fn a_built_bridge_is_cheaper_and_cancel_stops_the_search() {
        let mut layers = Layers::flat(N);
        for r in 0..N {
            layers = layers.wet("big_river", at(r, 30));
        }
        let params = RoadsParams::default();
        let cost = StepCost::new(layers.terrain(), &params, 100.0);
        let roads = Grid::filled(N, N, 0_u8);
        let mut built = BTreeSet::new();
        let across = |built: &BTreeSet<(u32, u32)>| {
            let router = Router {
                cost: &cost,
                roads: &roads,
                built,
            };
            let tree = router
                .tree(&[at(10, 29)], RoadClass::Main, &|| false)
                .unwrap();
            tree.dist[at(10, 31)]
        };
        let new = across(&built);
        built.insert(bridge_key(at(10, 31), at(10, 29)));
        assert_eq!(
            bridge_key(at(10, 31), at(10, 29)),
            bridge_key(at(10, 29), at(10, 31))
        );
        assert!((across(&built) - 0.35 * new).abs() < 1e-9);
        // Every one of the 4,096 cells of a dry grid is popped, so the search asks once.
        let dry = Layers::flat(N);
        let cost = StepCost::new(dry.terrain(), &params, 100.0);
        let router = Router {
            cost: &cost,
            roads: &roads,
            built: &built,
        };
        let stopped = router.route(&[0], at(63, 63), RoadClass::Main, &|| true);
        assert_eq!(stopped, Err(KernelError::Cancelled));
    }
}
