//! The flow weights of `partition_flow`: each cell's edges to its receivers.

use crate::kernels::routing::d8_distance;
use libm::pow;

use super::Exponent;
use crate::grid::{D8, Grid};
use crate::kernels::num::{max, to_u32, to_usize};
use crate::stages::evolution::flow::FlowWeights;

/// The weights of every cell of `surface`, or the first cell (and its receiver) whose receiver is
/// not a D8 neighbour.
///
/// A cell that receives itself has no edge. Otherwise `Steepest` sends everything to the
/// receiver; `SlopeAdaptive` spreads it over the strictly lower neighbours in `D8` order by
/// `slope^p`, `p = 0.5 + 0.6·S`, and falls back to the receiver when none is lower (a pass link).
/// The shares are computed as `(slope / S)^p`, the same proportions without overflow.
pub(super) fn build(
    surface: &Grid<f64>,
    receivers: &Grid<u32>,
    cell_m: f64,
    exponent: Exponent,
) -> Result<FlowWeights, (usize, u32)> {
    let heights = surface.as_slice();
    let mut out = FlowWeights {
        start: Vec::with_capacity(surface.len() + 1),
        to: Vec::new(),
        weight: Vec::new(),
        dist: Vec::new(),
    };
    for i in 0..surface.len() {
        out.start.push(to_u32(out.to.len()));
        let receiver = receivers.as_slice()[i];
        if to_usize(receiver) == i {
            continue;
        }
        let (r, c) = surface.cell(i);
        let mut lower: Vec<(usize, f64, f64)> = Vec::new();
        let mut to_receiver = None;
        for (dir, &offset) in D8.iter().enumerate() {
            let Some(next) = surface.neighbour_at(r, c, offset) else {
                continue;
            };
            let dist = d8_distance(cell_m, dir);
            if next == to_usize(receiver) {
                to_receiver = Some(dist);
            }
            let drop = heights[i] - heights[next];
            if drop > 0.0 {
                lower.push((next, drop / dist, dist));
            }
        }
        let Some(receiver_dist) = to_receiver else {
            return Err((i, receiver));
        };
        if exponent == Exponent::Steepest || lower.is_empty() {
            push(&mut out, receiver, 1.0, receiver_dist);
            continue;
        }
        let steepest = lower.iter().fold(0.0, |s, &(_, slope, _)| max(s, slope));
        let p = 0.5 + 0.6 * steepest;
        let shares: Vec<f64> = lower
            .iter()
            .map(|&(_, slope, _)| pow(slope / steepest, p))
            .collect();
        let total: f64 = shares.iter().sum();
        for (&(next, _, dist), share) in lower.iter().zip(shares) {
            push(&mut out, to_u32(next), share / total, dist);
        }
    }
    out.start.push(to_u32(out.to.len()));
    Ok(out)
}

/// Appends one edge.
fn push(w: &mut FlowWeights, to: u32, weight: f64, dist: f64) {
    w.to.push(to);
    w.weight.push(weight);
    w.dist.push(dist);
}
