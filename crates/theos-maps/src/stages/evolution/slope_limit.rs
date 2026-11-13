//! The per-step process `slope_limit`: an exact threshold limiter toward the rock's critical
//! slopes, whose debris feeds the rivers (spec §5.9 "Slope limit", ruling 51; threshold slopes
//! after Montgomery & Brandon 2002).
//!
//! A cell may stand at most its rock's critical slope × the D8 distance above each neighbour.
//! One sweep ([`sweep`]) lowers every cell to the highest surface at or below `height` that keeps
//! this rule, so the limiter settles exactly in every step; drained (fixed) cells keep their
//! height. What a cell loses is booked as `erosion` and added to `slope_supply`, the thickness
//! that the next `fluvial` takes into the sediment flux at that cell (spec §5.9 "Slope supply"): a
//! failing slope sends its debris into the drainage network, as landslides feed rivers. No
//! material moves to a neighbour, so the limiter never raises a cell. `slope_supply` adds up over
//! the runs until `fluvial` empties it, so nothing is lost or counted twice when the two run at
//! different `every` (plan ruling L27). The report holds `removed_m3`, the volume removed.

mod sweep;
#[cfg(test)]
mod tests;

use serde::Deserialize;

use self::sweep::limited;
use crate::grid::Grid;
use crate::stage::{LayerSpec, MapKind, StageContext, StageError, StageReport, TypedStage};
use crate::world::Layer;

/// The parameters of `slope_limit`: none; the limits are the layer `critical_slope`.
#[derive(Debug, Default, Deserialize)]
pub struct SlopeLimitParams {}

/// The process `slope_limit`.
pub struct SlopeLimit;

impl TypedStage for SlopeLimit {
    type Params = SlopeLimitParams;
    const ID: &'static str = "slope_limit";
    const SUMMARY: &'static str = "An exact threshold limiter in one sweep: every cell falls to the highest surface that keeps its rock's critical slope to each neighbour; the material removed is booked as erosion and goes to the rivers as slope supply; drained edges keep their height.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &SlopeLimitParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("critical_slope"),
            LayerSpec::mask("fixed"),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("slope_supply").optional(),
        ]
    }

    fn writes(&self, _: &SlopeLimitParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("slope_supply"),
        ]
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        _: &SlopeLimitParams,
    ) -> Result<StageReport, StageError> {
        if ctx.step().is_none() {
            return Err(ctx.fail("runs only as a process of landscape_evolution"));
        }
        let frame = *ctx.frame();
        let critical = ctx.f64("critical_slope")?.as_slice();
        let fixed = ctx.mask("fixed")?.as_slice();
        if let Some(problem) = unusable_slope(critical, fixed) {
            return Err(ctx.fail(problem));
        }
        let lower = limited(ctx.f64("height")?, critical, fixed, frame.cell_m);
        let height = ctx.take_f64("height")?;
        let mut erosion = ctx.take_f64("erosion")?;
        let mut supply = if ctx.f64_opt("slope_supply")?.is_some() {
            ctx.take_f64("slope_supply")?
        } else {
            Grid::filled(frame.rows, frame.cols, 0.0)
        };
        let mut removed_m = 0.0;
        let books = erosion.as_mut_slice().iter_mut().zip(supply.as_mut_slice());
        for ((&before, &after), (eroded, handed)) in
            height.as_slice().iter().zip(lower.as_slice()).zip(books)
        {
            let cut = before - after;
            if cut > 0.0 {
                *eroded += cut;
                *handed += cut;
                removed_m += cut;
            }
        }
        let mut report = StageReport::default();
        report
            .values
            .insert("removed_m3".to_owned(), removed_m * frame.cell_area_m2());
        ctx.write("height", Layer::F64(lower))?;
        ctx.write("erosion", Layer::F64(erosion))?;
        ctx.write("slope_supply", Layer::F64(supply))?;
        Ok(report)
    }
}

/// Why the critical slopes cannot limit the surface, or `None` when they can: every cell that is
/// not fixed needs a finite critical slope that is not negative, or the rule has no highest
/// surface below the heights.
fn unusable_slope(critical: &[f64], fixed: &[bool]) -> Option<String> {
    critical
        .iter()
        .zip(fixed)
        .enumerate()
        .find(|&(_, (&s, &f))| !f && !usable(s))
        .map(|(i, (s, _))| {
            format!("cell {i} has the critical slope {s}; it must be finite and not negative")
        })
}

/// Whether `slope` can be a critical slope: finite and not negative.
fn usable(slope: f64) -> bool {
    slope.is_finite() && slope >= 0.0
}
