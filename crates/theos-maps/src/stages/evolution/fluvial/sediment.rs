//! The sediment of one step routed over the flow weights (spec §5.9): each cell passes on what
//! reaches it plus its own sediment, split like its water. A cell's own sediment is its net
//! erosion and the slope supply it takes in ([`step_sediment`]). Everything that reaches a routed
//! depression before the walk passes its spill cell goes into the depression's inflow; there the
//! depression decides what it lets go.
//!
//! **The books** ([`books`]) close each step: after the Gauss–Seidel iterations they route the
//! step's sediment once more over the final heights, fill each depression with what reaches it
//! up to its capacity, and give its cells that fill. The volume balance then closes exactly,
//! converged or not.
//!
//! **The supply laid down ahead** ([`lay_down`]) gives the iterations their first iterate: one
//! walk of the step's slope supply alone, in which every cell keeps its share of what passes
//! through it, as the law lays it down where nothing incises.

use super::fill::Traps;
use crate::kernels::num::max;
use crate::stages::evolution::flow::{FlowWeights, cell_index};

/// Writes each cell's own sediment of the step into `volume`, m³: its net erosion
/// `(start − height)·cell_area` (negative where it gained), plus `supply`, the volume of slope
/// supply it takes in this step (spec §5.9 "Slope supply"). The supply enters the sediment flux
/// at its cell like eroded sediment: `slope_limit` removed it from that cell since `fluvial` last
/// ran. On a routed depression's cell `start` is its filled height, so its own sediment is its
/// incision since the fill: the fill itself is never sediment.
pub(super) fn step_sediment(
    volume: &mut [f64],
    start: &[f64],
    height: &[f64],
    supply: &[f64],
    cell_area: f64,
) {
    let own = start.iter().zip(height).zip(supply);
    for (v, ((before, after), added)) in volume.iter_mut().zip(own) {
        *v = (before - after) * cell_area + added;
    }
}

/// Routes `volume` (each cell's own sediment this step, m³; negative where the cell gained) over
/// `weights` in reverse `stack` order, donors first, and returns the volume that leaves at the
/// cells without an edge: the drained-edge outlets, or the closed map's sink.
///
/// `arriving[i]` becomes the volume that reaches cell `i` from upstream, its own excluded (the
/// cells of Yuan et al.'s `ups(i)`), plus what a depression lets go at it.
///
/// While depression `n` is open, every share that would reach one of its cells goes into the
/// depression's inflow instead, and so does each of its cells' own volume (its incision since its
/// fill and its slope supply), wherever that cell's edges lead: to the spill cell outside the
/// depression too, or, at a spill cell inside it, before the depression lets go. When the walk
/// reaches the depression's spill cell, `close(n, inflow)` returns the volume the depression lets
/// go. It counts as arriving at the spill cell, so it leaves with the spill cell's own flux over
/// its flow edges (with steepest routing, to `SpillPoint.to`), as spec §5.9's "passes on from the
/// spill node" says. A spill cell outside its depression (the higher cell of the pass) thus sees
/// the release in its deposition term; one inside it is a lake cell, whose deposition term sees
/// nothing, and it passes the release on. From then on the depression is closed: what reaches it
/// later flows through its cells like over any other cell. Every volume either leaves the map,
/// stays in a depression, or is in `arriving`, so the walk loses nothing.
pub(super) fn route(
    stack: &[u32],
    weights: &FlowWeights,
    traps: &Traps,
    volume: &[f64],
    arriving: &mut [f64],
    close: impl FnMut(usize, f64) -> f64,
) -> f64 {
    walk(stack, weights, traps, volume, arriving, close, |_, _| 0.0)
}

