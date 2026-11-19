//! The per-step process `fluvial`: the erosion–deposition law of Yuan et al. 2019 with n = 1
//! over the multiple-flow weights, solved implicitly in stack order, and sediment filling of the
//! routed depressions (spec §5.9 "The fluvial law", rulings 3, 28, 29, 43).
//!
//! The law is `dh/dt = U − K·Q^m·S + G·Qs/Q` (Yuan et al. 2019, eq. 11, after Davy & Lague
//! 2009), in `FastScape`'s multiple-flow form: `S` is the flow-weighted slope
//! `Σ_r w_r·(h − h_r) / d_r` over the cell's weighted receivers. `Q` is the runoff-weighted
//! drainage area, the upstream sum of `p̂ × cell area` with `p̂` the local runoff over the
//! map-mean runoff ([`runoff_weight`]). `Qs` is the sediment flux leaving the cell: what reaches
//! it from upstream plus its own net erosion (spec §5.9; `solve` says why its own term counts).
//!
//! - **G = 0** is detachment-limited stream power (Braun & Willett 2013): one implicit sweep, no
//!   deposition and no filling (spec ruling 29).
//! - **G > 0** couples each cell to its donors through `Qs`; Yuan's Gauss–Seidel iterations
//!   solve it (`solve`), stopping at 10⁻³ × max(U)·Δt, or 10⁻⁶ m without uplift (ruling 43).
//!   Where dry catchments break Yuan's condition `G/p̂ ≤ 1` they oscillate, and Aitken's
//!   relaxation (`relax`) damps them once they are not on track to converge in time; the
//!   solution they converge to is unchanged.
//! - **Depression filling**, in the spec's order: in each iteration the routed sediment first
//!   fills each routed depression in height order up to its spill level, with an ε slope toward
//!   its spill cell (`fill`), and passes the excess on from the spill node; then every cell is
//!   solved, the depressions' cells from their filled heights. A lake that is not full keeps its
//!   level and the slope that feeds it ends above it; a full lake follows the incision of its
//!   spill node instead of perching above it. Two consequences: the dry floor cells of a lake
//!   that is not full erode where their receiver lies lower (their sediment goes into the fill),
//!   and a spill cell inside its depression is a lake cell, whose deposition term sees no release,
//!   while one outside it sees the release (`solve` says more). From the iteration after half the
//!   limit a full depression stays full, so it changes state a bounded number of times.
//! - **The books** (`sediment`): after the iterations one more routing of the step's sediment
//!   over the final heights fills each depression with what reaches it up to its capacity, and
//!   gives its cells that fill in place of the one their last sweep started from. The volume
//!   balance closes exactly, also at the iteration limit: a lake that is not full keeps all of a
//!   positive inflow and lets nothing go, and an inflow that is not positive passes on with its
//!   sign. When the iterations ran past half the limit and the books moved a lake cell by the
//!   tolerance or more, a warning names the move.
//! - **The slope supply** (spec §5.9 "Slope supply", ruling 51): `slope_supply` holds, per cell,
//!   the thickness `slope_limit` has removed since `fluvial` last ran. `fluvial` takes it into the
//!   sediment flux at its cell like eroded sediment, in the iterations and in the books (`solve`
//!   gives the term); on a depression's cell it goes into the fill. It writes the layer back
//!   emptied. G decides how much of it is laid down; with G = 0 all of it leaves at the outlets.
//!   Without the layer there is no supply. With G > 0 the iterations start from `h*` with the
//!   supply laid down along its way, as the law lays it down where nothing incises (`solve`), so
//!   that dry cells do not route it undeposited in the first iteration.
//!
//! The report holds `sediment_out_m3`, the sediment that left the map in the step at the cells
//! without an edge (the drained edges, or the closed map's sink), `iterations`, and `supply_m3`,
//! the volume of slope supply taken in.

mod cell;
mod fill;
mod inputs;
mod relax;
mod sediment;
mod solve;
#[cfg(test)]
mod tests;

use serde::Deserialize;

