//! The `detuning` stage: the detuning field omega, ported from 0.3.0's `omega_detuning.py`.
//!
//! `omega = clip(baseline + sum over ruins of strength exp(-(d / R)^2) + drift_amplitude n, 0, 1)`
//! where `d` is the distance of the cell's centre to the ruin in metres, `R` is `ruin_radius_km`
//! in metres and `n` is spectral noise with `beta = 3.4`, zero mean and unit deviation, drawn
//! from the stream `drift`. The ruins are the `poi` features of type `ruin`; without any, omega
//! is the baseline with the drift.
//!
//! The lengths of 0.3.0 (cells of 62.5 m) are metres here: 0.3.0 measured `d` in cells and
//! divided by `ruin_radius_km` times the cells per km, which is the distance in metres over the
//! radius in metres. 0.3.0's drift amplitude 0.12 is the parameter `drift_amplitude`.
//!
//! The metrics are `mean`, `p95` and `pct_above_0_5` of omega, as in 0.3.0's summary.

use serde::Deserialize;

use crate::features::FeatureSet;
use crate::grid::Grid;
use crate::kernels::noise::spectral_noise;
use crate::kernels::num::{max, min, to_f64};
use crate::kernels::stats::percentile;
use crate::stage::{
    LayerSpec, MapKind, Metrics, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::world::{Layer, ValueType, World};

/// The spectral exponent of the drift, 0.3.0's 3.4.
const DRIFT_BETA: f64 = 3.4;

/// The parameters of `detuning`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DetuningParams {
    /// Omega far from ruins, before the drift, in `[0, 1]`.
    pub baseline: f64,
    /// The radius of a ruin's hot spot, km (the distance at which it falls to 1/e).
    pub ruin_radius_km: f64,
    /// The omega a ruin adds at its own cell.
    pub ruin_strength: f64,
    /// The amplitude of the slow drift noise.
    pub drift_amplitude: f64,
}

impl Default for DetuningParams {
    fn default() -> Self {
        Self {
            baseline: 0.12,
            ruin_radius_km: 2.5,
            ruin_strength: 0.6,
            drift_amplitude: 0.12,
        }
    }
}

/// Omega of every cell: baseline, ruin hot spots and drift, clipped to `[0, 1]`.
fn omega(
    rows: usize,
    cols: usize,
    centre: impl Fn(usize, usize) -> [f64; 2],
    ruins: &[[f64; 2]],
    drift: &Grid<f64>,
    params: &DetuningParams,
) -> Grid<f64> {
    let radius_m = params.ruin_radius_km * 1000.0;
    let mut out = Grid::filled(rows, cols, 0.0);
    for r in 0..rows {
        for c in 0..cols {
            let [x, y] = centre(r, c);
            let mut v = params.baseline;
            for ruin in ruins {
                let d = libm::hypot(x - ruin[0], y - ruin[1]) / radius_m;
                v += params.ruin_strength * libm::exp(-d * d);
            }
            v += params.drift_amplitude * drift.as_slice()[r * cols + c];
            out.as_mut_slice()[r * cols + c] = min(max(v, 0.0), 1.0);
        }
    }
    out
}

/// The stage `detuning`.
pub struct Detuning;

