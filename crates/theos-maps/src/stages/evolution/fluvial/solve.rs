//! The implicit stream-power step with n = 1 over multiple flow directions (Braun & Willett
//! 2013, in `FastScape`'s multiple-flow form; spec §5.9).
//!
//! Discretising `dh/dt = −K·Q^m·Σ_r w_r·(h − h_r) / d_r` implicitly, with every receiver `r`
//! already at its new height, gives for each cell `i` in stack order (receivers first)
//!
//! ```text
//! F_ir = K_i · Q_i^m · w_ir · Δt / d_ir
//! h_i  = (h*_i + Σ_r F_ir · h_r) / (1 + Σ_r F_ir)
//! ```
//!
//! with `h*` the height entering the process (uplift already applied). One sweep solves it
//! exactly: no iteration is needed while nothing couples a cell to its donors.

use libm::pow;

use crate::grid::Grid;
use crate::stages::evolution::flow::{FlowWeights, cell_index};

/// What one implicit step reads besides the starting heights. Every slice holds one entry per
/// cell, and `weights` and `stack` name only cells of the grid.
pub(super) struct Incision<'a> {
    /// Where each cell's water goes.
    pub weights: &'a FlowWeights,
    /// The cells, receivers before donors.
    pub stack: &'a [u32],
    /// The cells that keep their height (the drained edges).
    pub fixed: &'a [bool],
    /// The erodibility K.
    pub erodibility: &'a [f64],
    /// The runoff-weighted drainage area Q, m².
    pub discharge: &'a [f64],
    /// The exponent m of Q.
    pub m: f64,
    /// The time step, years.
    pub dt: f64,
}

/// The heights after one implicit stream-power step from `start` (`h*`).
///
/// A cell keeps its height when it is fixed, has no edge (an outlet or the closed map's sink),
/// or gathers no water (`Q = 0`, so it has no fluvial term). A receiver that is not lower than
/// `h*_i` takes no share: stream power erodes only downhill, so the step never raises a cell.
/// That happens over a pass link that climbs and inside a routed depression, whose routing
/// surface stands above the height.
pub(super) fn incise(step: &Incision<'_>, start: &Grid<f64>) -> Grid<f64> {
    let mut height = start.clone();
    let (before, h) = (start.as_slice(), height.as_mut_slice());
    for &cell in step.stack {
        let i = cell_index(cell);
        let edges = step.weights.edges(i);
        if step.fixed[i] || edges.is_empty() || step.discharge[i] <= 0.0 {
            continue;
        }
        let power = step.erodibility[i] * pow(step.discharge[i], step.m) * step.dt;
        let (mut top, mut bottom) = (before[i], 1.0);
        for e in edges {
            let receiver = h[cell_index(step.weights.to[e])];
            if receiver >= before[i] {
                continue;
            }
            let f = power * step.weights.weight[e] / step.weights.dist[e];
            top += f * receiver;
            bottom += f;
        }
        h[i] = top / bottom;
    }
    height
}
