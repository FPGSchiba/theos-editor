//! The cost of one road step over the terrain (spec §5.9 "Roads").

use super::{RoadClass, RoadsParams};
use crate::grid::{D16, Grid};
use crate::kernels::num::to_f64;

/// The terrain layers a road's cost reads, all of the frame's shape.
#[derive(Debug, Clone, Copy)]
pub struct Terrain<'a> {
    /// Height, m.
    pub height: &'a Grid<f64>,
    /// Vegetation density, 0 to 1.
    pub vegetation: &'a Grid<f64>,
    /// River cells, big rivers included.
    pub river: &'a Grid<bool>,
    /// Big-river cells.
    pub big_river: &'a Grid<bool>,
    /// Lake cells.
    pub lake: &'a Grid<bool>,
    /// The D8 flow direction codes, 1 to 8 for N to NW, 0 for none (plan ruling L19).
    pub flow_dir: &'a Grid<u8>,
}

/// The costs of road steps and bridges over one terrain with one set of parameters; `sites`
/// routes its spur tracks with it too.
#[derive(Debug, Clone, Copy)]
pub struct StepCost<'a> {
    pub(super) terrain: Terrain<'a>,
    pub(super) params: &'a RoadsParams,
    pub(super) cell_m: f64,
}

impl<'a> StepCost<'a> {
    /// The costs over `terrain` with `params`, on cells of `cell_m` metres.
    #[must_use]
    pub fn new(terrain: Terrain<'a>, params: &'a RoadsParams, cell_m: f64) -> Self {
        Self {
            terrain,
            params,
            cell_m,
        }
    }

    /// The cost of the step from cell `from` to cell `to` for a road of `class`: see the
    /// [module docs](super). `None` when `to` is not one of the 16 neighbours of `from`, when
    /// the step passes a lake or big-river cell (its ends included), or when it is steeper than
    /// the class's `max_grade`.
    #[must_use]
    pub fn step(&self, from: usize, to: usize, class: RoadClass) -> Option<f64> {
        let offset = self.offset(from, to)?;
        let between = self.between(from, offset);
        let mut passed = between.iter().flatten().chain([&from, &to]);
        if passed.any(|&c| self.is_water(c)) {
            return None;
        }
        let length = self.length(offset);
        let height = self.terrain.height.as_slice();
        let grade = (height[to] - height[from]).abs() / length;
        if grade > self.params.max_grade.of(class) {
            return None;
        }
        let g = grade / self.params.comfort_grade;
        let streams = between
            .iter()
            .flatten()
            .chain([&to])
            .filter(|&&c| self.is_stream(c))
            .count();
        let forest = self.params.forest_cost * self.terrain.vegetation.as_slice()[to];
        Some(
            length * (1.0 + g * g * 6.0)
                + forest * length
                + self.params.ford_cost * to_f64(streams),
        )
    }

    /// Whether `cell` is lake or big river, which only bridges cross.
    #[must_use]
    pub fn is_water(&self, cell: usize) -> bool {
        self.terrain.lake.as_slice()[cell] || self.terrain.big_river.as_slice()[cell]
    }

    /// Whether `cell` is a stream: a river below the big-river threshold, which roads ford.
    #[must_use]
    pub fn is_stream(&self, cell: usize) -> bool {
        self.terrain.river.as_slice()[cell] && !self.is_water(cell)
    }

    /// Whether the move from `from` to `to` is a bridge: longer than one of the 16 steps, or
    /// one of them across water.
    #[must_use]
    pub fn is_bridge(&self, from: usize, to: usize) -> bool {
        self.offset(from, to).is_none_or(|offset| {
            self.between(from, offset)
                .iter()
                .flatten()
                .any(|&c| self.is_water(c))
        })
    }

    /// The cells a step passes after `from`: those between its ends, then `to`; empty when `to`
    /// is not one of the 16 neighbours of `from`.
    pub(super) fn entered(&self, from: usize, to: usize) -> Vec<usize> {
        self.offset(from, to)
            .map(|offset| {
                let between = self.between(from, offset);
                between.into_iter().flatten().chain([to]).collect()
            })
            .unwrap_or_default()
    }

    /// The grid of the terrain's shape.
    pub(super) fn grid(&self) -> &Grid<f64> {
        self.terrain.height
    }

