//! The per-step process `fluvial`: stream-power incision with n = 1 over the multiple-flow
//! weights, solved implicitly in stack order (spec §5.9 "The fluvial law", rulings 3, 28, 29).
//!
//! With `g = 0` the law is detachment-limited stream power, `dh/dt = U − K·Q^m·S` (Braun &
//! Willett 2013), in `FastScape`'s multiple-flow form: `S` is the flow-weighted slope
//! `Σ_r w_r·(h − h_r) / d_r` over the cell's weighted receivers. `Q` is the runoff-weighted
//! drainage area, the upstream sum of `p̂ × cell area` with `p̂` the local runoff over the
//! map-mean runoff ([`runoff_weight`]). Deposition (`g > 0`) comes with the erosion–deposition
//! law of Yuan et al. 2019.

mod solve;
#[cfg(test)]
mod tests;

use serde::Deserialize;

use self::solve::{Incision, incise};
use super::flow::{FlowWeights, accumulate, cell_index};
use crate::grid::Grid;
use crate::kernels::num::{max, to_f64};
use crate::stage::{
    LayerSpec, MapKind, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::evolution::resolve_sinks::Depressions;
use crate::stages::terrain::climate::Climate;
use crate::world::{Layer, MapFrame, ValueType};

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
    const SUMMARY: &'static str = "Stream-power incision with n = 1 over the multiple-flow weights, solved implicitly in stack order; runoff-weighted drainage area; cumulative erosion and deposition.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &FluvialParams) -> Vec<LayerSpec> {
        vec![
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
        ]
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
        if params.g > 0.0 {
            return Err(ctx.fail("deposition (g > 0) is not implemented yet"));
        }
        let start = ctx.take_f64("height")?;
        let (height, drainage_area) = erode(ctx, params.m, step.dt_yr, &start)?;
        let mut erosion = ctx.take_f64("erosion")?;
        let mut deposition = ctx.take_f64("deposition")?;
        book(&start, &height, &mut erosion, &mut deposition);
        ctx.write("height", Layer::F64(height))?;
        ctx.write("erosion", Layer::F64(erosion))?;
        ctx.write("deposition", Layer::F64(deposition))?;
        ctx.write("drainage_area", Layer::F64(drainage_area))?;
        Ok(StageReport::default())
    }
}

/// One implicit step from `start` (`h*`) over the current weights and stack: the new heights and
/// the plain drainage area (spec §5.10: not weighted by runoff).
fn erode(
    ctx: &StageContext<'_>,
    m: f64,
    dt: f64,
    start: &Grid<f64>,
) -> Result<(Grid<f64>, Grid<f64>), StageError> {
    let frame = *ctx.frame();
    let (weights, stack) = (ctx.value::<FlowWeights>("weights")?, ctx.u32("stack")?);
    if let Some(problem) = misfit(weights, stack.as_slice(), frame.cells()) {
        return Err(ctx.fail(problem));
    }
    let p_hat = runoff_weight(ctx.value::<Climate>("climate")?, &frame);
    if p_hat.len() != frame.cells() {
        return Err(ctx.fail(format!(
            "the climate's runoff has {} cells, but the grid has {}",
            p_hat.len(),
            frame.cells()
        )));
    }
    let cell_area = frame.cell_area_m2();
    let discharge = accumulate(stack.as_slice(), weights, &p_hat.map(|p| p * cell_area));
    let area = Grid::filled(frame.rows, frame.cols, cell_area);
    let drainage_area = accumulate(stack.as_slice(), weights, &area);
    let step = Incision {
        weights,
        stack: stack.as_slice(),
        fixed: ctx.mask("fixed")?.as_slice(),
        erodibility: ctx.f64("erodibility")?.as_slice(),
        discharge: discharge.as_slice(),
        m,
        dt,
    };
    Ok((incise(&step, start), drainage_area))
}

/// Why `weights` and `stack` do not fit a grid of `cells` cells, or `None` when they do.
fn misfit(weights: &FlowWeights, stack: &[u32], cells: usize) -> Option<String> {
    if weights.cells() != cells {
        return Some(format!(
            "the flow weights cover {} cells, but the grid has {cells}",
            weights.cells()
        ));
    }
    let edges = weights.to.len();
    let broken = weights.weight.len() != edges
        || weights.dist.len() != edges
        || weights.start.iter().any(|&s| cell_index(s) > edges);
    let outside = |&c: &u32| cell_index(c) >= cells;
    if broken || weights.to.iter().any(outside) || stack.iter().any(outside) {
        return Some("the flow weights or the stack name a cell outside the grid".to_owned());
    }
    None
}

/// Adds the thickness each cell lost from `start` to `end` to `erosion` and the thickness it
/// gained to `deposition`.
fn book(start: &Grid<f64>, end: &Grid<f64>, erosion: &mut Grid<f64>, deposition: &mut Grid<f64>) {
    let cells = start.as_slice().iter().zip(end.as_slice());
    let records = erosion
        .as_mut_slice()
        .iter_mut()
        .zip(deposition.as_mut_slice());
    for ((&before, &after), (eroded, deposited)) in cells.zip(records) {
        *eroded += max(before - after, 0.0);
        *deposited += max(after - before, 0.0);
    }
}

/// The runoff weight `p̂ = runoff / mean(runoff)` of every cell (spec ruling 28): all 1 while
/// the climate has no fields yet, and all 0 when the mean runoff is 0 (no water, no stream).
#[must_use]
pub fn runoff_weight(climate: &Climate, frame: &MapFrame) -> Grid<f64> {
    let Some(fields) = &climate.fields else {
        return Grid::filled(frame.rows, frame.cols, 1.0);
    };
    let runoff = &fields.runoff;
    let total: f64 = runoff.as_slice().iter().sum();
    let mean = total / to_f64(runoff.len());
    if mean > 0.0 {
        runoff.map(|r| r / mean)
    } else {
        runoff.map(|_| 0.0)
    }
}
