//! The routed depressions as sediment traps: filling them in height order up to their spill
//! level, with an ε slope toward the spill cell (spec §5.9 "Depression filling", after goSPL's
//! documentation, combined with the MST resolver as this spec's own step).
//!
//! A depression is one lake, the pits nested in it included (`resolve_sinks`). Its **flood
//! order** is its cells sorted by (routing level, cell index), the order of the ε slope
//! `resolve_sinks`'s priority floods lifted them along. The spill cell (`SpillPoint.cell`)
//! holds place 0 whether or not it lies in the depression; the other cells follow from place 1.
//! Filled to level `ℓ`, a cell at place `k` stands at `ℓ + k·ε`, so filled cells slope down
//! toward the spill cell and are no pits next step.
//!
//! The fill finds `ℓ` by pouring the volume into the cells in height order (each height less its
//! `k·ε`, ties by cell index), up to the spill level: `ℓ = min(spill level, the level the volume
//! reaches)`. What does not fit passes on.
//!
//! **The decision of each Gauss–Seidel iteration** lives here too. A depression that the routed
//! sediment reaches with the volume `I` holds `min(max(I, 0), capacity)` of it and lets go the
//! rest, never a negative volume ([`Traps::release`]); the volume it holds fills its cells in the
//! start of the iteration's sweep ([`Traps::start`]). From the iteration after half the limit a
//! depression whose last start was full stays full ([`Traps::decide`]), so it changes between
//! full and not full a bounded number of times in a step.

use crate::kernels::num::{OrdF64, key, max, min, to_f64};
use crate::stages::evolution::flow::cell_index;
// `EPSILON` is the rise of the ε slope per place in a depression's flood order: the lift of
// one flood step of `resolve_sinks`.
use crate::stages::evolution::resolve_sinks::{Depressions, EPSILON};

/// One routed depression as a sediment trap.
#[derive(Debug)]
struct Trap {
    /// The spill level, the pass height (`SpillPoint.level`).
    level: f64,
    /// The depression's cells in flood order, each with its ε slope `k·ε` above the fill level.
    places: Vec<(usize, f64)>,
    /// The volume that fills every cell to its place at the spill level, m³.
    capacity: f64,
}

/// The routed depressions of one step as sediment traps, with their capacities on the heights
/// they were built from.
#[derive(Debug, Default)]
pub(super) struct Traps {
    /// Per cell, the depression holding it (from 1), 0 outside every depression.
    label: Vec<u32>,
    /// Per cell, the depression (from 1) whose spill cell it is, 0 for none.
    spill_of: Vec<u32>,
    /// Each depression as a trap, at `n − 1` for depression `n`.
    depressions: Vec<Trap>,
    /// The area of one cell, m².
    cell_area: f64,
}

impl Traps {
    /// The depressions of `depressions` as traps over `height`, flood order by `routing`. The
    /// depressions must fit the grid: labels name existing depressions, spill cells lie on the
    /// grid and differ.
    pub(super) fn new(
        depressions: &Depressions,
        routing: &[f64],
        height: &[f64],
        cell_area: f64,
    ) -> Self {
        let label = depressions.label.as_slice().to_vec();
        let mut members: Vec<Vec<usize>> = vec![Vec::new(); depressions.spill.len()];
        for (cell, &n) in label.iter().enumerate() {
            if let Some(list) = cell_index(n)
                .checked_sub(1)
                .and_then(|k| members.get_mut(k))
            {
                list.push(cell);
            }
        }
        let mut spill_of = vec![0; label.len()];
        let mut traps = Vec::with_capacity(members.len());
        for (n, (mut cells, spill)) in members.into_iter().zip(&depressions.spill).enumerate() {
            let at = cell_index(spill.cell);
            spill_of[at] = u32::try_from(n + 1).unwrap_or(u32::MAX);
            cells.sort_by_key(|&c| (OrdF64(key(routing[c])), c));
            let places: Vec<(usize, f64)> = cells
                .iter()
                .filter(|&&c| c == at)
                .map(|&c| (c, 0.0))
                .chain(
                    cells
                        .iter()
                        .filter(|&&c| c != at)
                        .enumerate()
                        .map(|(k, &c)| (c, to_f64(k + 1) * EPSILON)),
                )
                .collect();
            let capacity = places.iter().fold(0.0, |sum, &(c, lift)| {
                let rise = spill.level + lift - height[c];
                if rise > 0.0 {
                    sum + rise * cell_area
                } else {
                    sum
                }
            });
            traps.push(Trap {
                level: spill.level,
                places,
                capacity,
            });
        }
        Self {
            label,
            spill_of,
            depressions: traps,
            cell_area,
        }
    }

