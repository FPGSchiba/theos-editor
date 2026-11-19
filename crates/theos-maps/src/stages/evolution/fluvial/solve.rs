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
//! with `h*` the height entering the process (uplift already applied) and `Σ_r` over the
//! receivers below `h_i`, the cell's new height (see `cell`): stream power erodes only downhill.
//! The cell's own term sits on the diagonal (`c_i`); `Qs_in` comes from the cells strictly
//! upstream (Yuan's `ups(i)`).
//!
//! **Why the cell's own term is included.** At steady state `Qs = U·A` exactly, so the deposition
//! term is `G·U` and every cell's slope is `(1 + G)` times its slope at G = 0, as Yuan's eq. 24
//! and 28 (the continuum) and physics test 14 require. Yuan's discrete scheme sums `ups(i)`
//! alone, which gives `1 + G·(N − 1)/N` for a cell draining N cells: 1 on ridges, 1.909 at
//! N = 11. Its steady slope then rises from N = 1 to N = 2, so a ridge cell that captures a
//! donor stands higher, loses it again, and the landscape cycles without a steady state.
//!
//! **The slope supply** (spec §5.9 "Slope supply"). `slope_limit` removes a thickness `s_i` from a
//! cell and hands it on; it enters `Qs` at that cell like eroded sediment. `Qs` is a volume flux,
//! so the supply adds `s_i·area/Δt` to the flux leaving cell `i`, beside its net erosion
//! `(h*_i − h_i)·area/Δt`. In the deposition term `Δt·G·Qs/Q` that is `G·s_i·area/Q_i = c_i·s_i`
//! at the cell itself, and downstream it travels with the routed volume `s_i·area`. The step then
//! reads, with `V_i = s_i·area` the volume taken in,
//!
//! ```text
//! h_i  = (h*_i·(1 + c_i) + Δt·G·(Qs_in(i) + V_i/Δt)/Q_i + Σ_r F_ir · h_r) / (1 + c_i + Σ_r F_ir)
//! ```
//!
//! with `Σ_r` again over the receivers below `h_i`, and `Qs_in` routes `(h*_j − h_j)·area + V_j`
//! from every cell `j` upstream. A routed depression's cell routes `(s_j − h_j)·area + V_j` from
//! its filled start `s_j` instead, and its own inflow term is 0: what reaches it is in its fill
//! (see below). G thus decides how much of the supply is laid down and how much passes on; with
//! G = 0 the supply does not enter the heights and all of it leaves at the outlets (`books`).
//!
//! When the recipe lets the supply pass dry cells (`runout`), the volume `P_i` of supply that
//! passes cell `i` without being laid down leaves its deposition term:
//! `Δt·G·(Qs_in(i) + V_i/Δt)/Q_i` becomes `G·(Qs_in(i)·Δt + V_i − P_i)/Q_i`. `P_i` is 0 on every
//! cell whose catchment is not dry, and everywhere without the switch.
//!
//! With G = 0 one sweep solves the step exactly. With G > 0 `Qs_in` couples every cell to its
//! donors, and Yuan's Gauss–Seidel iterations solve it ([`deposit`]): each iteration routes the
//! sediment of the last iterate, then sweeps once. Where runoff is low the lagged sediment makes
//! the iterations oscillate; once they are not on track to reach the tolerance in time, each one
//! but the last is relaxed by Aitken's factor ([`Relaxation`]).
//!
//! **Where the iterations start.** Not at `h*` but at `h*` with the step's slope supply laid down
//! along its way ([`first_iterate`]): every cell that is solved keeps `c_i/(1 + c_i)` of what
//! passes through it and passes the rest on, as the step above lays it down where nothing
//! incises (`K = 0`, no receiver taking a share), and the routed depressions keep what reaches
//! them up to their capacity. From `h*` the first sweep would route all the supply undeposited:
//! on dry cells, where `c_i` is 100 or more, it then lay hundreds of metres too far downstream,
//! and the transient that followed outlasted the iteration limit (Task 27g). The step and its
//! solution are unchanged; only the first iterate is. Without supply it is `h*` bit for bit.
//!
//! **Routed depressions** (spec §5.9 "Depression filling") are filled in each iteration's step 1,
//! before the sweep, in the spec's order. Each one holds the sediment that reaches it up to its
//! capacity (`fill`), and that fill is the start of its cells for the sweep. The sweep then
//! solves every cell, the depressions' cells included. The sediment that reached a depression,
//! its cells' supply included, is in the fill, so their deposition term sees none of it. A lake
//! that is not full has no receiver below its filled level and stays where it is; a full one
//! starts at its spill level and follows the incision of its spill node, as a river does.
//!
//! Two consequences of filling before the solve:
//! - The dry floor of a lake that is not full (its cells above the water) is solved like any
//!   cell and erodes where its receiver lies lower; its sediment goes into the fill, so the
//!   basin's sediment lies lower in it, not in other amounts.
//! - A spill cell inside its depression is a lake cell: its deposition term sees no release,
//!   while a spill cell outside it (the higher cell of the pass) sees the release like sediment
//!   from upstream.

