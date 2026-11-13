//! The implicit erosion–deposition step with n = 1 over multiple flow directions (Braun &
//! Willett 2013; Yuan et al. 2019, eq. 13–21, generalised to the flow weights; spec §5.9).
//!
//! The law is `dh/dt = U − K·Q^m·Σ_r w_r·(h − h_r) / d_r + G·Qs/Q`. `Qs` is the sediment flux
//! *leaving* the cell, as spec §5.9 defines it: what reaches it from upstream, `Qs_in`, plus its
//! own net erosion `(U − ∂h/∂t)·area`. Discretised implicitly, with every receiver `r` already at
//! its new height, that gives for each cell `i` in stack order (receivers first)
//!
//! ```text
//! F_ir = K_i · Q_i^m · w_ir · Δt / d_ir,      c_i = G · area / Q_i
//! h_i  = (h*_i·(1 + c_i) + Δt·G·Qs_in(i)/Q_i + Σ_r F_ir · h_r) / (1 + c_i + Σ_r F_ir)
//! ```
//!
//! with `h*` the height entering the process (uplift already applied). The cell's own term sits
//! on the diagonal (`c_i`); `Qs_in` comes from the cells strictly upstream (Yuan's `ups(i)`).
//!
//! **Why the cell's own term is included.** At steady state `Qs = U·A` exactly, so the deposition
//! term is `G·U` and every cell's slope is `(1 + G)` times its slope at G = 0, as Yuan's eq. 24
//! and 28 (the continuum) and physics test 14 require. Yuan's discrete scheme sums `ups(i)`
//! alone, which gives `1 + G·(N − 1)/N` for a cell draining N cells: 1 on ridges, 1.909 at
//! N = 11. Its steady slope then rises from N = 1 to N = 2, so a ridge cell that captures a
//! donor stands higher, loses it again, and the landscape cycles without a steady state.
//!
//! With G = 0 one sweep solves the step exactly. With G > 0 `Qs_in` couples every cell to its
//! donors, and Yuan's Gauss–Seidel iterations solve it ([`deposit`]): each iteration routes the
//! net erosion of the last iterate, `(h*_j − h^k_j)·area`, then sweeps once. Where runoff is low
//! the lagged sediment makes the iterations oscillate; once they are not on track to reach the
//! tolerance in time, each one is relaxed by Aitken's factor ([`Relaxation`]).

use libm::pow;

use super::fill::Traps;
use super::relax::Relaxation;
use super::sediment::route;
use crate::grid::Grid;
use crate::kernels::num::max;
use crate::stages::evolution::flow::{FlowWeights, cell_index};

/// What one implicit step reads besides the starting heights. Every slice holds one entry per
/// cell, and `weights` and `stack` name only cells of the grid.
pub(super) struct Incision<'a> {
    /// Where each cell's water goes.
    pub weights: &'a FlowWeights,
    /// The cells, receivers before donors.
    pub stack: &'a [u32],
    /// The cells that keep their height: the drained edges and, with deposition, the routed
    /// depressions, which only sediment filling changes.
    pub held: &'a [bool],
    /// The erodibility K.
    pub erodibility: &'a [f64],
    /// The runoff-weighted drainage area Q, m².
    pub discharge: &'a [f64],
    /// The exponent m of Q.
    pub m: f64,
    /// The time step, years.
    pub dt: f64,
}

/// The deposition terms of one sweep, per cell.
struct Deposit<'a> {
    /// `Δt·G·Qs_in/Q`, metres: the sediment from upstream.
    inflow: &'a [f64],
    /// `c = G·area/Q`: the cell's own net erosion, on the diagonal.
    own: &'a [f64],
}

/// The heights after one implicit stream-power step from `start` (`h*`), without deposition.
pub(super) fn incise(step: &Incision<'_>, start: &Grid<f64>) -> Grid<f64> {
    let mut height = start.clone();
    sweep(step, start.as_slice(), None, height.as_mut_slice());
    height
}