    /// The number of depressions.
    pub(super) fn len(&self) -> usize {
        self.depressions.len()
    }

    /// The depression (from 0) holding `cell`, if one does.
    pub(super) fn holding(&self, cell: usize) -> Option<usize> {
        let n = cell_index(*self.label.get(cell)?);
        n.checked_sub(1)
    }

    /// The depression (from 0) whose spill cell `cell` is, if one.
    pub(super) fn spilling_at(&self, cell: usize) -> Option<usize> {
        let n = cell_index(*self.spill_of.get(cell)?);
        n.checked_sub(1)
    }

    /// Whether `cell` lies in a depression.
    pub(super) fn holds(&self, cell: usize) -> bool {
        self.holding(cell).is_some()
    }

    /// The volume (m³) that fills depression `n` to its spill level on the heights it was built
    /// from; 0 for a depression that does not exist.
    pub(super) fn capacity(&self, n: usize) -> f64 {
        self.depressions.get(n).map_or(0.0, |t| t.capacity)
    }

    /// The part of `volume` (m³) that depression `n` keeps on the heights it was built from: all
    /// of it up to its capacity, nothing of a volume that is not positive.
    pub(super) fn kept(&self, n: usize, volume: f64) -> f64 {
        if volume > 0.0 {
            min(volume, self.capacity(n))
        } else {
            0.0
        }
    }

    /// What depression `n` does with the volume `reached` (m³) that the routing of one
    /// Gauss–Seidel iteration brings it: `(kept, released)`. It keeps [`Traps::kept`] and lets
    /// go the excess over its capacity, never a negative volume. Early iterations can route a
    /// negative inflow (the cells upstream deposited more than the last iterate routed); at a
    /// fixed point it is not negative, so holding it back changes only the path to it.
    pub(super) fn release(&self, n: usize, reached: f64) -> (f64, f64) {
        let kept = self.kept(n, reached);
        (kept, max(reached - kept, 0.0))
    }

    /// The volume (m³) depression `n` holds in the next start, from the volume `target` its
    /// routing keeps and the volume `previous` the last start held: the target, except that
    /// `past_half` the iteration limit a depression whose last start was full stays full. After
    /// half the limit each depression thus changes from not full to full at most once and never
    /// back, whatever the other cells do.
    pub(super) fn decide(&self, n: usize, target: f64, previous: f64, past_half: bool) -> f64 {
        let capacity = self.capacity(n);
        if past_half && previous >= capacity {
            capacity
        } else {
            target
        }
    }

    /// Writes into `start` the heights `height` with each depression `n` filled by the volume
    /// `kept[n]` (m³), as [`Traps::fill`] fills it: the start of an iteration's sweep.
    pub(super) fn start(&self, kept: &[f64], height: &[f64], start: &mut [f64]) {
        start.copy_from_slice(height);
        for (n, &volume) in kept.iter().enumerate() {
            self.fill(n, volume, start);
        }
    }

    /// Pours `volume` (m³) into depression `n` over `height` and returns the volume it took:
    /// every cell below the fill level rises to its place on the ε slope. Nothing is filled
    /// from a volume that is not positive.
    pub(super) fn fill(&self, n: usize, volume: f64, height: &mut [f64]) -> f64 {
        let Some(trap) = self.depressions.get(n).filter(|_| volume > 0.0) else {
            return 0.0;
        };
        let level = if volume >= trap.capacity {
            trap.level
        } else {
            min(
                fill_level(trap, volume / self.cell_area, height),
                trap.level,
            )
        };
        let mut taken = 0.0;
        for &(c, lift) in &trap.places {
            let target = level + lift;
            if target > height[c] {
                taken += (target - height[c]) * self.cell_area;
                height[c] = target;
            }
        }
        taken
    }
}

/// The level `ℓ` at which `depth` metres of one cell's area fill `trap` over `height`: the cells
/// in height order (less their ε slope, ties by cell index) take water until the level reaches
/// the next one.
fn fill_level(trap: &Trap, depth: f64, height: &[f64]) -> f64 {
    let mut floors: Vec<(f64, usize)> = trap
        .places
        .iter()
        .map(|&(c, lift)| (height[c] - lift, c))
        .collect();
    floors.sort_by_key(|&(floor, c)| (OrdF64(key(floor)), c));
    let mut sum = 0.0;
    for (m, &(floor, _)) in floors.iter().enumerate() {
        sum += floor;
        let level = (depth + sum) / to_f64(m + 1);
        if floors.get(m + 1).is_none_or(|&(next, _)| level <= next) {
            return level;
        }
    }
    trap.level
}