    /// The offset `(rows, cols)` from `from` to `to` when it is one of the 16 directions.
    pub(super) fn offset(&self, from: usize, to: usize) -> Option<(isize, isize)> {
        let (dr, dc) = self.delta(from, to)?;
        D16.contains(&(dr, dc)).then_some((dr, dc))
    }

    /// The offset `(rows, cols)` from cell `from` to cell `to`; `None` beyond `isize`.
    pub(super) fn delta(&self, from: usize, to: usize) -> Option<(isize, isize)> {
        let cols = self.grid().cols();
        let signed = |i: usize| isize::try_from(i).ok();
        let dr = signed(to / cols)? - signed(from / cols)?;
        let dc = signed(to % cols)? - signed(from % cols)?;
        Some((dr, dc))
    }

    /// The cells the segment of the step `offset` from `from` passes strictly between its
    /// ends: none for a step to a side, the two side cells whose corner a diagonal passes, the
    /// two cells a knight move crosses (the one beside `from` first).
    pub(super) fn between(&self, from: usize, offset: (isize, isize)) -> [Option<usize>; 2] {
        between(self.grid(), from, offset)
    }

    /// The length of a step `offset`, m.
    pub(super) fn length(&self, offset: (isize, isize)) -> f64 {
        let (dr, dc) = (offset.0.unsigned_abs(), offset.1.unsigned_abs());
        self.cell_m * libm::sqrt(to_f64(dr * dr + dc * dc))
    }
}