impl TypedStage for Detuning {
    type Params = DetuningParams;
    const ID: &'static str = "detuning";
    const SUMMARY: &'static str =
        "The detuning field omega: a baseline, hot spots around ruins and a slow drift.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &DetuningParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::value("features", FeatureSet::NAME).optional(),
        ]
    }

    fn writes(&self, _: &DetuningParams) -> Vec<LayerSpec> {
        vec![LayerSpec::f64("omega")]
    }

    fn metric_names(&self, _: &DetuningParams) -> Vec<&'static str> {
        vec!["mean", "p95", "pct_above_0_5"]
    }

    fn validate(&self, params: &mut DetuningParams) -> Result<(), ParamError> {
        let at = |name: &str, message: &str| ParamError::at(Self::ID, name, message);
        if !(params.baseline.is_finite() && (0.0..=1.0).contains(&params.baseline)) {
            return Err(at("baseline", "must be in [0, 1]"));
        }
        if !(params.ruin_radius_km.is_finite() && params.ruin_radius_km > 0.0) {
            return Err(at("ruin_radius_km", "must be positive and finite"));
        }
        if !(params.ruin_strength.is_finite() && params.ruin_strength >= 0.0) {
            return Err(at("ruin_strength", "must be at least 0 and finite"));
        }
        if !(params.drift_amplitude.is_finite() && params.drift_amplitude >= 0.0) {
            return Err(at("drift_amplitude", "must be at least 0 and finite"));
        }
        Ok(())
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &DetuningParams,
    ) -> Result<StageReport, StageError> {
        let frame = *ctx.frame();
        let mut rng = ctx.sub_rng("drift");
        ctx.f64("height")?;
        let ruins = ctx
            .value_opt::<FeatureSet>("features")?
            .map_or_else(Vec::new, |set| set.points("poi", Some("ruin")));
        let drift = spectral_noise(frame.rows, frame.cols, &mut rng, DRIFT_BETA, 0.0, None);
        let field = omega(
            frame.rows,
            frame.cols,
            |r, c| frame.centre_m(r, c),
            &ruins,
            &drift,
            params,
        );
        let mut report = StageReport::default();
        let values = field.as_slice();
        let n = to_f64(values.len());
        let above = to_f64(values.iter().filter(|&&v| v > 0.5).count());
        let p95 = percentile(values, 95.0).map_err(|e| ctx.fail(e.to_string()))?;
        report
            .values
            .insert("mean".to_owned(), values.iter().sum::<f64>() / n);
        report.values.insert("p95".to_owned(), p95);
        report
            .values
            .insert("pct_above_0_5".to_owned(), above / n * 100.0);
        ctx.write("omega", Layer::F64(field))?;
        Ok(report)
    }

    fn metrics(&self, _: &World, _: &DetuningParams, report: &StageReport) -> Metrics {
        Metrics(report.values.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::features::{Feature, FeatureSet, Geometry};
    use crate::grid::Grid;
    use crate::stage::{Registry, Stage};
    use crate::stages::terrain::detuning::{Detuning, DetuningParams};
    use crate::stages::terrain::support::{run_stage, table, world};
    use crate::world::{Layer, World};
    use std::collections::BTreeMap;

    type TestResult = Result<(), Box<dyn std::error::Error>>;
    const N: usize = 64;
    /// The centre of cell (32, 32) at 100 m cells.
    const RUIN: [f64; 2] = [3250.0, 3250.0];

    /// A flat 64 x 64 world, with one ruin at the centre when `ruin` is set.
    fn setup(ruin: bool) -> Result<World, Box<dyn std::error::Error>> {
        let mut w = world(N, N);
        w.insert("height", Layer::F64(Grid::filled(N, N, 100.0)))?;
        if ruin {
            let mut set = FeatureSet::default();
            set.add(Feature {
                kind: "poi",
                ty: "ruin".into(),
                geometry: Geometry::Point(RUIN),
                props: BTreeMap::new(),
            });
            w.insert("features", Layer::Value(Box::new(set)))?;
        }
        Ok(w)
    }

    fn omega_of(w: &World) -> Result<&Grid<f64>, Box<dyn std::error::Error>> {
        Ok(w.f64("omega").ok_or("the stage wrote omega")?)
    }

    fn run_detuning_on_test_world() -> Result<Grid<f64>, Box<dyn std::error::Error>> {
        let mut w = setup(true)?;
        run_stage(&mut w, &Detuning, "")?;
        Ok(omega_of(&w)?.clone())
    }

    // AC: CE-28.7
    #[test]
    fn detuning_is_a_registered_stage_that_writes_omega() -> TestResult {
        assert!(Registry::builtin().get("detuning").is_some());
        let omega = run_detuning_on_test_world()?;
        assert!(omega.as_slice().iter().all(|v| (0.0..=1.0).contains(v)));
        assert!(omega.get(32, 32) > omega.get(2, 2));
        Ok(())
    }

    /// Without drift and with a radius of 0.5 km, the cells far from the ruin hold the baseline
    /// exactly and the ruin's own cell holds baseline plus strength.
    #[test]
    fn the_baseline_holds_far_from_ruins() -> TestResult {
        let mut w = setup(true)?;
        run_stage(
            &mut w,
            &Detuning,
            "ruin_radius_km = 0.5\ndrift_amplitude = 0.0",
        )?;
        let omega = omega_of(&w)?;
        for (r, c) in [(2, 2), (60, 5), (5, 60), (58, 58)] {
            assert!((omega.get(r, c) - 0.12).abs() < 1e-12, "{r} {c}");
        }
        assert!((omega.get(32, 32) - 0.72).abs() < 1e-12);
        // One radius away (5 cells): baseline + strength / e.
        let expected = 0.12 + 0.6 * libm::exp(-1.0);
        assert!((omega.get(32, 37) - expected).abs() < 1e-12);
        Ok(())
    }

    #[test]
    fn without_ruins_omega_is_the_baseline_with_the_drift() -> TestResult {
        let mut w = setup(false)?;
        run_stage(&mut w, &Detuning, "")?;
        let omega = omega_of(&w)?;
        let mean = omega.as_slice().iter().sum::<f64>() / 4096.0;
        assert!((mean - 0.12).abs() < 0.05, "{mean}");
        assert!(omega.as_slice().iter().any(|&v| (v - 0.12).abs() > 1e-6));
        Ok(())
    }

    #[test]
    fn omega_is_clipped_to_the_unit_interval() -> TestResult {
        let mut w = setup(true)?;
        run_stage(&mut w, &Detuning, "ruin_strength = 5.0\nbaseline = 1.0")?;
        let omega = omega_of(&w)?;
        assert!(omega.as_slice().iter().all(|v| (0.0..=1.0).contains(v)));
        assert!((omega.get(32, 32) - 1.0).abs() < 1e-12);
        Ok(())
    }

    #[test]
    fn the_same_inputs_give_the_same_omega() -> TestResult {
        assert_eq!(run_detuning_on_test_world()?, run_detuning_on_test_world()?);
        Ok(())
    }

    #[test]
    fn the_stage_reports_its_metrics() -> TestResult {
        let mut w = setup(true)?;
        let report = run_stage(&mut w, &Detuning, "")?;
        let defaults = Detuning.params(&table(""))?;
        let metrics = Stage::metrics(&Detuning, &w, defaults.as_ref(), &report);
        let mut names = Stage::metric_names(&Detuning, defaults.as_ref());
        names.sort_unstable();
        let keys: Vec<&str> = metrics.0.keys().map(String::as_str).collect();
        assert_eq!(names, keys);
        let mean = metrics.0.get("mean").copied().unwrap_or(f64::NAN);
        assert!(mean > 0.0 && mean < 1.0, "{mean}");
        Ok(())
    }

    #[test]
    fn parameters_have_defaults_and_are_checked() -> TestResult {
        let d = DetuningParams::default();
        assert!((d.baseline - 0.12).abs() < 1e-12);
        assert!((d.ruin_radius_km - 2.5).abs() < 1e-12);
        assert!((d.ruin_strength - 0.6).abs() < 1e-12);
        assert!((d.drift_amplitude - 0.12).abs() < 1e-12);
        for bad in [
            "baseline = 1.5",
            "baseline = -0.1",
            "baseline = nan",
            "ruin_radius_km = 0.0",
            "ruin_radius_km = inf",
            "ruin_strength = -1.0",
            "drift_amplitude = -0.1",
            "drift_amplitude = nan",
        ] {
            assert!(Detuning.params(&table(bad)).is_err(), "{bad}");
        }
        let err = Detuning
            .params(&table("ruin_radius = 2.0"))
            .err()
            .ok_or("an unknown key is refused")?;
        assert!(err.to_string().contains("ruin_radius"), "{err}");
        let err = Detuning
            .params(&table("baseline = 2.0"))
            .err()
            .ok_or("baseline 2.0 is refused")?;
        assert_eq!(err.parameter.as_deref(), Some("baseline"));
        Ok(())
    }
}