use libm::pow;

use super::cell::{Receiver, solve};
use super::fill::Traps;
use super::relax::{Relaxation, toward};
use super::sediment::{lay_down, route, step_sediment};
use crate::grid::Grid;
use crate::kernels::num::max;
use crate::stage::StageError;
use crate::stages::evolution::flow::{FlowWeights, cell_index};

/// What one implicit step reads besides the starting heights. Every slice holds one entry per
/// cell, and `weights` and `stack` name only cells of the grid.
pub(super) struct Incision<'a> {
    /// Where each cell's water goes.
    pub weights: &'a FlowWeights,
    /// The cells, receivers before donors.
    pub stack: &'a [u32],
    /// The cells that keep their height: the drained edges, the base level.
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

/// The deposition terms of one sweep, per cell.
struct Deposit<'a> {
    /// `G·(Qs_in·Δt + V − P)/Q`, metres: the sediment from upstream and the slope supply `V`
    /// taken in at the cell, less the supply `P` that passes it without being laid down; 0 on a
    /// routed depression's cell, whose sediment is in its fill.
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

/// One implicit sweep from `start` (`h*`, with the routed depressions filled when there is
/// deposition) into `h`, in stack order, with the deposition `terms` if any.
///
/// A cell keeps its start when it is fixed, has no edge (an outlet or the closed map's sink), or
/// gathers no water (`Q = 0`, so it has no fluvial term). Otherwise [`solve`] gives its new
/// height, at which exactly the receivers below it take a share: stream power erodes only
/// downhill, so incision never raises a cell above the height it reaches without incision,
/// `(h*_i·(1 + c_i) + Δt·G·(Qs_in + V/Δt)/Q_i) / (1 + c_i)` (`h*_i` without deposition), and a
/// receiver above the cell never pulls it up. Receivers lie above their cell over a pass link
/// that climbs, inside a routed depression (whose routing surface stands above the height), and
/// where a receiver's deposit lifts it above its donor.
fn sweep(step: &Incision<'_>, start: &[f64], terms: Option<&Deposit<'_>>, h: &mut [f64]) {
    h.copy_from_slice(start);
    let mut receivers: Vec<Receiver> = Vec::with_capacity(8);
    for &cell in step.stack {
        let i = cell_index(cell);
        let edges = step.weights.edges(i);
        if step.fixed[i] || edges.is_empty() || step.discharge[i] <= 0.0 {
            continue;
        }
        let (top, bottom) = match terms {
            None => (start[i], 1.0),
            Some(d) => {
                let diagonal = 1.0 + d.own[i];
                (start[i] * diagonal + d.inflow[i], diagonal)
            }
        };
        let power = step.erodibility[i] * pow(step.discharge[i], step.m) * step.dt;
        receivers.clear();
        receivers.extend(edges.map(|e| Receiver {
            height: h[cell_index(step.weights.to[e])],
            share: power * step.weights.weight[e] / step.weights.dist[e],
        }));
        h[i] = solve(top, bottom, &receivers);
    }
}

