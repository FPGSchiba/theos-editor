//! The per-step process `slope_limit`: a thermal step toward the rock's critical slopes (spec
//! §5.9 "Slope limit").
//!
//! The slope of a cell is its largest downhill gradient to the 8 neighbours (height difference
//! over the D8 distance). A pass visits the cells from the highest to the lowest (ties by
//! index); where a cell's slope to its steepest neighbour exceeds the critical slope of its
//! rock, half of the excess height difference moves to that neighbour, which brings the pair
//! exactly to the critical slope. Passes repeat until no slope exceeds its critical slope by
//! more than 10⁻⁶, or 100 passes have run (then a warning). Fixed cells (the drained edges) are
//! sinks: material that moves onto one leaves the map and the cell keeps its height. The
//! report holds `removed_m3`, the volume that left, and `passes`.

use std::f64::consts::SQRT_2;

use serde::Deserialize;

use crate::grid::Grid;
use crate::kernels::num::{key, max};
use crate::stage::{
    Finding, LayerSpec, MapKind, StageContext, StageError, StageReport, TypedStage,
};
use crate::world::Layer;

/// The most passes of one step.
const MAX_PASSES: u32 = 100;

/// A slope may exceed its critical slope by this much when the passes stop.
const TOLERANCE: f64 = 1e-6;

/// The eight neighbours as `(Δrow, Δcol)`, with the distance in cells.
const NEIGHBOURS: [((isize, isize), f64); 8] = [
    ((-1, 0), 1.0),
    ((0, -1), 1.0),
    ((0, 1), 1.0),
    ((1, 0), 1.0),
    ((-1, -1), SQRT_2),
    ((-1, 1), SQRT_2),
    ((1, -1), SQRT_2),
    ((1, 1), SQRT_2),
];

/// The parameters of `slope_limit`: none; the limits are the layer `critical_slope`.
#[derive(Debug, Default, Deserialize)]
pub struct SlopeLimitParams {}

/// The process `slope_limit`.
pub struct SlopeLimit;

impl TypedStage for SlopeLimit {
    type Params = SlopeLimitParams;
    const ID: &'static str = "slope_limit";
    const SUMMARY: &'static str = "A thermal step toward the rock's critical slopes: half of the excess moves to the steepest neighbour, highest cells first, until stable or 100 passes; drained edges are sinks.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &SlopeLimitParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("critical_slope"),
            LayerSpec::mask("fixed"),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("deposition"),
        ]
    }

    fn writes(&self, _: &SlopeLimitParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("deposition"),
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
        let mut height = ctx.take_f64("height")?;
        let mut erosion = ctx.take_f64("erosion")?;
        let mut deposition = ctx.take_f64("deposition")?;
        let limits = Limits {
            critical: ctx.f64("critical_slope")?.as_slice(),
            fixed: ctx.mask("fixed")?.as_slice(),
            cell_m: frame.cell_m,
        };
        let mut books = Books {
            erosion: &mut erosion,
            deposition: &mut deposition,
            removed_m: 0.0,
        };
        let (passes, stable) = relax(&limits, &mut height, &mut books);
        let removed_m3 = books.removed_m * frame.cell_area_m2();
        let mut report = StageReport::default();
        report.values.insert("removed_m3".to_owned(), removed_m3);
        report.values.insert("passes".to_owned(), f64::from(passes));
        if !stable {
            report.findings.push(
                Finding::warning(format!(
                    "{MAX_PASSES} passes did not bring every slope below its critical slope"
                ))
                .on_layer("height"),
            );
        }
        ctx.write("height", Layer::F64(height))?;
        ctx.write("erosion", Layer::F64(erosion))?;
        ctx.write("deposition", Layer::F64(deposition))?;
        Ok(report)
    }
}

/// The limits the passes work to.
struct Limits<'a> {
    critical: &'a [f64],
    fixed: &'a [bool],
    cell_m: f64,
}

/// The cumulative layers the passes book into, and the thickness that left the map, summed
/// over cells.
struct Books<'a> {
    erosion: &'a mut Grid<f64>,
    deposition: &'a mut Grid<f64>,
    removed_m: f64,
}

/// The steepest downhill neighbour of cell `i`.
struct Descent {
    /// The neighbour's index.
    to: usize,
    /// The height difference to it.
    drop: f64,
    /// The distance to it, m.
    dist: f64,
}