#[cfg(test)]
mod tests {
    use super::{EPSILON, Traps};
    use crate::grid::Grid;
    use crate::stages::evolution::resolve_sinks::Depressions;
    use crate::stages::evolution::resolve_sinks::SpillPoint;

    /// A 1 x 6 strip of 1 m² cells: cells 1..=4 form depression 1, spilling from cell 1 (in the
    /// depression) to cell 0 at level 10. The flood order by routing is 1, 2, 4, 3.
    fn strip() -> (Depressions, Vec<f64>, Vec<f64>) {
        let depressions = Depressions {
            label: Grid::from_vec(1, 6, vec![0, 1, 1, 1, 1, 0]).unwrap(),
            spill: vec![SpillPoint {
                cell: 1,
                to: 0,
                level: 10.0,
            }],
        };
        let routing = vec![
            10.0,
            10.0,
            10.0 + EPSILON,
            10.0 + 3.0 * EPSILON,
            10.0 + 2.0 * EPSILON,
            20.0,
        ];
        let height = vec![10.0, 8.0, 4.0, 6.0, 9.0, 20.0];
        (depressions, routing, height)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9
    }

    #[test]
    fn places_follow_the_flood_order_from_the_spill_cell() {
        let (dep, routing, height) = strip();
        let traps = Traps::new(&dep, &routing, &height, 1.0);
        assert_eq!(traps.len(), 1);
        // Places: cell 1 → 0, cell 2 → ε, cell 4 → 2ε, cell 3 → 3ε.
        let want = [
            (1, 0.0),
            (2, EPSILON),
            (4, 2.0 * EPSILON),
            (3, 3.0 * EPSILON),
        ];
        let got: Vec<(usize, f64)> = traps.depressions[0].places.clone();
        assert_eq!(got.len(), want.len());
        for ((c, lift), (wc, wl)) in got.iter().zip(want) {
            assert_eq!(*c, wc);
            assert!(close(*lift, wl), "{c}: {lift} vs {wl}");
        }
        // Capacity: (10 − 8) + (10 + ε − 4) + (10 + 2ε − 9) + (10 + 3ε − 6).
        let capacity = 2.0 + 6.0 + 1.0 + 4.0 + 6.0 * EPSILON;
        assert!(close(traps.depressions[0].capacity, capacity));
        assert_eq!(traps.holding(3), Some(0));
        assert_eq!(traps.holding(0), None);
        assert_eq!(traps.holding(99), None);
        assert_eq!(traps.spilling_at(1), Some(0));
        assert_eq!(traps.spilling_at(2), None);
        assert!(traps.holds(4) && !traps.holds(5));
    }

    #[test]
    fn a_spill_cell_outside_the_depression_keeps_place_zero() {
        let (mut dep, routing, height) = strip();
        dep.label = Grid::from_vec(1, 6, vec![0, 0, 1, 1, 1, 0]).unwrap();
        let traps = Traps::new(&dep, &routing, &height, 1.0);
        let places = &traps.depressions[0].places;
        assert_eq!(places.iter().map(|p| p.0).collect::<Vec<_>>(), [2, 4, 3]);
        assert!(
            close(places[0].1, EPSILON),
            "the first lake cell sits ε above the spill cell"
        );
        assert_eq!(traps.spilling_at(1), Some(0));
    }

    #[test]
    fn a_small_volume_fills_the_lowest_cells_to_one_level() {
        let (dep, routing, height) = strip();
        let traps = Traps::new(&dep, &routing, &height, 1.0);
        let mut h = height.clone();
        // 3 m³: cell 2 (floor 4 − ε) alone takes 2 m to reach cell 3's floor (6 − 3ε); the last
        // metre spreads over both, so the level is 6.5 − 2ε.
        let taken = traps.fill(0, 3.0, &mut h);
        assert!(close(taken, 3.0), "{taken}");
        let level = 6.5 - 2.0 * EPSILON;
        assert!(close(h[2], level + EPSILON), "{}", h[2]);
        assert!(close(h[3], level + 3.0 * EPSILON), "{}", h[3]);
        assert_eq!(
            h[1].to_bits(),
            8.0_f64.to_bits(),
            "cell 1 lies above the level"
        );
        assert_eq!(h[4].to_bits(), 9.0_f64.to_bits());
        assert!(h[3] > h[2], "the ε slope falls toward the spill cell");
    }

