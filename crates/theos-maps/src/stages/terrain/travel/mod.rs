//! The `travel` stage: the walking time in minutes from every cell to the nearest Chime, ported
//! from 0.3.0's `features.py::travel`.
//!
//! **Graph:** the 16 directions of [`crate::grid::D16`]. A step of horizontal length `L` and
//! signed slope `s = Δh / L` is walked at Tobler's speed `6·exp(−3.5·|s + 0.05|)` km/h (1993),
//! peaking on a gentle downhill of −5 %.
//!
//! **Roads:** a step between two cells that a road passes goes at
//! `max(speed, road_min_speed_ms) × road_speedup` (0.3.0: `max(speed, 1.2 m/s) × 1.3`); every
//! other step at `speed × off_path_factor` (spec §5.9: 0.6; 0.3.0 had none). A bridge of the
//! network is an edge between its banks at the road speed on the level, with no ford penalty.
//!
//! **Water:** each river or big-river cell a step passes after its first cell (those between its
//! ends, then its end; the crossed-cell set of `roads`) costs `ford_penalty_s` (120 s). 0.3.0
//! charged every step with an end in a big river that was not on a road, because its roads
//! bridged every river; here roads ford streams, so the fee is also paid on roads, and only
//! bridges avoid it. Every step that passes or ends in a lake cell costs `lake_penalty_s`
//! (900 s), as 0.3.0's `lake[a] | lake[b]` with the crossed cells added, so wading across `n`
//! lake cells costs `n + 1` penalties.
//!
//! **Outputs:** `reach_min`, a raster of minutes from the nearest Chime (multi-source Dijkstra),
//! −1 where a cell cannot be reached and everywhere when the map has no Chime (spec ruling 36:
//! layers are finite). The metrics are `chime_to_chime_min`, the walk from the first Chime to the
//! second by handle (only with two or more Chimes, and omitted should the second be unreachable,
//! which finite step costs rule out: a guard), and `floor_pct_within_15_min`, the percent
//! of cells with `0 ≤ reach_min ≤ 15`.

#[cfg(test)]
mod tests;
mod walk;

/// The metric of the walk from the first Chime to the second, in minutes; the summary also
/// holds it as the parity path [`crate::metrics::CHIME_WALK`].
pub(crate) const CHIME_TO_CHIME_MIN: &str = "chime_to_chime_min";

/// The metric of the share of cells within 15 minutes of a Chime, percent.
const FLOOR_PCT_WITHIN_15_MIN: &str = "floor_pct_within_15_min";

use serde::Deserialize;

use self::walk::Walk;
use crate::features::FeatureSet;
use crate::grid::Grid;
use crate::kernels::KernelError;
use crate::kernels::num::to_f64;
use crate::stage::{
    Check, LayerSpec, MapKind, Metrics, ParamError, StageContext, StageError, StageReport,
    TypedStage,
};
use crate::stages::terrain::roads::RoadNetwork;
use crate::world::{Layer, ValueType, World};

/// Tobler's speed on flat ground at the peak, km/h.
const PEAK_KMH: f64 = 6.0;
/// The steepness of the fall-off, per unit of slope.
const FALL_OFF: f64 = 3.5;
/// The slope of the peak: a downhill of 5 %.
const PEAK_SLOPE: f64 = -0.05;
/// Kilometres per hour in one metre per second.
const KMH_PER_MS: f64 = 3.6;
/// The minutes a cell may be from a Chime to count for the floor share.
const NEAR_MIN: f64 = 15.0;

/// Tobler's hiking speed in m/s on a step of signed slope `slope` (rise over run).
#[must_use]
pub fn tobler_ms(slope: f64) -> f64 {
    PEAK_KMH * libm::exp(-FALL_OFF * (slope - PEAK_SLOPE).abs()) / KMH_PER_MS
}