/// Lays the step's slope `supply` (m³ per cell) down along its way, onto `height`: the walk of
/// [`route`] over the supply alone, in which every cell `i` with an edge keeps the volume
/// `keep(i, through)` of what passes through it (what reaches it plus its own supply) and passes
/// the rest on. A routed depression keeps what reaches it up to its capacity and lets go only the
/// excess, at its spill cell (`Traps::release`); its cells are not raised here. A cell that keeps
/// a volume rises by it over `cell_area`; every other height keeps its bits.
pub(super) fn lay_down(
    stack: &[u32],
    weights: &FlowWeights,
    traps: &Traps,
    supply: &[f64],
    cell_area: f64,
    height: &mut [f64],
    keep: impl Fn(usize, f64) -> f64,
) {
    let mut arriving = vec![0.0; supply.len()];
    walk(
        stack,
        weights,
        traps,
        supply,
        &mut arriving,
        |n, reached| traps.release(n, reached).1,
        |i, through| {
            let kept = keep(i, through);
            if kept != 0.0 {
                height[i] += kept / cell_area;
            }
            kept
        },
    );
}

/// The walk of [`route`] and [`lay_down`]: [`route`]'s walk, in which `keep(i, through)` returns
/// the part of what passes through cell `i` (a cell with an edge: what reaches it plus its own
/// volume) that stays on it; the rest leaves over its edges. Returns the volume that leaves at the
/// cells without an edge.
fn walk(
    stack: &[u32],
    weights: &FlowWeights,
    traps: &Traps,
    volume: &[f64],
    arriving: &mut [f64],
    mut close: impl FnMut(usize, f64) -> f64,
    mut keep: impl FnMut(usize, f64) -> f64,
) -> f64 {
    arriving.fill(0.0);
    let mut inflow = vec![0.0; traps.len()];
    let mut open = vec![true; traps.len()];
    let mut out = 0.0;
    for &cell in stack.iter().rev() {
        let i = cell_index(cell);
        let mut own = volume[i];
        if let Some(n) = traps.holding(i).filter(|&n| open[n]) {
            inflow[n] += own;
            own = 0.0;
        }
        if let Some(n) = traps.spilling_at(i) {
            open[n] = false;
            arriving[i] += close(n, inflow[n]);
        }
        let through = arriving[i] + own;
        let edges = weights.edges(i);
        if edges.is_empty() {
            out += through;
            continue;
        }
        let leaving = through - keep(i, through);
        for e in edges {
            let to = cell_index(weights.to[e]);
            let share = leaving * weights.weight[e];
            match traps.holding(to) {
                Some(n) if open[n] => inflow[n] += share,
                _ => arriving[to] += share,
            }
        }
    }
    out
}

/// What [`books`] reads besides the routing; every slice holds one entry per cell.
pub(super) struct Ledger<'a> {
    /// The heights entering the step, `h*`, on which the depressions were built.
    pub h_star: &'a [f64],
    /// The start of the sweep that gave the final heights: `h*` with the depressions filled
    /// (`h*` itself without deposition).
    pub start: &'a [f64],
    /// The volume of slope supply each cell takes in this step, m³.
    pub supply: &'a [f64],
    /// The area of one cell, m².
    pub cell_area: f64,
}

/// The result of [`books`].
pub(super) struct Books {
    /// The sediment that left the map at the cells without an edge, m³.
    pub out: f64,
    /// The largest move of a depression's cell by the books, metres: how far the fill of the
    /// final routing lies from the fill the last sweep started from.
    pub correction: f64,
}

/// The books of one step. Routes each cell's own sediment against the start of the final sweep,
/// `(start − height)·cell_area + supply` ([`step_sediment`]), once more. Each depression takes
/// what reaches it up to its capacity onto a copy of `h*`, as a fill, and lets go the rest with
/// its sign. Each depression cell then gets that fill in place of the one its sweep started
/// from, `height += filled − start`, and keeps the incision the sweep gave it.
///
/// The volume balance closes exactly, for any final heights and start: outside the depressions
/// `start = h*`, so a cell's change of volume is its own sediment less its supply; on them the
/// fill adds what the depression took; and every volume either leaves or is taken. So the map's
/// volume changes by the supply taken in less [`Books::out`]. A depression that is not full
/// takes all of a positive inflow and lets go nothing. An inflow that is not positive (possible
/// at the iteration limit, where the cells upstream deposited more than the final routing brings)
/// fills nothing and passes on with its sign, so the balance stays exact.
pub(super) fn books(
    stack: &[u32],
    weights: &FlowWeights,
    traps: &Traps,
    ledger: &Ledger<'_>,
    height: &mut [f64],
) -> Books {
    let mut volume = vec![0.0; height.len()];
    step_sediment(
        &mut volume,
        ledger.start,
        height,
        ledger.supply,
        ledger.cell_area,
    );
    let mut filled = ledger.h_star.to_vec();
    let mut arriving = vec![0.0; height.len()];
    let out = route(
        stack,
        weights,
        traps,
        &volume,
        &mut arriving,
        |n, inflow| inflow - traps.fill(n, inflow, &mut filled),
    );
    let mut correction = 0.0;
    let lakes = height.iter_mut().zip(filled.iter().zip(ledger.start));
    for (i, (h, (&fill, &from))) in lakes.enumerate() {
        if traps.holds(i) {
            let moved = fill - from;
            *h += moved;
            correction = max(correction, moved.abs());
        }
    }
    Books { out, correction }
}

