//! Laying routed paths as roads: the road list, the `roads` raster and the bridges built.

use std::collections::BTreeSet;

use super::route::bridge_key;
use super::{Road, RoadClass, Router, StepCost};
use crate::features::FeatureHandle;
use crate::grid::Grid;
use crate::kernels::num::{to_u32, to_usize};

/// The roads laid so far, the raster of their classes and the bridges built.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Laid {
    /// The roads in the order laid.
    pub(crate) roads: Vec<Road>,
    /// The `roads` raster: 0, or the highest class code of the roads on the cell.
    pub(crate) raster: Grid<u8>,
    /// The bridges built, as [`bridge_key`]s.
    pub(crate) built: BTreeSet<(u32, u32)>,
}

impl Laid {
    /// No road on a `rows` x `cols` grid.
    pub(crate) fn new(rows: usize, cols: usize) -> Self {
        Self {
            roads: Vec::new(),
            raster: Grid::filled(rows, cols, 0),
            built: BTreeSet::new(),
        }
    }

    /// The roads of a network already built and their `raster`: later roads merge with them and
    /// reuse their bridges.
    pub(crate) fn resume(roads: Vec<Road>, raster: Grid<u8>) -> Self {
        let built = roads
            .iter()
            .flat_map(|r| r.bridges.iter())
            .map(|&(a, b)| bridge_key(to_usize(a), to_usize(b)))
            .collect();
        Self {
            roads,
            raster,
            built,
        }
    }

    /// The router over `cost` that sees the roads and bridges laid so far.
    pub(crate) fn router<'a>(&'a self, cost: &'a StepCost<'a>) -> Router<'a> {
        Router {
            cost,
            roads: &self.raster,
            built: &self.built,
        }
    }

    /// Lays the road of `class` along `path` from `from` to `to`: its cells, the bridges among
    /// its steps, and its class on the raster at every cell it passes (for a knight move the
    /// cell between its ends that touches both, for a bridge every cell under it). Returns the
    /// road's index in the list.
    pub(crate) fn add(
        &mut self,
        cost: &StepCost<'_>,
        path: &[usize],
        class: RoadClass,
        from: FeatureHandle,
        to: FeatureHandle,
    ) -> usize {
        let mut marked: Vec<usize> = path.to_vec();
        let mut bridges = Vec::new();
        for pair in path.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if cost.is_bridge(a, b) {
                bridges.push((to_u32(a), to_u32(b)));
                self.built.insert(bridge_key(a, b));
                marked.extend(cost.under(a, b));
            } else if let Some(offset) = cost.offset(a, b)
                && offset.0.abs() * offset.1.abs() == 2
                && let [Some(beside), _] = cost.between(a, offset)
            {
                marked.push(beside);
            }
        }
        let code = class.code();
        for i in marked {
            let v = &mut self.raster.as_mut_slice()[i];
            *v = (*v).max(code);
        }
        self.roads.push(Road {
            class,
            cells: path.iter().map(|&i| to_u32(i)).collect(),
            from,
            to,
            bridges,
        });
        self.roads.len() - 1
    }

    /// Every place some road starts or ends at, in one pass over the roads.
    pub(crate) fn reached(&self) -> BTreeSet<FeatureHandle> {
        self.roads.iter().flat_map(|r| [r.from, r.to]).collect()
    }

    /// Every cell of every road's path.
    pub(crate) fn cells(&self) -> BTreeSet<usize> {
        self.roads
            .iter()
            .flat_map(|r| r.cells.iter().map(|&c| to_usize(c)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::cost::tests::{Layers, N, at};
    use super::super::{RoadsParams, StepCost};
    use super::*;

    #[test]
    fn laying_marks_the_raster_bridges_and_knight_moves() {
        let mut layers = Layers::flat(N);
        for r in 0..N {
            layers = layers.wet("big_river", at(r, 8));
        }
        let params = RoadsParams::default();
        let cost = StepCost::new(layers.terrain(), &params, &layers.frame());
        let mut laid = Laid::new(N, N);
        // A knight move, a step east, a bridge over column 8, a diagonal.
        let path = [at(4, 4), at(5, 6), at(5, 7), at(5, 9), at(6, 10)];
        let (a, b) = (FeatureHandle(1), FeatureHandle(2));
        laid.add(&cost, &path, RoadClass::Road, a, b);
        laid.add(&cost, &[at(5, 6), at(5, 5)], RoadClass::Main, b, a);
        laid.add(&cost, &[at(5, 5), at(5, 4)], RoadClass::Trail, b, a);
        let road = &laid.roads[0];
        assert_eq!(road.bridges, [(to_u32(at(5, 7)), to_u32(at(5, 9)))]);
        assert!(laid.built.contains(&bridge_key(at(5, 9), at(5, 7))));
        let marked: Vec<(usize, u8)> = laid
            .raster
            .as_slice()
            .iter()
            .enumerate()
            .filter(|&(_, &v)| v > 0)
            .map(|(i, &v)| (i, v))
            .collect();
        let want = [
            (at(4, 4), 3),
            (at(4, 5), 3),
            (at(5, 4), 1),
            (at(5, 5), 4),
            (at(5, 6), 4),
            (at(5, 7), 3),
            (at(5, 8), 3),
            (at(5, 9), 3),
            (at(6, 10), 3),
        ];
        assert_eq!(marked, want);
        assert!(laid.reached().contains(&a) && !laid.reached().contains(&FeatureHandle(3)));
        assert_eq!(laid.cells().len(), 7);
    }
}