use self::fill::Traps;
use self::inputs::{depressions_misfit, routing_misfit};
use self::sediment::{Books, Ledger, books};
use self::solve::{Deposition, Incision, Solution, deposit, incise};
use super::booking::book;
use super::flow::{FlowWeights, accumulate};
use crate::grid::Grid;
use crate::kernels::num::{max, to_f64};
use crate::stage::{
    Finding, LayerSpec, MapKind, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::evolution::resolve_sinks::Depressions;
use crate::stages::terrain::climate::Climate;
use crate::world::{Layer, MapFrame, ValueType};

/// Yuan et al.'s tolerance as a share of the largest uplift per step, max(U)·Δt (ruling 43).
const TOLERANCE_SHARE: f64 = 1e-3;

/// The tolerance when no cell uplifts, metres (ruling 43).
const FALLBACK_TOLERANCE_M: f64 = 1e-6;

/// The parameters of `fluvial`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FluvialParams {
    /// The drainage-area exponent m of `K·Q^m·S`; n is 1 (spec ruling 3).
    #[serde(default = "default_m")]
    pub m: f64,
    /// The dimensionless deposition coefficient `G = d*·v_s / r̄` (Yuan et al. 2019, eq. 9); 0 is
    /// detachment-limited stream power without deposition or depression filling (spec ruling
    /// 29). The recipe states it.
    pub g: f64,
    /// The most Gauss–Seidel iterations per step while deposition couples the cells (`g > 0`).
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
}

/// The default m, 0.45.
fn default_m() -> f64 {
    0.45
}

/// The default iteration limit, 50 (spec §5.9).
fn default_max_iterations() -> u32 {
    50
}

/// The process `fluvial`.
pub struct Fluvial;

impl TypedStage for Fluvial {
    type Params = FluvialParams;
    const ID: &'static str = "fluvial";
    const SUMMARY: &'static str = "The erosion–deposition law with n = 1 over the multiple-flow weights (Yuan et al. 2019), solved implicitly in stack order with Gauss–Seidel iterations; sediment fills routed depressions; runoff-weighted drainage area; cumulative erosion and deposition.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    /// With deposition (`g > 0`) also the routing surface, whose order fills the depressions,
    /// and the uplift, whose largest rate sets the tolerance; it may be absent.
    fn reads(&self, params: &FluvialParams) -> Vec<LayerSpec> {
        let mut reads = vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("erodibility"),
            LayerSpec::u32("receivers"),
            LayerSpec::value("weights", FlowWeights::NAME),
            LayerSpec::u32("stack"),
            LayerSpec::value("depressions", Depressions::NAME),
            LayerSpec::mask("fixed"),
            LayerSpec::value("climate", Climate::NAME),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("deposition"),
            LayerSpec::f64("slope_supply").optional(),
        ];
        if params.g > 0.0 {
            reads.push(LayerSpec::f64("routing_surface"));
            reads.push(LayerSpec::f64("uplift").optional());
        }
        reads
    }

    /// The slope supply is written back emptied when the world holds it.
    fn writes(&self, _: &FluvialParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("deposition"),
            LayerSpec::f64("drainage_area"),
            LayerSpec::f64("slope_supply").optional(),
        ]
    }

    fn validate(&self, params: &mut FluvialParams) -> Result<(), ParamError> {
        let at = |parameter: &str, message: &str| ParamError::at(Self::ID, parameter, message);
        if !(params.m.is_finite() && params.m >= 0.0) {
            return Err(at("m", "must be finite and not negative"));
        }
        if !(params.g.is_finite() && params.g >= 0.0) {
            return Err(at("g", "must be finite and not negative"));
        }
        if params.max_iterations == 0 {
            return Err(at("max_iterations", "must be at least 1"));
        }
        Ok(())
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &FluvialParams,
    ) -> Result<StageReport, StageError> {
        let Some(step) = ctx.step() else {
            return Err(ctx.fail("runs only as a process of landscape_evolution"));
        };
        let start = ctx.take_f64("height")?;
        let supply = take_supply(ctx)?;
        let outcome = erode(ctx, params, step.dt_yr, &start, supply.as_ref())?;
        let mut erosion = ctx.take_f64("erosion")?;
        let mut deposition = ctx.take_f64("deposition")?;
        book(&start, &outcome.height, &mut erosion, &mut deposition);
        ctx.write("height", Layer::F64(outcome.height))?;
        ctx.write("erosion", Layer::F64(erosion))?;
        ctx.write("deposition", Layer::F64(deposition))?;
        ctx.write("drainage_area", Layer::F64(outcome.drainage_area))?;
        if let Some(mut taken) = supply {
            taken.as_mut_slice().fill(0.0);
            ctx.write("slope_supply", Layer::F64(taken))?;
        }
        Ok(outcome.report)
    }
}

/// The slope supply, taken out of the world to be written back emptied; `None` when the world
/// holds none.
fn take_supply(ctx: &mut StageContext<'_>) -> Result<Option<Grid<f64>>, StageError> {
    if ctx.f64_opt("slope_supply")?.is_none() {
        return Ok(None);
    }
    ctx.take_f64("slope_supply").map(Some)
}