#[cfg(test)]
mod tests {
    use super::{Ledger, books, lay_down, route};
    use crate::grid::Grid;
    use crate::stages::evolution::flow::FlowWeights;
    use crate::stages::evolution::fluvial::fill::{EPSILON, Traps};
    use crate::stages::evolution::resolve_sinks::{Depressions, SpillPoint};

    /// A 1 x 6 strip: cell 0 is the outlet; 1 → 0; 2 → 1; 3 → 2; 4 sends 1/4 to 3 and 3/4 to 2;
    /// 5 → 4. All edges are 1 m long.
    fn strip() -> FlowWeights {
        FlowWeights {
            start: vec![0, 0, 1, 2, 3, 5, 6],
            to: vec![0, 1, 2, 3, 2, 4],
            weight: vec![1.0, 1.0, 1.0, 0.25, 0.75, 1.0],
            dist: vec![1.0; 6],
        }
    }

    const STACK: [u32; 6] = [0, 1, 2, 3, 4, 5];

    /// Depressions on the strip with `label`, each spilling at `(cell, to)`.
    fn traps(label: Vec<u32>, spills: &[(u32, u32)]) -> Traps {
        let depressions = Depressions {
            label: Grid::from_vec(1, 6, label).unwrap(),
            spill: spills
                .iter()
                .map(|&(cell, to)| SpillPoint {
                    cell,
                    to,
                    level: 100.0,
                })
                .collect(),
        };
        Traps::new(&depressions, &[0.0; 6], &[0.0; 6], 1.0)
    }

    #[test]
    fn without_depressions_the_walk_accumulates_over_the_weights() {
        let mut arriving = vec![0.0; 6];
        let volume = [0.0, 1.0, 2.0, 4.0, 8.0, 16.0];
        let out = route(
            &STACK,
            &strip(),
            &Traps::default(),
            &volume,
            &mut arriving,
            |_, v| v,
        );
        // 5 → 4: 16; 4 passes 24: 6 to 3, 18 to 2; 3 passes 10 to 2; 2 passes 30; 1 passes 31.
        assert_eq!(arriving, [31.0, 30.0, 28.0, 6.0, 16.0, 0.0]);
        assert_eq!(out.to_bits(), 31.0_f64.to_bits());
    }

    #[test]
    fn a_depression_keeps_its_inflow_and_lets_go_at_its_spill_cell() {
        // Depression 1 holds cells 2 and 3 and spills from cell 2 to cell 1.
        let traps = traps(vec![0, 0, 1, 1, 0, 0], &[(2, 1)]);
        let mut arriving = vec![0.0; 6];
        let volume = [0.0, 1.0, 0.0, 0.0, 8.0, 16.0];
        let mut calls = Vec::new();
        let out = route(&STACK, &strip(), &traps, &volume, &mut arriving, |n, v| {
            calls.push((n, v));
            v - 20.0
        });
        // Cell 4 passes 24: 6 into cell 3 and 18 into cell 2, both kept.
        assert_eq!(calls, [(0, 24.0)]);
        // Cell 2, the spill cell, receives the 4 it lets go and passes it to cell 1, which adds
        // its own 1.
        assert_eq!(arriving, [5.0, 4.0, 4.0, 0.0, 16.0, 0.0]);
        assert_eq!(out.to_bits(), 5.0_f64.to_bits());
    }