/// The deposition terms of one step: the coefficient, the traps and how the iterations stop.
pub(super) struct Deposition<'a> {
    /// The dimensionless deposition coefficient G, above 0.
    pub g: f64,
    /// The routed depressions, which the sediment that reaches them fills up to their capacity.
    pub traps: &'a Traps,
    /// The area of one cell, m².
    pub cell_area: f64,
    /// Per cell, the volume of slope supply taken in this step, m³ (`s_i × area`).
    pub supply: &'a [f64],
    /// Per cell, the volume of slope supply that passes it without being laid down, m³
    /// (`runout`); 0 everywhere unless the recipe lets the supply pass dry cells.
    pub passing: &'a [f64],
    /// The iterations stop once no height changes by this much, metres.
    pub tolerance: f64,
    /// The most iterations.
    pub max_iterations: u32,
    /// The cancel point, called at the start of every iteration.
    pub cancel: &'a dyn Fn() -> Result<(), StageError>,
}

/// The result of the Gauss–Seidel iterations.
pub(super) struct Solution {
    /// The heights of the last iterate.
    pub height: Grid<f64>,
    /// The start of the sweep that gave them: `h*` with each routed depression filled.
    pub start: Vec<f64>,
    /// The iterations run.
    pub iterations: u32,
    /// The largest height change of the last iteration, metres.
    pub residual: f64,
    /// Whether the iterations ran past half the limit, where full depressions stay full.
    pub past_half: bool,
}

impl Solution {
    /// Whether the last iteration changed every height by less than `tolerance`.
    pub(super) fn converged(&self, tolerance: f64) -> bool {
        self.residual < tolerance
    }
}

/// Lays the step's slope supply down onto `height` (`h*`) along its way, the first iterate of
/// [`deposit`]: every cell that is not fixed and not in a routed depression keeps `c/(1 + c)` of
/// what passes through it (`own` holds `c = G·area/Q`, 0 without water), less the supply that
/// passes it without being laid down (`dep.passing`), and the depressions keep what reaches them
/// up to their capacity ([`lay_down`]). Where nothing incises this is the step's solution outside
/// the depressions. Without supply `height` keeps its bits.
fn first_iterate(
    step: &Incision<'_>,
    dep: &Deposition<'_>,
    own: &[f64],
    lake: &[bool],
    height: &mut [f64],
) {
    if dep.supply.iter().all(|&v| v == 0.0) {
        return;
    }
    let keeps: Vec<f64> = own
        .iter()
        .zip(lake)
        .zip(step.fixed)
        .map(
            |((&c, &in_lake), &fixed)| {
                if in_lake || fixed { 0.0 } else { c / (1.0 + c) }
            },
        )
        .collect();
    lay_down(
        step.stack,
        step.weights,
        dep.traps,
        dep.supply,
        dep.cell_area,
        height,
        |i, through| (through - dep.passing[i]) * keeps[i],
    );
}