/// One step's result besides the cumulative layers.
struct Outcome {
    /// The new heights.
    height: Grid<f64>,
    /// The plain drainage area (spec §5.10: not weighted by runoff).
    drainage_area: Grid<f64>,
    /// `sediment_out_m3`, `iterations`, `supply_m3`, a warning when the iterations did not
    /// converge, and one when the books moved a lake cell by the tolerance or more after the
    /// iterations ran past half the limit.
    report: StageReport,
}

/// One implicit step from `start` (`h*`) over the current weights and stack, with deposition
/// and depression filling when `g > 0`, taking in the slope `supply` (thickness per cell) if any.
fn erode(
    ctx: &StageContext<'_>,
    params: &FluvialParams,
    dt: f64,
    start: &Grid<f64>,
    supply: Option<&Grid<f64>>,
) -> Result<Outcome, StageError> {
    let frame = *ctx.frame();
    let cells = frame.cells();
    let (weights, stack) = (ctx.value::<FlowWeights>("weights")?, ctx.u32("stack")?);
    let stack = stack.as_slice();
    if let Some(problem) = routing_misfit(weights, stack, cells) {
        return Err(ctx.fail(problem));
    }
    let climate = ctx.value::<Climate>("climate")?;
    let Some(p_hat) = runoff_weight(climate, &frame) else {
        let (rows, cols) = climate
            .fields
            .as_ref()
            .map_or((0, 0), |f| (f.runoff.rows(), f.runoff.cols()));
        return Err(ctx.fail(format!(
            "the climate's runoff covers {rows} x {cols} cells, but the grid has {} x {}",
            frame.rows, frame.cols
        )));
    };
    let cell_area = frame.cell_area_m2();
    let discharge = accumulate(stack, weights, &p_hat.map(|p| p * cell_area));
    let area = Grid::filled(frame.rows, frame.cols, cell_area);
    let drainage_area = accumulate(stack, weights, &area);
    let (fixed, erodibility) = (ctx.mask("fixed")?.as_slice(), ctx.f64("erodibility")?);
    // The supply as a volume per cell, m³ (thickness × cell area).
    let supply_m3: Vec<f64> = supply.map_or_else(
        || vec![0.0; cells],
        |s| s.as_slice().iter().map(|t| t * cell_area).collect(),
    );
    let mut report = StageReport::default();
    report
        .values
        .insert("supply_m3".to_owned(), supply_m3.iter().sum());
    let step = Incision {
        weights,
        stack,
        fixed,
        erodibility: erodibility.as_slice(),
        discharge: discharge.as_slice(),
        m: params.m,
        dt,
    };
    let (height, out) = if params.g > 0.0 {
        with_deposition(ctx, params, &step, start, &supply_m3, &mut report)?
    } else {
        report.values.insert("iterations".to_owned(), 1.0);
        let mut height = incise(&step, start);
        let ledger = Ledger {
            h_star: start.as_slice(),
            start: start.as_slice(),
            supply: &supply_m3,
            cell_area,
        };
        let books = books(
            stack,
            weights,
            &Traps::default(),
            &ledger,
            height.as_mut_slice(),
        );
        (height, books.out)
    };
    report.values.insert("sediment_out_m3".to_owned(), out);
    Ok(Outcome {
        height,
        drainage_area,
        report,
    })
}

/// The step with deposition (`g > 0`) from `start` (`h*`): the routed depressions as traps,
/// Yuan's Gauss–Seidel iterations, then the books. Returns the new heights and the sediment
/// that left the map, m³, and adds `iterations` and the warnings of the iterations and the books
/// to `report`.
fn with_deposition(
    ctx: &StageContext<'_>,
    params: &FluvialParams,
    step: &Incision<'_>,
    start: &Grid<f64>,
    supply_m3: &[f64],
    report: &mut StageReport,
) -> Result<(Grid<f64>, f64), StageError> {
    let depressions = ctx.value::<Depressions>("depressions")?;
    if let Some(problem) = depressions_misfit(depressions, start.len()) {
        return Err(ctx.fail(problem));
    }
    let cell_area = ctx.frame().cell_area_m2();
    let routing = ctx.f64("routing_surface")?.as_slice();
    let traps = Traps::new(depressions, routing, start.as_slice(), cell_area);
    let dep = Deposition {
        g: params.g,
        traps: &traps,
        cell_area,
        supply: supply_m3,
        tolerance: tolerance(ctx.f64_opt("uplift")?, step.dt),
        max_iterations: params.max_iterations,
        cancel: &|| ctx.check_cancel(),
    };
    let solution = deposit(step, &dep, start)?;
    let converged = solution.converged(dep.tolerance);
    let Solution {
        mut height,
        start: from,
        iterations,
        residual,
        past_half,
    } = solution;
    let ledger = Ledger {
        h_star: start.as_slice(),
        start: &from,
        supply: supply_m3,
        cell_area,
    };
    let Books { out, correction } = books(
        step.stack,
        step.weights,
        &traps,
        &ledger,
        height.as_mut_slice(),
    );
    if !converged {
        report.findings.push(
            Finding::warning(format!(
                "the Gauss–Seidel iterations stopped after {iterations} iterations with the \
                 residual {residual:e} m, above the tolerance {:e} m",
                dep.tolerance
            ))
            .on_layer("height"),
        );
    }
    if let Some(warning) = books_warning(iterations, past_half, correction, dep.tolerance) {
        report.findings.push(warning);
    }
    report
        .values
        .insert("iterations".to_owned(), f64::from(iterations));
    Ok((height, out))
}

