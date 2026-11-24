//! Bridge edges: single spans across lake and big-river cells (spec §5.9 "Roads").

use super::StepCost;
use crate::grid::{D8, D16};
use crate::kernels::num::{max, min, to_f64};

/// A span found from a bank cell along one direction.
struct Span {
    /// The bank cell on the far side.
    to: usize,
    /// The first water cell the span crosses, for the flow direction.
    first_water: usize,
    /// The length between the bank cells, m.
    length_m: f64,
}

impl StepCost<'_> {
    /// The bridge edges out of the land cell `from` as `(far bank cell, cost)`, at most one per
    /// direction: along each of the 16 directions, the straight line across lake or big-river
    /// cells to its first land cell within `max_bridge_span_m`. Empty for a water cell. See the
    /// [module docs](super) for the cost.
    #[must_use]
    pub fn bridges(&self, from: usize) -> Vec<(usize, f64)> {
        if self.is_water(from) {
            return Vec::new();
        }
        D16.iter()
            .filter_map(|&d| {
                self.span(from, d)
                    .map(|s| (s.to, self.bridge_cost(from, &s, d)))
            })
            .collect()
    }

    /// The cells strictly between the ends of the move from `from` to `to` along its straight
    /// line: the cells a bridge passes over. Empty when `to` lies on none of the 16 directions
    /// from `from`.
    #[must_use]
    pub fn under(&self, from: usize, to: usize) -> Vec<usize> {
        let mut cells = Vec::new();
        let Some(d) = self.direction(from, to) else {
            return cells;
        };
        let mut at = from;
        while at != to {
            cells.extend(self.between(at, d).into_iter().flatten());
            let Some(next) = self.grid().neighbour(at, d) else {
                break;
            };
            if next != to {
                cells.push(next);
            }
            at = next;
        }
        cells
    }

    /// The deck's height above the base level for a bridge between the bank cells `a` and `b`:
    /// the higher bank plus `clearance_m`.
    #[must_use]
    pub fn deck_m(&self, a: usize, b: usize) -> f64 {
        let height = self.terrain.height.as_slice();
        max(height[a], height[b]) + self.params.clearance_m
    }

    /// The span from `from` along the direction `d`, when its line crosses water before it
    /// reaches land within the longest span.
    fn span(&self, from: usize, d: (isize, isize)) -> Option<Span> {
        let unit = self.length(d);
        let mut at = from;
        let mut first_water = None;
        let mut steps = 0_usize;
        loop {
            steps += 1;
            let length_m = to_f64(steps) * unit;
            if length_m > self.params.max_bridge_span_m {
                return None;
            }
            let next = self.grid().neighbour(at, d)?;
            let crossed = self.between(at, d).into_iter().flatten().chain([next]);
            for cell in crossed {
                if first_water.is_none() && self.is_water(cell) {
                    first_water = Some(cell);
                }
            }
            if !self.is_water(next) {
                return first_water.map(|first_water| Span {
                    to: next,
                    first_water,
                    length_m,
                });
            }
            at = next;
        }
    }

    /// The one of the 16 directions whose multiple leads from `from` to `to`.
    fn direction(&self, from: usize, to: usize) -> Option<(isize, isize)> {
        let (dr, dc) = self.delta(from, to)?;
        D16.iter().copied().find(|&(r, c)| {
            let k = if r == 0 { dc / c } else { dr / r };
            k > 0 && (r * k, c * k) == (dr, dc)
        })
    }

    /// The cost of the span `s` from `from` along `d`.
    fn bridge_cost(&self, from: usize, s: &Span, d: (isize, isize)) -> f64 {
        let p = self.params;
        let height = self.terrain.height.as_slice();
        let lower = min(height[from], height[s.to]);
        let rise = self.deck_m(from, s.to) - lower;
        let skew = self.flow_cos(s.first_water, d);
        s.length_m * p.bridge_cost * (1.0 + p.angle_penalty * skew) + p.deck_cost * rise
    }

    /// `|cos θ|` between the direction `d` and the flow direction of `water`; 0 (θ = 90°) on a
    /// lake and where the cell has no flow direction.
    fn flow_cos(&self, water: usize, d: (isize, isize)) -> f64 {
        let code = self.terrain.flow_dir.as_slice()[water];
        if self.terrain.lake.as_slice()[water] || !(1..=8).contains(&code) {
            return 0.0;
        }
        let (fr, fc) = D8[usize::from(code - 1)];
        let dot = d.0 * fr + d.1 * fc;
        let norm = |(r, c): (isize, isize)| {
            let (r, c) = (r.unsigned_abs(), c.unsigned_abs());
            libm::sqrt(to_f64(r * r + c * c))
        };
        to_f64(dot.unsigned_abs()) / (norm(d) * norm((fr, fc)))
    }
}

