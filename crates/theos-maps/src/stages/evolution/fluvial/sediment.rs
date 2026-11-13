//! The sediment of one step routed over the flow weights (spec §5.9): each cell passes on what
//! reaches it plus its own sediment, split like its water. A cell's own sediment is its net
//! erosion and the slope supply it takes in ([`step_sediment`]). A routed depression keeps what
//! reaches it while it is open and lets go what it does not keep when the walk reaches its spill
//! cell.

use super::fill::Traps;
use crate::stages::evolution::flow::{FlowWeights, cell_index};

/// Writes each cell's own sediment of the step into `volume`, m³: its net erosion
/// `(start − height)·cell_area` (negative where it gained), plus `supply`, the volume of slope
/// supply it takes in this step (spec §5.9 "Slope supply"). The supply enters the sediment flux
/// at its cell like eroded sediment: `slope_limit` removed it from that cell since `fluvial` last
/// ran.
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
/// depression's inflow instead, and so does each of its cells' own volume (the slope supply on a
/// held cell), wherever that cell's edges lead: to the spill cell outside the depression too, or,
/// at a spill cell inside it, before the depression lets go. When the walk reaches the
/// depression's spill cell, `close(n, inflow)` returns the volume the depression lets go. It
/// counts as arriving at the spill cell, so it leaves with the spill cell's own flux over its
/// flow edges (with steepest routing, to `SpillPoint.to`), as spec §5.9's "passes on from the
/// spill node" says. A spill cell outside
/// its depression (the higher cell of the pass) thus sees the release in its deposition term;
/// one inside it is held and only passes it on. From then on the depression is closed: what
/// reaches it later flows through its cells like over any other cell. Every volume either
/// leaves the map, stays in a depression, or is in `arriving`, so the walk loses nothing.
pub(super) fn route(
    stack: &[u32],
    weights: &FlowWeights,
    traps: &Traps,
    volume: &[f64],
    arriving: &mut [f64],
    mut close: impl FnMut(usize, f64) -> f64,
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
        let leaving = arriving[i] + own;
        let edges = weights.edges(i);
        if edges.is_empty() {
            out += leaving;
            continue;
        }
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

/// Routes the step's sediment, `volume` per cell (m³, from [`step_sediment`] on the step's final
/// heights: the net erosion plus the slope supply taken in), once more and fills every
/// depression with the sediment that reaches it, raising its cells in `height`. Returns the
/// volume that leaves the map (m³). The volume each depression lets go is exactly what reached
/// it less what it took, so the sediment's volume is kept.
pub(super) fn settle(
    stack: &[u32],
    weights: &FlowWeights,
    traps: &Traps,
    volume: &[f64],
    height: &mut [f64],
) -> f64 {
    let mut arriving = vec![0.0; volume.len()];
    route(stack, weights, traps, volume, &mut arriving, |n, inflow| {
        inflow - traps.fill(n, inflow, height)
    })
}

#[cfg(test)]
mod tests {
    use super::route;
    use crate::grid::Grid;
    use crate::stages::evolution::flow::FlowWeights;
    use crate::stages::evolution::fluvial::fill::Traps;
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
        // the pass). Cell 3's own 5 (slope supply on a held cell) goes into the depression like
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