/// The warning when the books moved a lake cell by `correction` metres, the tolerance or more,
/// at a step whose `iterations` ran `past_half` the limit: a depression's inflow was still
/// changing when the iterations stopped. A step that converged before half the limit never
/// warns; its correction follows only the last iteration's change of inflow.
fn books_warning(
    iterations: u32,
    past_half: bool,
    correction: f64,
    tolerance: f64,
) -> Option<Finding> {
    (past_half && correction >= tolerance).then(|| {
        Finding::warning(format!(
            "after {iterations} iterations the final routing of the sediment moved a lake cell \
             by {correction:e} m, at or above the tolerance {tolerance:e} m"
        ))
        .on_layer("height")
    })
}

/// Yuan et al.'s tolerance `10⁻³ × max(U)·Δt` for the largest `uplift` rate, or 10⁻⁶ m when no
/// cell uplifts or the layer is absent (spec ruling 43).
fn tolerance(uplift: Option<&Grid<f64>>, dt: f64) -> f64 {
    let most = uplift.map_or(0.0, |u| u.as_slice().iter().fold(0.0, |m, &x| max(m, x)));
    if most > 0.0 {
        TOLERANCE_SHARE * most * dt
    } else {
        FALLBACK_TOLERANCE_M
    }
}

/// The runoff weight `p̂ = runoff / mean(runoff)` of every cell (spec ruling 28): all 1 while
/// the climate has no fields yet, and all 0 when the mean runoff is 0 (no water, no stream).
/// `None` when the climate's runoff does not have the shape of `frame`.
#[must_use]
pub fn runoff_weight(climate: &Climate, frame: &MapFrame) -> Option<Grid<f64>> {
    let Some(fields) = &climate.fields else {
        return Some(Grid::filled(frame.rows, frame.cols, 1.0));
    };
    let runoff = &fields.runoff;
    if (runoff.rows(), runoff.cols()) != (frame.rows, frame.cols) {
        return None;
    }
    let total: f64 = runoff.as_slice().iter().sum();
    let mean = total / to_f64(runoff.len());
    Some(if mean > 0.0 {
        runoff.map(|r| r / mean)
    } else {
        runoff.map(|_| 0.0)
    })
}

#[cfg(test)]
mod unit_tests {
    use super::{books_warning, tolerance};
    use crate::grid::Grid;

    #[test]
    fn the_books_warn_only_past_half_the_limit_and_at_the_tolerance() {
        // A step that converged before half the limit never warns: its correction follows only
        // the last iteration's change of inflow (Task 27f, R8).
        assert!(books_warning(20, false, 5.0, 0.1).is_none());
        // Past half the limit it warns when a lake cell moved by the tolerance or more.
        assert!(books_warning(30, true, 0.099, 0.1).is_none());
        for moved in [0.1, 3.5] {
            let finding = books_warning(30, true, moved, 0.1).unwrap();
            let message = &finding.message;
            assert!(message.contains(&format!("{moved:e} m")), "{message}");
            assert!(message.contains("30 iterations"), "{message}");
            assert!(message.contains("tolerance 1e-1 m"), "{message}");
        }
    }

    #[test]
    fn the_tolerance_follows_the_largest_uplift_or_falls_back() {
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-15 * b.abs();
        let rates = Grid::from_vec(1, 3, vec![2e-4, 1e-3, -5e-3]).unwrap();
        assert!(close(tolerance(Some(&rates), 2.5e4), 1e-3 * 1e-3 * 2.5e4));
        for flat in [
            None,
            Some(Grid::filled(1, 3, 0.0)),
            Some(Grid::filled(1, 3, -1e-3)),
        ] {
            assert_eq!(
                tolerance(flat.as_ref(), 2.5e4).to_bits(),
                1e-6_f64.to_bits()
            );
        }
    }
}
