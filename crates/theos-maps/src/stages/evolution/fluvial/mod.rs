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
//!   solution they converge to is unchanged. Each routed depression is a sediment trap
//!   (`fill`): while the iterations run, its cells keep their height and the sediment that
//!   reaches it stays in it up to its capacity, so the cells below see only what passes its
//!   spill. After the iterations one more routing of the step's sediment (`sediment`) fills
//!   each depression in height order up to its spill level, with an ε slope toward its spill
//!   cell, and passes the rest on.
//!
//! The report holds `sediment_out_m3`, the sediment that left the map in the step at the cells
//! without an edge (the drained edges, or the closed map's sink), and `iterations`.

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
use self::sediment::settle;
use self::solve::{Deposition, Incision, deposit, incise};
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
        ];
        if params.g > 0.0 {
            reads.push(LayerSpec::f64("routing_surface"));
            reads.push(LayerSpec::f64("uplift").optional());
        }
        reads
    }

    fn writes(&self, _: &FluvialParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("deposition"),
            LayerSpec::f64("drainage_area"),
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
        let outcome = erode(ctx, params, step.dt_yr, &start)?;
        let mut erosion = ctx.take_f64("erosion")?;
        let mut deposition = ctx.take_f64("deposition")?;
        book(&start, &outcome.height, &mut erosion, &mut deposition);
        ctx.write("height", Layer::F64(outcome.height))?;
        ctx.write("erosion", Layer::F64(erosion))?;
        ctx.write("deposition", Layer::F64(deposition))?;
        ctx.write("drainage_area", Layer::F64(outcome.drainage_area))?;
        Ok(outcome.report)
    }
}

/// One step's result besides the cumulative layers.
struct Outcome {
    /// The new heights.
    height: Grid<f64>,
    /// The plain drainage area (spec §5.10: not weighted by runoff).
    drainage_area: Grid<f64>,
    /// `sediment_out_m3`, `iterations` and a warning when the iterations did not converge.
    report: StageReport,
}

/// One implicit step from `start` (`h*`) over the current weights and stack, with deposition
/// and depression filling when `g > 0`.
fn erode(
    ctx: &StageContext<'_>,
    params: &FluvialParams,
    dt: f64,
    start: &Grid<f64>,
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
    let mut report = StageReport::default();
    let mut step = Incision {
        weights,
        stack,
        held: fixed,
        erodibility: erodibility.as_slice(),
        discharge: discharge.as_slice(),
        m: params.m,
        dt,
    };
    let (mut height, traps) = if params.g > 0.0 {
        let depressions = ctx.value::<Depressions>("depressions")?;
        if let Some(problem) = depressions_misfit(depressions, cells) {
            return Err(ctx.fail(problem));
        }
        let routing = ctx.f64("routing_surface")?.as_slice();
        let traps = Traps::new(depressions, routing, start.as_slice(), cell_area);
        let held: Vec<bool> = (0..cells).map(|i| fixed[i] || traps.holds(i)).collect();
        step.held = &held;
        let dep = Deposition {
            g: params.g,
            traps: &traps,
            cell_area,
            tolerance: tolerance(ctx.f64_opt("uplift")?, dt),
            max_iterations: params.max_iterations,
        };
        let solution = deposit(&step, &dep, start);
        if !solution.converged(dep.tolerance) {
            report.findings.push(
                Finding::warning(format!(
                    "the Gauss–Seidel iterations stopped after {} iterations with the residual \
                     {:e} m, above the tolerance {:e} m",
                    solution.iterations, solution.residual, dep.tolerance
                ))
                .on_layer("height"),
            );
        }
        report
            .values
            .insert("iterations".to_owned(), f64::from(solution.iterations));
        (solution.height, traps)
    } else {
        report.values.insert("iterations".to_owned(), 1.0);
        (incise(&step, start), Traps::default())
    };
    let out = settle(
        stack,
        weights,
        &traps,
        start.as_slice(),
        height.as_mut_slice(),
        cell_area,
    );
    report.values.insert("sediment_out_m3".to_owned(), out);
    Ok(Outcome {
        height,
        drainage_area,
        report,
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
    use super::tolerance;
    use crate::grid::Grid;

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