    #[test]
    fn a_spill_cell_outside_its_depression_receives_the_release() {
        // Depression 1 holds cell 3 alone and spills from cell 2, the higher cell of the pass,
        // which lies outside it. What the depression lets go reaches cell 2 like sediment from
        // upstream, so cell 2's deposition term sees it.
        let traps = traps(vec![0, 0, 0, 1, 0, 0], &[(2, 1)]);
        let mut arriving = vec![0.0; 6];
        let volume = [0.0, 0.0, 1.0, 0.0, 8.0, 16.0];
        let mut calls = Vec::new();
        let out = route(&STACK, &strip(), &traps, &volume, &mut arriving, |n, v| {
            calls.push((n, v));
            v - 2.0
        });
        // Cell 4 passes 24: 6 into the depression, 18 to cell 2. The depression keeps 2 and lets
        // go 4 at cell 2, which then passes 18 + 4 + its own 1.
        assert_eq!(calls, [(0, 6.0)]);
        assert_eq!(arriving, [23.0, 23.0, 22.0, 0.0, 16.0, 0.0]);
        assert_eq!(out.to_bits(), 23.0_f64.to_bits());
    }

    #[test]
    fn a_depression_keeps_its_own_cells_sediment() {
        // Depression 1 holds cell 3 alone and spills from cell 2, outside it (the higher cell of
        // the pass). Cell 3's own 5 (slope supply on a lake cell) goes into the depression like
        // cell 4's share of 6, although its edge leads to the spill cell.
        let traps_outside = traps(vec![0, 0, 0, 1, 0, 0], &[(2, 1)]);
        let mut arriving = vec![0.0; 6];
        let volume = [0.0, 0.0, 0.0, 5.0, 8.0, 16.0];
        let mut calls = Vec::new();
        let out = route(
            &STACK,
            &strip(),
            &traps_outside,
            &volume,
            &mut arriving,
            |n, v| {
                calls.push((n, v));
                v - 2.0
            },
        );
        // It keeps 2 of 11 and lets go 9 at cell 2, which passes 18 + 9.
        assert_eq!(calls, [(0, 11.0)]);
        assert_eq!(arriving, [27.0, 27.0, 27.0, 0.0, 16.0, 0.0]);
        assert_eq!(out.to_bits(), 27.0_f64.to_bits());
        // Depression 1 holds cells 2 and 3 and spills from cell 2, inside it: the spill cell's
        // own 4 joins the depression before it lets go.
        let traps_inside = traps(vec![0, 0, 1, 1, 0, 0], &[(2, 1)]);
        let volume = [0.0, 1.0, 4.0, 0.0, 8.0, 16.0];
        let mut calls = Vec::new();
        let out = route(
            &STACK,
            &strip(),
            &traps_inside,
            &volume,
            &mut arriving,
            |n, v| {
                calls.push((n, v));
                v - 20.0
            },
        );
        assert_eq!(calls, [(0, 28.0)]);
        assert_eq!(arriving, [9.0, 8.0, 8.0, 0.0, 16.0, 0.0]);
        assert_eq!(out.to_bits(), 9.0_f64.to_bits());
    }

    #[test]
    fn what_reaches_a_closed_depression_flows_through() {
        // Depression 1 holds cell 3 alone and spills from cell 5, which the walk meets first:
        // cell 4's later share into cell 3 is not kept.
        let traps = traps(vec![0, 0, 0, 1, 0, 0], &[(5, 4)]);
        let mut arriving = vec![0.0; 6];
        let volume = [0.0, 0.0, 0.0, 0.0, 8.0, 16.0];
        let mut calls = Vec::new();
        let out = route(&STACK, &strip(), &traps, &volume, &mut arriving, |n, v| {
            calls.push((n, v));
            v
        });
        assert_eq!(calls, [(0, 0.0)]);
        assert_eq!(arriving, [24.0, 24.0, 24.0, 6.0, 16.0, 0.0]);
        assert_eq!(out.to_bits(), 24.0_f64.to_bits());
    }