#[cfg(test)]
mod tests {
    use super::super::cost::tests::{Layers, N, at};
    use super::super::{RoadsParams, StepCost};

    /// `Layers` with `water` in the columns `cols`, flowing south.
    fn river(water: &str, cols: std::ops::RangeInclusive<usize>) -> Layers {
        let mut layers = Layers::flat(N);
        for r in 0..N {
            for c in cols.clone() {
                layers = layers.wet(water, at(r, c));
                layers.flow_dir.as_mut_slice()[at(r, c)] = 5;
            }
        }
        layers
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn a_bridge_reaches_the_first_land_cell_within_the_span() {
        let params = RoadsParams::default();
        let layers = river("big_river", 6..=8);
        let cost = StepCost::new(layers.terrain(), &params, &layers.frame());
        // Only the perpendicular span east is within 400 m; the diagonals and knight lines
        // across three cells are longer. Cost: 400·25 + 12.5·3 (the deck's clearance).
        let found = cost.bridges(at(5, 5));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, at(5, 9));
        assert!(close(found[0].1, 10_037.5), "{}", found[0].1);
        assert!(cost.bridges(at(5, 6)).is_empty(), "no bridge from water");
        assert!(cost.bridges(at(5, 3)).is_empty(), "no bridge from inland");
        let short = RoadsParams {
            max_bridge_span_m: 399.0,
            ..Default::default()
        };
        let cost = StepCost::new(layers.terrain(), &short, &layers.frame());
        assert_eq!(cost.bridges(at(5, 5)).len(), 0);
    }

    #[test]
    fn skew_crossings_pay_the_angle_penalty_and_lakes_do_not() {
        let params = RoadsParams::default();
        let sqrt5 = libm::sqrt(5.0);
        let sqrt2 = libm::sqrt(2.0);
        for (water, skewed) in [("big_river", true), ("lake", false)] {
            let layers = river(water, 6..=6);
            let cost = StepCost::new(layers.terrain(), &params, &layers.frame());
            let found = cost.bridges(at(5, 5));
            let to = |cell: usize| found.iter().find(|(c, _)| *c == cell).map(|&(_, v)| v);
            let pay = |span: f64, cos: f64| {
                let cos = if skewed { cos } else { 0.0 };
                span * 25.0 * (1.0 + cos) + 12.5 * 3.0
            };
            // East, perpendicular to the flow south.
            assert!(close(to(at(5, 7)).unwrap(), pay(200.0, 0.0)));
            // Knight east-south-east: one step over the two water cells it crosses.
            assert!(close(
                to(at(6, 7)).unwrap(),
                pay(100.0 * sqrt5, 1.0 / sqrt5)
            ));
            assert!(close(
                to(at(4, 7)).unwrap(),
                pay(100.0 * sqrt5, 1.0 / sqrt5)
            ));
            // The diagonal passes (6, 6) on its way to (7, 7).
            assert!(close(
                to(at(7, 7)).unwrap(),
                pay(200.0 * sqrt2, 1.0 / sqrt2)
            ));
            // Knight south-south-east lands on water at (7, 6); the next land is beyond 400 m.
            assert_eq!(to(at(9, 7)), None);
            assert_eq!(found.len(), 5, "{found:?}");
            assert!(found.iter().all(|&(c, _)| c % N > 5), "{found:?}");
            // A bridge passes over the cells between its banks.
            assert_eq!(cost.under(at(5, 5), at(5, 7)), [at(5, 6)]);
            assert_eq!(
                cost.under(at(5, 5), at(7, 7)),
                [at(6, 5), at(5, 6), at(6, 6), at(7, 6), at(6, 7)]
            );
            assert_eq!(cost.under(at(5, 5), at(6, 9)).len(), 0);
        }
    }

    #[test]
    fn the_deck_lies_above_the_higher_bank() {
        let params = RoadsParams::default();
        let mut layers = river("lake", 6..=6);
        layers.height.as_mut_slice()[at(5, 7)] = 110.0;
        let cost = StepCost::new(layers.terrain(), &params, &layers.frame());
        assert!(close(cost.deck_m(at(5, 5), at(5, 7)), 113.0));
        let found = cost.bridges(at(5, 5));
        let east = found.iter().find(|(c, _)| *c == at(5, 7)).unwrap().1;
        // The deck rises 13 m above the lower bank.
        assert!(close(east, 200.0 * 25.0 + 12.5 * 13.0), "{east}");
    }
}