    #[test]
    fn a_large_volume_fills_to_the_spill_level_and_leaves_the_rest() {
        let (dep, routing, height) = strip();
        let traps = Traps::new(&dep, &routing, &height, 1.0);
        let capacity = traps.depressions[0].capacity;
        for volume in [capacity, capacity + 5.0] {
            let mut h = height.clone();
            let taken = traps.fill(0, volume, &mut h);
            assert!(close(taken, capacity), "{volume}: took {taken}");
            assert_eq!(
                h[1].to_bits(),
                10.0_f64.to_bits(),
                "the spill cell at the level"
            );
            assert!(close(h[2], 10.0 + EPSILON));
            assert!(close(h[4], 10.0 + 2.0 * EPSILON));
            assert!(close(h[3], 10.0 + 3.0 * EPSILON));
            assert_eq!(h[0].to_bits(), 10.0_f64.to_bits());
            assert_eq!(h[5].to_bits(), 20.0_f64.to_bits());
        }
        assert!(close(traps.kept(0, capacity + 5.0), capacity));
        assert!(close(traps.kept(0, 1.5), 1.5));
    }

    #[test]
    fn nothing_is_filled_from_no_sediment() {
        let (dep, routing, height) = strip();
        let traps = Traps::new(&dep, &routing, &height, 1.0);
        for volume in [0.0, -2.0, f64::NAN] {
            let mut h = height.clone();
            assert_eq!(traps.fill(0, volume, &mut h).to_bits(), 0.0_f64.to_bits());
            assert_eq!(h, height);
            assert_eq!(traps.kept(0, volume).to_bits(), 0.0_f64.to_bits());
        }
        let mut h = height.clone();
        assert_eq!(traps.fill(7, 1.0, &mut h).to_bits(), 0.0_f64.to_bits());
        assert_eq!(traps.kept(7, 1.0).to_bits(), 0.0_f64.to_bits());
        let none = Traps::default();
        assert_eq!(
            (none.len(), none.holding(0), none.spilling_at(0)),
            (0, None, None)
        );
    }

    #[test]
    fn a_depression_lets_go_only_what_it_does_not_keep() {
        let (dep, routing, height) = strip();
        let traps = Traps::new(&dep, &routing, &height, 1.0);
        let capacity = traps.capacity(0);
        assert!(close(capacity, 13.0 + 6.0 * EPSILON));
        // A negative inflow (the cells upstream deposited more than the last iterate routed)
        // keeps nothing and lets go nothing.
        let (kept, release) = traps.release(0, -3.0);
        assert_eq!((kept.to_bits(), release.to_bits()), (0, 0));
        let (kept, release) = traps.release(0, 1.5);
        assert!(
            close(kept, 1.5) && release.to_bits() == 0,
            "{kept} {release}"
        );
        let (kept, release) = traps.release(0, capacity + 5.0);
        assert!(
            close(kept, capacity) && close(release, 5.0),
            "{kept} {release}"
        );
        assert_eq!(traps.capacity(7).to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn a_full_depression_stays_full_past_half_the_limit() {
        let (dep, routing, height) = strip();
        let traps = Traps::new(&dep, &routing, &height, 1.0);
        let capacity = traps.capacity(0);
        let decide = |target: f64, previous: f64, past_half: bool| {
            traps.decide(0, target, previous, past_half).to_bits()
        };
        // Up to half the limit the start holds the target, whatever the last start held.
        assert_eq!(decide(2.0, capacity, false), 2.0_f64.to_bits());
        assert_eq!(decide(capacity, 0.0, false), capacity.to_bits());
        // Past it a depression whose last start was full stays full when its target drops.
        assert_eq!(decide(2.0, capacity, true), capacity.to_bits());
        // One that was not full follows its target, and may become full once.
        assert_eq!(decide(2.0, 1.0, true), 2.0_f64.to_bits());
        assert_eq!(decide(capacity, 2.0, true), capacity.to_bits());
    }

    #[test]
    fn a_start_is_the_heights_with_each_depression_filled() {
        let (dep, routing, height) = strip();
        let traps = Traps::new(&dep, &routing, &height, 1.0);
        let mut want = height.clone();
        traps.fill(0, 3.0, &mut want);
        // Whatever the buffer held, the start is `height` with the depression filled by 3 m³.
        let mut start = vec![99.0; 6];
        traps.start(&[3.0], &height, &mut start);
        assert_eq!(start, want);
        traps.start(&[0.0], &height, &mut start);
        assert_eq!(start, height);
    }

    #[test]
    fn cells_with_area_scale_the_volume() {
        let (dep, routing, height) = strip();
        let traps = Traps::new(&dep, &routing, &height, 100.0);
        let mut h = height.clone();
        // 150 m³ over 100 m² cells: 1.5 m on cell 2 alone, below cell 3's floor (6 − 3ε).
        let taken = traps.fill(0, 150.0, &mut h);
        assert!(close(taken, 150.0), "{taken}");
        assert!(close(h[2], 5.5), "{}", h[2]);
        assert_eq!(h[3].to_bits(), 6.0_f64.to_bits());
    }
}