impl Limits<'_> {
    /// The steepest downhill neighbour of `i`, or `None` on flat or rising ground.
    fn steepest(&self, height: &Grid<f64>, i: usize) -> Option<Descent> {
        let h = height.as_slice();
        let mut best: Option<(f64, Descent)> = None;
        for (offset, cells) in NEIGHBOURS {
            let Some(j) = height.neighbour(i, offset) else {
                continue;
            };
            let (drop, dist) = (h[i] - h[j], cells * self.cell_m);
            let slope = drop / dist;
            if drop > 0.0 && best.as_ref().is_none_or(|(s, _)| slope > *s) {
                best = Some((slope, Descent { to: j, drop, dist }));
            }
        }
        best.map(|(_, descent)| descent)
    }

    /// How far the cell `i` exceeds its critical slope by (negative or 0 when it does not).
    fn excess(&self, height: &Grid<f64>, i: usize) -> f64 {
        self.steepest(height, i)
            .map_or(0.0, |d| d.drop / d.dist - self.critical[i])
    }

    /// Whether any cell that is not fixed exceeds its critical slope by more than [`TOLERANCE`].
    fn violated(&self, height: &Grid<f64>) -> bool {
        (0..height.len()).any(|i| !self.fixed[i] && self.excess(height, i) > TOLERANCE)
    }
}

/// Runs passes over `height` until it is stable or [`MAX_PASSES`] passes have run. Returns the
/// number of passes and whether the surface is stable.
fn relax(limits: &Limits<'_>, height: &mut Grid<f64>, books: &mut Books<'_>) -> (u32, bool) {
    let mut passes = 0;
    while limits.violated(height) {
        if passes == MAX_PASSES {
            return (passes, false);
        }
        pass(limits, height, books);
        passes += 1;
    }
    (passes, true)
}