/// One implicit sweep from `start` (`h*`) into `h`, in stack order, with the deposition `terms` if
/// any.
///
/// A cell keeps its height when it is held, has no edge (an outlet or the closed map's sink), or
/// gathers no water (`Q = 0`, so it has no fluvial term). A receiver that is not lower than the
/// height the cell reaches without incision, `(h*_i·(1 + c_i) + Δt·G·Qs_in/Q_i) / (1 + c_i)`,
/// takes no share: stream power erodes only downhill, so incision never raises a cell. With a
/// single receiver this makes the step consistent: the cell erodes toward its receiver exactly
/// when it ends above it. It happens over a pass link that climbs, inside a routed depression
/// (whose routing surface stands above the height), and where a receiver's deposit lifts it
/// above its donor. Without deposition that height is `h*_i`.
fn sweep(step: &Incision<'_>, start: &[f64], terms: Option<&Deposit<'_>>, h: &mut [f64]) {
    h.copy_from_slice(start);
    for &cell in step.stack {
        let i = cell_index(cell);
        let edges = step.weights.edges(i);
        if step.held[i] || edges.is_empty() || step.discharge[i] <= 0.0 {
            continue;
        }
        let (mut top, mut bottom, base) = match terms {
            None => (start[i], 1.0, start[i]),
            Some(d) => {
                let diagonal = 1.0 + d.own[i];
                let top = start[i] * diagonal + d.inflow[i];
                (top, diagonal, top / diagonal)
            }
        };
        let power = step.erodibility[i] * pow(step.discharge[i], step.m) * step.dt;
        for e in edges {
            let receiver = h[cell_index(step.weights.to[e])];
            if receiver >= base {
                continue;
            }
            let f = power * step.weights.weight[e] / step.weights.dist[e];
            top += f * receiver;
            bottom += f;
        }
        h[i] = top / bottom;
    }
}

/// The deposition terms of one step: the coefficient, the traps and how the iterations stop.
pub(super) struct Deposition<'a> {
    /// The dimensionless deposition coefficient G, above 0.
    pub g: f64,
    /// The routed depressions, which keep the sediment that reaches them up to their capacity.
    pub traps: &'a Traps,
    /// The area of one cell, m².
    pub cell_area: f64,
    /// The iterations stop once no height changes by this much, metres.
    pub tolerance: f64,
    /// The most iterations.
    pub max_iterations: u32,
}

/// The result of the Gauss–Seidel iterations.
pub(super) struct Solution {
    /// The heights of the last iterate.
    pub height: Grid<f64>,
    /// The iterations run.
    pub iterations: u32,
    /// The largest height change of the last iteration, metres.
    pub residual: f64,
}

impl Solution {
    /// Whether the last iteration changed every height by less than `tolerance`.
    pub(super) fn converged(&self, tolerance: f64) -> bool {
        self.residual < tolerance
    }
}

/// Yuan et al.'s Gauss–Seidel iterations (eq. 19–21) from `h⁰ = h*` (`start`), over the weights.
///
/// Each iteration `k`:
/// 1. routes every cell's net erosion `(h*_j − h^k_j)·area` in reverse stack order, so each cell
///    gets `Qs_in(i)·Δt`, the volume reaching it from upstream; a routed depression keeps what
///    reaches it up to its capacity and lets the rest go at its spill cell (`traps`);
/// 2. sweeps once in stack order with `Δt·G·Qs_in(i)/Q_i` and the cell's own term `c_i`;
/// 3. stops with the sweep's heights when it changed no height by `tolerance` or more, or after
///    `max_iterations`; otherwise moves the heights toward the sweep's by Aitken's relaxation
///    factor ([`Relaxation`]) and iterates again.
pub(super) fn deposit(step: &Incision<'_>, dep: &Deposition<'_>, start: &Grid<f64>) -> Solution {
    let before = start.as_slice();
    let cells = before.len();
    let mut height = start.clone();
    let mut next = start.clone();
    let share = |q: f64, x: f64| if q > 0.0 { dep.g * x / q } else { 0.0 };
    let own: Vec<f64> = step
        .discharge
        .iter()
        .map(|&q| share(q, dep.cell_area))
        .collect();
    let (mut volume, mut arriving, mut inflow) =
        (vec![0.0; cells], vec![0.0; cells], vec![0.0; cells]);
    let (mut iterations, mut residual) = (0, f64::INFINITY);
    let mut relaxation = Relaxation::new(dep.tolerance, dep.max_iterations);
    while iterations < dep.max_iterations {
        iterations += 1;
        for ((v, &b), &h) in volume.iter_mut().zip(before).zip(height.as_slice()) {
            *v = (b - h) * dep.cell_area;
        }
        route(
            step.stack,
            step.weights,
            dep.traps,
            &volume,
            &mut arriving,
            |n, reached| reached - dep.traps.kept(n, reached),
        );
        for ((x, &q), &a) in inflow.iter_mut().zip(step.discharge).zip(&arriving) {
            *x = share(q, a);
        }
        let terms = Deposit {
            inflow: &inflow,
            own: &own,
        };
        sweep(step, before, Some(&terms), next.as_mut_slice());
        residual = next
            .as_slice()
            .iter()
            .zip(height.as_slice())
            .fold(0.0, |most, (a, b)| max(most, (a - b).abs()));
        let converged = residual < dep.tolerance;
        if !converged {
            relaxation.apply(height.as_slice(), next.as_mut_slice(), residual);
        }
        std::mem::swap(&mut height, &mut next);
        if converged {
            break;
        }
    }
    Solution {
        height,
        iterations,
        residual,
    }
}