    /// The pass strip of 1 m² cells: the outlet 0 at 0 m; cell 1, the pass cell at 50 m and the
    /// spill cell, outside the lake; cell 2, the lake at 45 m (depression 1, spilling from cell 1
    /// to cell 0 at 50 m, so it holds `5 + ε` m³); cell 3, a slope at 100 m. Each drains west.
    fn pass_strip() -> (FlowWeights, Traps, Vec<f64>) {
        let weights = FlowWeights {
            start: vec![0, 0, 1, 2, 3],
            to: vec![0, 1, 2],
            weight: vec![1.0; 3],
            dist: vec![1.0; 3],
        };
        let depressions = Depressions {
            label: Grid::from_vec(1, 4, vec![0, 0, 1, 0]).unwrap(),
            spill: vec![SpillPoint {
                cell: 1,
                to: 0,
                level: 50.0,
            }],
        };
        let h_star = vec![0.0, 50.0, 45.0, 100.0];
        let routing = [0.0, 50.0, 50.0 + EPSILON, 100.0];
        let traps = Traps::new(&depressions, &routing, &h_star, 1.0);
        (weights, traps, h_star)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9
    }

    #[test]
    fn the_books_fill_each_lake_with_what_the_final_routing_brings_it() {
        let (weights, traps, h_star) = pass_strip();
        let stack = [0, 1, 2, 3];
        let supply = [0.0; 4];
        // The last sweep started from a full lake (50 + ε) and cut it 2 + ε m down; the slope
        // lost 1 m and the pass cell 10 m. Only 3 + ε m³ reach the lake: it holds that above h*
        // and keeps its incision, 2 + ε m below its fill, and it lets go nothing.
        let full = [0.0, 50.0, 50.0 + EPSILON, 100.0];
        let mut height = vec![0.0, 40.0, 48.0, 99.0];
        let ledger = Ledger {
            h_star: &h_star,
            start: &full,
            supply: &supply,
            cell_area: 1.0,
        };
        let booked = books(&stack, &weights, &traps, &ledger, &mut height);
        assert!(close(height[2], 45.0 + (3.0 + EPSILON) - (2.0 + EPSILON)));
        assert_eq!(height[..2], [0.0, 40.0]);
        assert_eq!(height[3].to_bits(), 99.0_f64.to_bits());
        // Out: every cell's own sediment, 13 + ε, less what the lake took.
        assert!(close(booked.out, 10.0), "{}", booked.out);
        assert!(close(booked.correction, 2.0), "{}", booked.correction);
        // The last sweep started from 1 m of fill and the lake kept its height; the slope lost
        // 8 m, more than the lake holds. It fills to its place one ε above the spill level and
        // the rest leaves over the pass.
        let partly = [0.0, 50.0, 46.0, 100.0];
        let mut height = vec![0.0, 40.0, 46.0, 92.0];
        let ledger = Ledger {
            start: &partly,
            ..ledger
        };
        let booked = books(&stack, &weights, &traps, &ledger, &mut height);
        assert!(close(height[2], 50.0 + EPSILON), "{}", height[2]);
        assert!(close(booked.out, 18.0 - (5.0 + EPSILON)), "{}", booked.out);
        assert!(close(booked.correction, 4.0 + EPSILON));
    }