/// The parameters of `travel`; the defaults are 0.3.0's, but the off-path factor (spec §5.9).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct TravelParams {
    /// Seconds for each stream or big-river cell a step passes.
    pub ford_penalty_s: f64,
    /// Seconds for each step that passes or ends in a lake cell.
    pub lake_penalty_s: f64,
    /// The factor on the speed on a road.
    pub road_speedup: f64,
    /// The factor on the speed off roads.
    pub off_path_factor: f64,
    /// The least speed before the road speed-up, m/s.
    pub road_min_speed_ms: f64,
}

impl Default for TravelParams {
    fn default() -> Self {
        Self {
            ford_penalty_s: 120.0,
            lake_penalty_s: 900.0,
            road_speedup: 1.3,
            off_path_factor: 0.6,
            road_min_speed_ms: 1.2,
        }
    }
}

/// The stage `travel`.
pub struct Travel;

impl TypedStage for Travel {
    type Params = TravelParams;
    const ID: &'static str = "travel";
    const SUMMARY: &'static str =
        "Walking minutes from the nearest Chime by Tobler's hiking function, faster on roads.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &TravelParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::mask("river"),
            LayerSpec::mask("big_river"),
            LayerSpec::mask("lake"),
            LayerSpec::value("road_network", RoadNetwork::NAME).optional(),
            LayerSpec::value("features", FeatureSet::NAME).optional(),
        ]
    }

    fn writes(&self, _: &TravelParams) -> Vec<LayerSpec> {
        vec![LayerSpec::f64("reach_min")]
    }

    fn metric_names(&self, _: &TravelParams) -> Vec<&'static str> {
        vec![CHIME_TO_CHIME_MIN, FLOOR_PCT_WITHIN_15_MIN]
    }

    fn validate(&self, params: &mut TravelParams) -> Result<(), ParamError> {
        let check = Check::of(Self::ID);
        for (name, v) in [
            ("ford_penalty_s", params.ford_penalty_s),
            ("lake_penalty_s", params.lake_penalty_s),
            ("road_min_speed_ms", params.road_min_speed_ms),
        ] {
            check.at_least(name, v, 0.0)?;
        }
        check.positive("road_speedup", params.road_speedup)?;
        check.positive("off_path_factor", params.off_path_factor)
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &TravelParams,
    ) -> Result<StageReport, StageError> {
        let frame = *ctx.frame();
        let chimes: Vec<usize> = ctx
            .value_opt::<FeatureSet>("features")?
            .map_or_else(Vec::new, |set| set.points("poi", Some("chime")))
            .into_iter()
            .map(|at| frame.index_at_m(at))
            .collect();
        let mut reach = Grid::filled(frame.rows, frame.cols, -1.0);
        let mut report = StageReport::default();
        if !chimes.is_empty() {
            let cancel = || ctx.is_cancelled();
            let walk = Walk::new(
                ctx.f64("height")?,
                [ctx.mask("river")?, ctx.mask("big_river")?],
                ctx.mask("lake")?,
                ctx.value_opt::<RoadNetwork>("road_network")?,
                params,
                frame,
            );
            let failed = |e: KernelError| ctx.kernel_failed(e);
            let tree = walk.tree(&chimes, &cancel).map_err(failed)?;
            for (out, &d) in reach.as_mut_slice().iter_mut().zip(&tree.dist) {
                if d.is_finite() {
                    *out = d / 60.0;
                }
            }
            if let [first, second, ..] = chimes[..] {
                let from_first = walk.tree(&[first], &cancel).map_err(failed)?;
                let seconds = from_first.dist[second];
                if seconds.is_finite() {
                    report
                        .values
                        .insert(CHIME_TO_CHIME_MIN.to_owned(), seconds / 60.0);
                }
            }
        }
        let near = reach
            .as_slice()
            .iter()
            .filter(|&&v| (0.0..=NEAR_MIN).contains(&v))
            .count();
        report.values.insert(
            FLOOR_PCT_WITHIN_15_MIN.to_owned(),
            100.0 * to_f64(near) / to_f64(reach.len()),
        );
        ctx.write("reach_min", Layer::F64(reach))?;
        Ok(report)
    }

    fn metrics(&self, _: &World, _: &TravelParams, report: &StageReport) -> Metrics {
        Metrics(report.values.clone())
    }
}
