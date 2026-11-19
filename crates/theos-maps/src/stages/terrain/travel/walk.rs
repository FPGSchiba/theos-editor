//! The walking graph: the time of a step over the terrain, and the search from the Chimes.

use std::collections::BTreeMap;

use super::{TravelParams, tobler_ms};
use crate::grid::Grid;
use crate::kernels::KernelError;
use crate::kernels::num::{max, to_f64, to_usize};
use crate::kernels::paths::{Directions, PathTree, shortest_paths};
use crate::stages::terrain::roads::{RoadNetwork, between, distance_m};
use crate::world::MapFrame;

/// The terrain and roads a walk crosses.
pub(super) struct Walk<'a> {
    height: &'a Grid<f64>,
    /// Stream and big-river cells.
    wet: Vec<bool>,
    lake: &'a Grid<bool>,
    /// The cells some road passes (bridge banks included).
    on_road: Vec<bool>,
    /// The bridges as extra edges `(to, seconds)` out of each bank, both ways.
    bridges: BTreeMap<usize, Vec<(usize, f64)>>,
    params: &'a TravelParams,
    frame: MapFrame,
}

impl<'a> Walk<'a> {
    /// The walk over the layers, with the roads of `network` when there is one.
    pub(super) fn new(
        height: &'a Grid<f64>,
        water: [&Grid<bool>; 2],
        lake: &'a Grid<bool>,
        network: Option<&RoadNetwork>,
        params: &'a TravelParams,
        frame: MapFrame,
    ) -> Self {
        let [river, big_river] = water;
        let wet = river
            .as_slice()
            .iter()
            .zip(big_river.as_slice())
            .map(|(&a, &b)| a || b)
            .collect();
        let mut walk = Self {
            height,
            wet,
            lake,
            on_road: vec![false; height.len()],
            bridges: BTreeMap::new(),
            params,
            frame,
        };
        for road in network.map_or(&[][..], |n| n.roads.as_slice()) {
            for &c in &road.cells {
                if let Some(on) = walk.on_road.get_mut(to_usize(c)) {
                    *on = true;
                }
            }
            for &(a, b) in &road.bridges {
                let (a, b) = (to_usize(a), to_usize(b));
                let seconds = walk.length_m(a, b) / walk.road_ms(tobler_ms(0.0));
                walk.bridges.entry(a).or_default().push((b, seconds));
                walk.bridges.entry(b).or_default().push((a, seconds));
            }
        }
        walk
    }

    /// The distance between the centres of two cells, m.
    fn length_m(&self, a: usize, b: usize) -> f64 {
        distance_m(&self.frame, a, b)
    }

    /// The cells the step from `a` to `b` crosses between its ends (see [`between`]).
    fn passed(&self, a: usize, b: usize) -> [Option<usize>; 2] {
        let (ra, ca) = self.height.cell(a);
        let (rb, cb) = self.height.cell(b);
        let signed = |i: usize| isize::try_from(i).unwrap_or(0);
        let offset = (signed(rb) - signed(ra), signed(cb) - signed(ca));
        between(self.height, a, offset)
    }

    /// Tobler's speed on a road: at least `road_min_speed_ms`, times `road_speedup`.
    fn road_ms(&self, tobler: f64) -> f64 {
        max(tobler, self.params.road_min_speed_ms) * self.params.road_speedup
    }

    /// The seconds of the step from `a` to `b`: its length over the speed, plus a ford for
    /// each stream or big-river cell it passes after its first cell (the cells between its ends,
    /// then `b`), and a lake penalty for a step that passes or ends in a lake cell.
    fn seconds(&self, a: usize, b: usize) -> f64 {
        let length = self.length_m(a, b);
        let heights = self.height.as_slice();
        let tobler = tobler_ms((heights[b] - heights[a]) / length);
        let speed = if self.on_road[a] && self.on_road[b] {
            self.road_ms(tobler)
        } else {
            tobler * self.params.off_path_factor
        };
        let mut time = length / speed;
        let passed = self.passed(a, b);
        let fords = passed
            .iter()
            .flatten()
            .chain([&b])
            .filter(|&&c| self.wet[c])
            .count();
        time += self.params.ford_penalty_s * to_f64(fords);
        let lake = self.lake.as_slice();
        if lake[a] || lake[b] || passed.iter().flatten().any(|&c| lake[c]) {
            time += self.params.lake_penalty_s;
        }
        time
    }

    /// The walking seconds from the nearest of `sources` to every cell.
    pub(super) fn tree(
        &self,
        sources: &[usize],
        cancel: &dyn Fn() -> bool,
    ) -> Result<PathTree, KernelError> {
        let mut step = |a: usize, b: usize| Some(self.seconds(a, b));
        // The kernel takes a list; an empty one does not allocate, so only bank cells clone.
        let mut extra = |a: usize| self.bridges.get(&a).map_or_else(Vec::new, Clone::clone);
        shortest_paths(
            self.height.rows(),
            self.height.cols(),
            sources,
            Directions::Sixteen,
            &mut step,
            &mut extra,
            cancel,
        )
    }
}