    #[test]
    fn without_traps_the_books_only_route() {
        // With no depressions the books are the walk of `step_sediment`'s volumes: the slope
        // supply and the net erosion leave at the outlet and no height moves.
        let (weights, _, h_star) = pass_strip();
        let mut height = vec![0.0, 49.0, 44.0, 97.0];
        let ledger = Ledger {
            h_star: &h_star,
            start: &h_star,
            supply: &[0.0, 0.0, 0.0, 2.0],
            cell_area: 1.0,
        };
        let before = height.clone();
        let booked = books(
            &[0, 1, 2, 3],
            &weights,
            &Traps::default(),
            &ledger,
            &mut height,
        );
        assert_eq!(height, before);
        assert_eq!(booked.out.to_bits(), 7.0_f64.to_bits());
        assert_eq!(booked.correction.to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn the_supply_is_laid_down_on_its_way_and_a_depression_keeps_its_share() {
        // Depression 1 holds cells 2 and 3 and spills from cell 2 (inside it) to cell 1; it holds
        // 200 + ε m³. Cells 1, 4 and 5 keep half of what passes through them; the lake cells and
        // the outlet keep nothing.
        let traps = traps(vec![0, 0, 1, 1, 0, 0], &[(2, 1)]);
        let keeps = [0.0, 0.5, 0.0, 0.0, 0.5, 0.5];
        let half = |i: usize, through: f64| through * keeps[i];
        // 300 m³ on cell 5: it keeps 150, cell 4 keeps 75 of the rest and sends 75 into the
        // depression (a quarter over cell 3, three quarters to cell 2), which keeps all of it.
        let mut height = vec![0.0; 6];
        lay_down(
            &STACK,
            &strip(),
            &traps,
            &[0.0, 0.0, 0.0, 0.0, 0.0, 300.0],
            1.0,
            &mut height,
            half,
        );
        assert_eq!(height, [0.0, 0.0, 0.0, 0.0, 75.0, 150.0]);
        // 1,000 m³: the depression gets 250 and lets go the excess over its capacity at its
        // spill cell, which passes it on to cell 1; cell 1 keeps half of it.
        let mut height = vec![0.0; 6];
        lay_down(
            &STACK,
            &strip(),
            &traps,
            &[0.0, 0.0, 0.0, 0.0, 0.0, 1000.0],
            1.0,
            &mut height,
            half,
        );
        let excess = 250.0 - (200.0 + EPSILON);
        assert!(close(height[1], excess / 2.0), "{}", height[1]);
        assert_eq!(height[2..], [0.0, 0.0, 250.0, 500.0]);
        assert_eq!(height[0].to_bits(), 0.0_f64.to_bits(), "the outlet");
        // Cells of 4 m² rise by a quarter of the volume they keep.
        let mut height = vec![0.0; 6];
        lay_down(
            &STACK,
            &strip(),
            &traps,
            &[0.0, 0.0, 0.0, 0.0, 0.0, 300.0],
            4.0,
            &mut height,
            half,
        );
        assert_eq!(height, [0.0, 0.0, 0.0, 0.0, 18.75, 37.5]);
    }

    #[test]
    fn without_supply_nothing_is_laid_down() {
        // Runs without slope supply stay bit for bit as they were: no height is touched, not
        // even a negative zero.
        let traps = traps(vec![0, 0, 1, 1, 0, 0], &[(2, 1)]);
        let before = [-0.0, 1.0, -0.0, 3.0, 4.0, -0.0];
        let mut height = before.to_vec();
        lay_down(
            &STACK,
            &strip(),
            &traps,
            &[0.0; 6],
            1.0,
            &mut height,
            |_, through| through * 0.5,
        );
        let bits = |h: &[f64]| h.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&height), bits(&before));
    }

    #[test]
    fn nested_depressions_pass_their_excess_downstream_into_the_next() {
        // Depression 1 (cell 2) spills from cell 2 into cell 1; depression 2 (cell 4) spills from
        // cell 4 into cells 3 and 2. Depression 2 closes first and its excess reaches depression
        // 1 both directly and through cell 3.
        let traps = traps(vec![0, 0, 1, 0, 2, 0], &[(2, 1), (4, 3)]);
        let mut arriving = vec![0.0; 6];
        let volume = [0.0, 0.0, 0.0, 1.0, 0.0, 16.0];
        let mut calls = Vec::new();
        let out = route(&STACK, &strip(), &traps, &volume, &mut arriving, |n, v| {
            calls.push((n, v));
            v / 2.0
        });
        // Depression 2 keeps 8 of 16 and lets go 8 at its spill cell 4: 2 to cell 3, 6 into
        // depression 1. Cell 3 passes 2 + 1 into depression 1, which holds 9 and lets go 4.5 at
        // its spill cell 2.
        assert_eq!(calls, [(1, 16.0), (0, 9.0)]);
        assert_eq!(arriving, [4.5, 4.5, 4.5, 2.0, 8.0, 0.0]);
        assert_eq!(out.to_bits(), 4.5_f64.to_bits());
    }
}