/// Yuan et al.'s Gauss–Seidel iterations (eq. 19–21) over the weights, with the routed
/// depressions filled first in each iteration (spec §5.9's order).
///
/// The iterate is the heights `h^k` and the start `s^k` of the sweep that gave them. They begin
/// at `h⁰`, `h*` (`h_star`) with the step's slope supply laid down along its way
/// ([`first_iterate`]), and at `s⁰ = h*`, so a cell's own sediment in the first iteration is the
/// supply it passes on. Each iteration `k`:
/// 1. routes every cell's own sediment `(s^k_j − h^k_j)·area + V_j` in reverse stack order, so
///    each cell gets `Qs_in(i)·Δt`, the volume reaching it from upstream. A routed depression
///    gathers what reaches it, its cells' own sediment included, and holds up to its capacity of
///    it; it lets go only the excess (`Traps::release`). The volumes held, decided by
///    `Traps::decide`, fill a fresh copy of `h*`: the new start `ŝ`;
/// 2. sweeps once in stack order from `ŝ`, every cell but the fixed ones, with
///    `G·(Qs_in(i)·Δt + V_i − P_i)/Q_i` (`P_i` the supply passing it, `dep.passing`) and the
///    cell's own term `c_i`; a depression's cell has no inflow term, since what reached it is in
///    its fill;
/// 3. stops with the sweep's heights and `ŝ` when it changed no height by `tolerance` or more,
///    or after `max_iterations` (then with the plain sweep of the last iteration); otherwise
///    moves the heights toward the sweep's by Aitken's relaxation factor ([`Relaxation`]), and
///    the start and each depression's held volume by the same factor ([`toward`]), and iterates
///    again.
///
/// # Errors
/// What `dep.cancel` returns, checked before each iteration ([`StageError::Cancelled`]); nothing
/// of the iterate escapes.
pub(super) fn deposit(
    step: &Incision<'_>,
    dep: &Deposition<'_>,
    h_star: &Grid<f64>,
) -> Result<Solution, StageError> {
    let before = h_star.as_slice();
    let (cells, traps) = (before.len(), dep.traps);
    let lake: Vec<bool> = (0..cells).map(|i| traps.holds(i)).collect();
    let share = |q: f64, x: f64| if q > 0.0 { dep.g * x / q } else { 0.0 };
    let own: Vec<f64> = step
        .discharge
        .iter()
        .map(|&q| share(q, dep.cell_area))
        .collect();
    let (mut height, mut next) = (h_star.clone(), h_star.clone());
    first_iterate(step, dep, &own, &lake, height.as_mut_slice());
    // The start of the iterate and the next one.
    let (mut start, mut filled) = (before.to_vec(), before.to_vec());
    // Per depression: the volume the routing keeps, the one the start holds, and the next one.
    let depressions = traps.len();
    let (mut target, mut kept, mut decided) = (
        vec![0.0; depressions],
        vec![0.0; depressions],
        vec![0.0; depressions],
    );
    let (mut volume, mut arriving, mut inflow) =
        (vec![0.0; cells], vec![0.0; cells], vec![0.0; cells]);
    let half = dep.max_iterations / 2;
    let (mut iterations, mut residual) = (0, f64::INFINITY);
    let mut relaxation = Relaxation::new(dep.tolerance, dep.max_iterations);
    while iterations < dep.max_iterations {
        (dep.cancel)()?;
        iterations += 1;
        // Step 1: route the iterate's sediment and fill the depressions.
        step_sediment(
            &mut volume,
            &start,
            height.as_slice(),
            dep.supply,
            dep.cell_area,
        );
        route(
            step.stack,
            step.weights,
            traps,
            &volume,
            &mut arriving,
            |n, reached| {
                let (keeps, release) = traps.release(n, reached);
                target[n] = keeps;
                release
            },
        );
        let past_half = iterations > half;
        for (n, held) in decided.iter_mut().enumerate() {
            *held = traps.decide(n, target[n], kept[n], past_half);
        }
        traps.start(&decided, before, &mut filled);
        // Step 2: solve every cell from the start.
        let reaching = arriving.iter().zip(dep.supply).zip(dep.passing).zip(&lake);
        for ((x, &q), (((&a, &s), &p), &in_lake)) in
            inflow.iter_mut().zip(step.discharge).zip(reaching)
        {
            *x = if in_lake { 0.0 } else { share(q, a + s - p) };
        }
        let terms = Deposit {
            inflow: &inflow,
            own: &own,
        };
        sweep(step, &filled, Some(&terms), next.as_mut_slice());
        // Step 3: stop, or relax the heights and what belongs to them.
        residual = next
            .as_slice()
            .iter()
            .zip(height.as_slice())
            .fold(0.0, |most, (a, b)| max(most, (a - b).abs()));
        let converged = residual < dep.tolerance;
        let last = iterations == dep.max_iterations;
        if !converged && !last {
            let factor = relaxation.apply(height.as_slice(), next.as_mut_slice(), residual);
            toward(&start, &mut filled, factor);
            toward(&kept, &mut decided, factor);
        }
        std::mem::swap(&mut height, &mut next);
        std::mem::swap(&mut start, &mut filled);
        std::mem::swap(&mut kept, &mut decided);
        if converged {
            break;
        }
    }
    Ok(Solution {
        height,
        start,
        iterations,
        residual,
        past_half: iterations > half,
    })
}