/// One pass over the cells, highest first, ties by index.
fn pass(limits: &Limits<'_>, height: &mut Grid<f64>, books: &mut Books<'_>) {
    let mut order: Vec<usize> = (0..height.len()).collect();
    let h = height.as_slice();
    order.sort_by(|&a, &b| key(h[b]).total_cmp(&key(h[a])).then_with(|| a.cmp(&b)));
    for i in order {
        if limits.fixed[i] {
            continue;
        }
        let Some(descent) = limits.steepest(height, i) else {
            continue;
        };
        let allowed = limits.critical[i] * descent.dist;
        let moved = 0.5 * max(descent.drop - allowed, 0.0);
        if moved <= 0.0 {
            continue;
        }
        let h = height.as_mut_slice();
        h[i] -= moved;
        books.erosion.as_mut_slice()[i] += moved;
        if limits.fixed[descent.to] {
            books.removed_m += moved;
        } else {
            h[descent.to] += moved;
            books.deposition.as_mut_slice()[descent.to] += moved;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::{Limits, SlopeLimit};
    use crate::grid::Grid;
    use crate::rng::StageRng;
    use crate::stage::{LayerSpec, Severity, Stage, StageError};
    use crate::stages::evolution::PlannedProcess;
    use crate::stages::evolution::test_support::{evolve, processes};
    use crate::stages::terrain::support::run_stage;
    use crate::world::{Layer, MapFrame, World};

    const N: usize = 64;

    /// Random terrain of `relief` metres with critical slopes between 0.45 and 0.84; the first
    /// column is fixed at 0 m.
    fn steep_world(relief: f64, seed: u64) -> World {
        let mut rng = StageRng::for_key(seed, "steep");
        let mut w = World::new(MapFrame {
            rows: N,
            cols: N,
            cell_m: 100.0,
        });
        let fixed: Vec<bool> = (0..N * N).map(|i| i % N == 0).collect();
        let height: Vec<f64> = fixed
            .iter()
            .map(|&f| if f { 0.0 } else { rng.range(0.0, relief) })
            .collect();
        let critical: Vec<f64> = (0..N * N).map(|_| rng.range(0.45, 0.84)).collect();
        let layer = |v: Vec<f64>| Layer::F64(Grid::from_vec(N, N, v).unwrap());
        w.insert("height", layer(height)).unwrap();
        w.insert("critical_slope", layer(critical)).unwrap();
        w.insert("fixed", Layer::Mask(Grid::from_vec(N, N, fixed).unwrap()))
            .unwrap();
        w.insert("erosion", layer(vec![0.0; N * N])).unwrap();
        w.insert("deposition", layer(vec![0.0; N * N])).unwrap();
        w
    }

    fn slope_limit() -> Vec<PlannedProcess> {
        processes("[[process]]\nid = \"slope_limit\"")
    }

    /// The largest excess of a non-fixed cell's D8 slope over its critical slope.
    fn worst_excess(w: &World) -> f64 {
        let limits = Limits {
            critical: w.f64("critical_slope").unwrap().as_slice(),
            fixed: w.mask("fixed").unwrap().as_slice(),
            cell_m: 100.0,
        };
        let h = w.f64("height").unwrap();
        (0..N * N)
            .filter(|&i| !limits.fixed[i])
            .map(|i| limits.excess(h, i))
            .fold(f64::NEG_INFINITY, |m, x| match x.total_cmp(&m) {
                Ordering::Greater => x,
                _ => m,
            })
    }

    fn sum(w: &World, layer: &str) -> f64 {
        w.f64(layer).unwrap().as_slice().iter().sum()
    }

    // Physics test 10.
    #[test]
    fn slope_limit_leaves_no_slope_above_critical() {
        let mut w = steep_world(150.0, 5);
        assert!(worst_excess(&w) > 0.5, "the terrain starts steep");
        let before = sum(&w, "height");
        let report = evolve(&mut w, &slope_limit(), 1, 1e4, 1).unwrap();
        assert!(report.findings.is_empty(), "{report:?}");
        assert!(worst_excess(&w) <= 1e-6, "excess {}", worst_excess(&w));
        let removed = report.values["slope_limit_removed_m3"];
        assert!(removed > 0.0);
        let change = (sum(&w, "height") - before) * 1.0e4;
        assert!(
            ((change + removed) / removed).abs() < 1e-9,
            "{change} vs {removed}"
        );
        let booked = sum(&w, "deposition") - sum(&w, "erosion");
        assert!(((booked * 1.0e4 + removed) / removed).abs() < 1e-9);
    }

    #[test]
    fn a_stable_surface_is_left_alone() {
        let mut w = steep_world(30.0, 8);
        let before = w.f64("height").unwrap().clone();
        assert!(worst_excess(&w) <= 0.0);
        let report = evolve(&mut w, &slope_limit(), 1, 1e4, 1).unwrap();
        assert_eq!(w.f64("height").unwrap(), &before);
        assert!(report.values["slope_limit_removed_m3"].abs() < f64::MIN_POSITIVE);
        assert!(report.values["slope_limit_passes"].abs() < f64::MIN_POSITIVE);
    }

    #[test]
    fn fixed_cells_keep_their_height_and_a_cliff_next_to_one_drains() {
        let mut w = steep_world(30.0, 9);
        let mut height = w.f64("height").unwrap().clone();
        height.set(10, 1, 400.0);
        w.insert("height", Layer::F64(height)).unwrap();
        let report = evolve(&mut w, &slope_limit(), 1, 1e4, 1).unwrap();
        assert!(report.values["slope_limit_removed_m3"] > 0.0);
        let h = w.f64("height").unwrap();
        assert!((0..N).all(|r| h.get(r, 0).to_bits() == 0.0_f64.to_bits()));
        let deposition = w.f64("deposition").unwrap();
        assert!((0..N).all(|r| deposition.get(r, 0).to_bits() == 0.0_f64.to_bits()));
    }

    #[test]
    fn the_result_does_not_depend_on_the_run() {
        let run = || {
            let mut w = steep_world(150.0, 5);
            evolve(&mut w, &slope_limit(), 1, 1e4, 1).unwrap();
            w.f64("height").unwrap().clone()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn a_surface_that_cannot_settle_in_100_passes_warns() {
        let mut w = steep_world(1.0e7, 3);
        let report = evolve(&mut w, &slope_limit(), 1, 1e4, 1).unwrap();
        assert_eq!(report.findings.len(), 1, "{report:?}");
        assert_eq!(report.findings[0].severity, Severity::Warning);
        assert!((report.values["slope_limit_passes"] - 100.0).abs() < 1e-12);
    }

    #[test]
    fn slope_limit_declares_its_layers_and_runs_only_in_the_evolution() {
        let stage: &dyn Stage = &SlopeLimit;
        let params = stage.params(&toml::Table::new()).unwrap();
        let names = |l: Vec<LayerSpec>| l.iter().map(|s| s.name).collect::<Vec<_>>();
        assert_eq!(
            names(stage.reads(params.as_ref())),
            ["height", "critical_slope", "fixed", "erosion", "deposition"]
        );
        assert_eq!(
            names(stage.writes(params.as_ref())),
            ["height", "erosion", "deposition"]
        );
        assert!(stage.params(&"limit = 1.0".parse().unwrap()).is_err());
        let mut w = steep_world(10.0, 1);
        let err = run_stage(&mut w, &SlopeLimit, "").unwrap_err();
        assert!(matches!(&err, StageError::Failed { stage, .. } if stage == "slope_limit"));
    }
}