/// The cells the segment of the step `offset` from `from` passes strictly between its ends on
/// `grid`: none for a step to a side, the two side cells whose corner a diagonal passes, the two
/// cells a knight move crosses (the one beside `from` first).
pub(crate) fn between<T: Copy>(
    grid: &Grid<T>,
    from: usize,
    offset: (isize, isize),
) -> [Option<usize>; 2] {
    let (dr, dc) = offset;
    let at = |d: (isize, isize)| grid.neighbour(from, d);
    match (dr.abs(), dc.abs()) {
        (1, 1) => [at((dr, 0)), at((0, dc))],
        (1, 2) => [at((0, dc / 2)), at((dr, dc / 2))],
        (2, 1) => [at((dr / 2, 0)), at((dr / 2, dc))],
        _ => [None, None],
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(in super::super) const N: usize = 16;

    /// Owned layers of an `N` x `N` terrain for the unit tests.
    pub(in super::super) struct Layers {
        pub height: Grid<f64>,
        pub vegetation: Grid<f64>,
        pub river: Grid<bool>,
        pub big_river: Grid<bool>,
        pub lake: Grid<bool>,
        pub flow_dir: Grid<u8>,
    }

    impl Layers {
        /// Flat ground at 100 m, bare and dry.
        pub fn flat(n: usize) -> Self {
            Self {
                height: Grid::filled(n, n, 100.0),
                vegetation: Grid::filled(n, n, 0.0),
                river: Grid::filled(n, n, false),
                big_river: Grid::filled(n, n, false),
                lake: Grid::filled(n, n, false),
                flow_dir: Grid::filled(n, n, 0),
            }
        }

        /// Sets the mask `name` at `cell`.
        pub fn wet(mut self, name: &str, cell: usize) -> Self {
            let mask = match name {
                "lake" => &mut self.lake,
                "big_river" => &mut self.big_river,
                _ => &mut self.river,
            };
            mask.as_mut_slice()[cell] = true;
            if name == "big_river" {
                self.river.as_mut_slice()[cell] = true;
            }
            self
        }

        pub fn terrain(&self) -> Terrain<'_> {
            Terrain {
                height: &self.height,
                vegetation: &self.vegetation,
                river: &self.river,
                big_river: &self.big_river,
                lake: &self.lake,
                flow_dir: &self.flow_dir,
            }
        }
    }

    pub(in super::super) fn at(r: usize, c: usize) -> usize {
        r * N + c
    }

    #[test]
    fn knight_steps_check_every_cell_they_cross() {
        let params = RoadsParams::default();
        let check = |layers: &Layers, from: usize, to: usize| {
            let cost = StepCost::new(layers.terrain(), &params, 100.0);
            [
                cost.step(from, to, RoadClass::Trail),
                cost.step(to, from, RoadClass::Trail),
            ]
        };
        let dry = Layers::flat(N);
        // A knight move east-south-east, a knight move south-south-east and a diagonal, with
        // the cells their segments cross.
        let moves = [
            (at(5, 5), at(6, 7), [at(5, 6), at(6, 6)], 5.0),
            (at(5, 5), at(7, 6), [at(6, 5), at(6, 6)], 5.0),
            (at(5, 5), at(6, 6), [at(5, 6), at(6, 5)], 2.0),
        ];
        for (from, to, crossed, squared) in moves {
            let length = 100.0 * libm::sqrt(squared);
            for cost in check(&dry, from, to) {
                assert!((cost.unwrap() - length).abs() < 1e-9);
            }
            for cell in crossed.into_iter().chain([from, to]) {
                for water in ["lake", "big_river"] {
                    let wet = Layers::flat(N).wet(water, cell);
                    assert_eq!(check(&wet, from, to), [None, None], "{water} at {cell}");
                }
            }
            // A stream there is forded, not blocked; water beside the segment is passed.
            let stream = Layers::flat(N).wet("river", crossed[0]);
            assert!(check(&stream, from, to)[0].unwrap() > length + 249.0);
            let beside = Layers::flat(N).wet("lake", at(4, 5));
            assert_eq!(check(&beside, from, to), check(&dry, from, to));
        }
    }

    #[test]
    fn a_step_costs_its_length_grade_forest_and_fords() {
        let params = RoadsParams::default();
        let mut layers = Layers::flat(N).wet("river", at(5, 6));
        layers.height.as_mut_slice()[at(5, 6)] = 104.0;
        layers.vegetation.as_mut_slice()[at(5, 6)] = 0.5;
        let cost = StepCost::new(layers.terrain(), &params, 100.0);
        // Grade 0.04 is half the comfort grade: 100·(1 + 0.25·6) = 250; forest 0.8·0.5·100 =
        // 40; one stream cell 250.
        let got = cost.step(at(5, 5), at(5, 6), RoadClass::Main).unwrap();
        assert!((got - 540.0).abs() < 1e-9, "{got}");
        // Leaving the stream costs no ford: only the cell stepped onto pays its vegetation.
        let back = cost.step(at(5, 6), at(5, 7), RoadClass::Main).unwrap();
        assert!((back - 250.0).abs() < 1e-9, "{back}");
        assert!(cost.is_stream(at(5, 6)) && !cost.is_water(at(5, 6)));
    }

    #[test]
    fn steps_steeper_than_their_class_are_forbidden() {
        let params = RoadsParams::default();
        let mut layers = Layers::flat(N);
        layers.height.as_mut_slice()[at(5, 6)] = 109.0;
        layers.height.as_mut_slice()[at(5, 4)] = 108.0;
        let cost = StepCost::new(layers.terrain(), &params, 100.0);
        assert_eq!(cost.step(at(5, 5), at(5, 6), RoadClass::Main), None);
        assert!(cost.step(at(5, 5), at(5, 6), RoadClass::Road).is_some());
        // Exactly the class's grade is allowed.
        assert!(cost.step(at(5, 5), at(5, 4), RoadClass::Main).is_some());
        let steep = RoadsParams {
            max_grade: crate::stages::terrain::roads::MaxGrade {
                trail: 0.05,
                ..Default::default()
            },
            ..Default::default()
        };
        let cost = StepCost::new(layers.terrain(), &steep, 100.0);
        assert_eq!(cost.step(at(5, 5), at(5, 4), RoadClass::Trail), None);
    }

    #[test]
    fn only_the_sixteen_directions_are_steps() {
        let params = RoadsParams::default();
        let layers = Layers::flat(N);
        let cost = StepCost::new(layers.terrain(), &params, 100.0);
        for to in [at(5, 5), at(5, 8), at(7, 7), at(6, 0)] {
            assert_eq!(cost.step(at(5, 5), to, RoadClass::Trail), None, "{to}");
            assert_eq!(cost.entered(at(5, 5), to).len(), 0);
        }
        // Across the grid's east edge is no step either.
        assert_eq!(cost.step(at(5, 15), at(6, 0), RoadClass::Trail), None);
        assert_eq!(
            cost.entered(at(5, 5), at(6, 7)),
            [at(5, 6), at(6, 6), at(6, 7)]
        );
        assert!(cost.is_bridge(at(5, 5), at(5, 9)));
        assert!(!cost.is_bridge(at(5, 5), at(6, 7)));
    }
}
