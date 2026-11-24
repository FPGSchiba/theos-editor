//! The typed form of a stage, and the one impl that makes every typed stage a [`Stage`].

use std::collections::BTreeSet;
use std::fmt::Debug;

use serde::de::DeserializeOwned;

use super::params::{self, StageParams};
use super::{
    Finding, LayerSpec, MapKind, Metrics, ParamError, Stage, StageContext, StageError, StageReport,
};
use crate::world::World;

/// A stage with its own parameter type. Every stage implements this trait; the blanket impl
/// makes it a [`Stage`] that parses its parameters (unknown keys refused), validates them and
/// downcasts them, so no stage writes a downcast (ruling L2).
///
/// `Params` is deserialised from the recipe's stage table without its `id` key. Give every field
/// a default (`#[serde(default)]`) unless the recipe must set it. Enums tagged by a field
/// (`#[serde(tag = "type")]`) and `#[serde(flatten)]` fields hide their keys from the
/// unknown-key check, so they carry `#[serde(deny_unknown_fields)]`.
pub trait TypedStage: Send + Sync + 'static {
    /// The parameters.
    type Params: DeserializeOwned + Debug + Send + Sync + 'static;
    /// The id recipes use, `^[a-z][a-z0-9_]*$`.
    const ID: &'static str;
    /// One line for `describe` and the docs.
    const SUMMARY: &'static str;
    /// The map kinds the stage applies to.
    const KINDS: &'static [MapKind];

    /// The layers the stage reads with `params`.
    fn reads(&self, params: &Self::Params) -> Vec<LayerSpec>;

    /// The layers the stage writes with `params`.
    fn writes(&self, params: &Self::Params) -> Vec<LayerSpec>;

    /// The names of the metrics the stage reports with `params`, known before anything runs.
    fn metric_names(&self, _params: &Self::Params) -> Vec<&'static str> {
        Vec::new()
    }

    /// Checks `params` beyond their types and completes them: a stage may fill fields the recipe
    /// does not set (`#[serde(skip)]`), such as the parsed per-step processes of
    /// `landscape_evolution`.
    ///
    /// Domains go through [`Check`](super::Check) (`Check::of(Self::ID)`), one wording per
    /// rule: `finite`, `positive`, `at_least`, the intervals and `count_at_least`, and `refuse`
    /// for a rule of the stage's own. After `validate`, every number of the stage table must also
    /// keep the bounds of every recipe number (a float 0 or of a magnitude from 1e-12 to 1e12, a
    /// whole number at most a million), so a domain needs no upper bound of its own unless the
    /// stage raises a parameter to a power or sizes something by it.
    ///
    /// # Errors
    /// [`ParamError`] naming the stage and, where one is to blame, the parameter
    /// ([`ParamError::at`], [`Check::refuse`](super::Check::refuse)).
    fn validate(&self, _params: &mut Self::Params) -> Result<(), ParamError> {
        Ok(())
    }

    /// Checks the order of nested steps against the layers `available` before this stage.
    ///
    /// # Errors
    /// [`ParamError`] naming the step and what is wrong: a layer no earlier step writes, a layer
    /// declared as two kinds, or a step out of its allowed order.
    fn check_order(
        &self,
        _params: &Self::Params,
        _available: &BTreeSet<&str>,
    ) -> Result<(), ParamError> {
        Ok(())
    }

    /// Runs the stage; the context hands out only the declared layers.
    ///
    /// # Errors
    /// [`StageError`] when the stage breaks its declaration, is cancelled or fails.
    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &Self::Params,
    ) -> Result<StageReport, StageError>;

    /// Invariants checked after the run (finite values, ranges, balances).
    fn checks(
        &self,
        _world: &World,
        _params: &Self::Params,
        _report: &StageReport,
    ) -> Vec<Finding> {
        Vec::new()
    }

    /// The numbers the stage reports, under the names of [`TypedStage::metric_names`].
    fn metrics(&self, _world: &World, _params: &Self::Params, _report: &StageReport) -> Metrics {
        Metrics::default()
    }
}

/// What a stage says when handed the parameters of another stage. The runner keeps each stage
/// with its own parameters, so this marks a bug in the caller, never in a recipe.
const FOREIGN: &str = "was handed the parameters of another stage";

/// `params` as the parameters of `T`.
fn own<T: TypedStage>(params: &dyn StageParams) -> Option<&T::Params> {
    params.as_any().downcast_ref()
}

/// A typed stage is a [`Stage`]. Parameters of another stage give no layers and no metrics,
/// fail `check_order` and `run`, and give an error finding from `checks`.
impl<T: TypedStage> Stage for T {
    fn id(&self) -> &'static str {
        T::ID
    }

    fn summary(&self) -> &'static str {
        T::SUMMARY
    }

    fn kinds(&self) -> &'static [MapKind] {
        T::KINDS
    }

    fn params(&self, table: &toml::Table) -> Result<Box<dyn StageParams>, ParamError> {
        let mut parsed: T::Params = params::parse(T::ID, table)?;
        TypedStage::validate(self, &mut parsed)?;
        params::check_numbers(T::ID, table)?;
        Ok(Box::new(parsed))
    }

    fn reads(&self, params: &dyn StageParams) -> Vec<LayerSpec> {
        own::<T>(params)
            .map(|p| TypedStage::reads(self, p))
            .unwrap_or_default()
    }

    fn writes(&self, params: &dyn StageParams) -> Vec<LayerSpec> {
        own::<T>(params)
            .map(|p| TypedStage::writes(self, p))
            .unwrap_or_default()
    }

    fn metric_names(&self, params: &dyn StageParams) -> Vec<&'static str> {
        own::<T>(params)
            .map(|p| TypedStage::metric_names(self, p))
            .unwrap_or_default()
    }

    fn check_order(
        &self,
        params: &dyn StageParams,
        available: &BTreeSet<&str>,
    ) -> Result<(), ParamError> {
        let p = own::<T>(params).ok_or_else(|| ParamError {
            stage: T::ID.to_owned(),
            parameter: None,
            message: FOREIGN.to_owned(),
        })?;
        TypedStage::check_order(self, p, available)
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &dyn StageParams,
    ) -> Result<StageReport, StageError> {
        let p = own::<T>(params).ok_or_else(|| ctx.fail(FOREIGN))?;
        TypedStage::run(self, ctx, p)
    }

    fn checks(
        &self,
        world: &World,
        params: &dyn StageParams,
        report: &StageReport,
    ) -> Vec<Finding> {
        match own::<T>(params) {
            Some(p) => TypedStage::checks(self, world, p, report),
            None => vec![Finding::error(FOREIGN)],
        }
    }

    fn metrics(&self, world: &World, params: &dyn StageParams, report: &StageReport) -> Metrics {
        own::<T>(params)
            .map(|p| TypedStage::metrics(self, world, p, report))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde::Deserialize;

    use super::TypedStage;
    use crate::grid::Grid;
    use crate::stage::test_support::{Harness, NoParams};
    use crate::stage::{
        Finding, LayerSpec, MapKind, Metrics, ParamError, Severity, Stage, StageContext,
        StageError, StageReport,
    };
    use crate::world::{Layer, World};

    #[derive(Debug, Deserialize)]
    #[serde(default)]
    struct RateParams {
        rate: f64,
        steps: u32,
        deposit: bool,
        /// Derived by `validate`: the rate per step.
        #[serde(skip)]
        per_step: f64,
    }

    impl Default for RateParams {
        fn default() -> Self {
            Self {
                rate: 2.5,
                steps: 3,
                deposit: false,
                per_step: 0.0,
            }
        }
    }

    /// A stage whose writes and metrics depend on its parameters.
    struct Rate;

    impl TypedStage for Rate {
        type Params = RateParams;
        const ID: &'static str = "t";
        const SUMMARY: &'static str = "A test stage with parameters.";
        const KINDS: &'static [MapKind] = &[MapKind::Terrain];

        fn reads(&self, _: &RateParams) -> Vec<LayerSpec> {
            vec![LayerSpec::f64("height")]
        }

        fn writes(&self, p: &RateParams) -> Vec<LayerSpec> {
            let mut writes = vec![LayerSpec::f64("erosion")];
            if p.deposit {
                writes.push(LayerSpec::f64("deposition"));
            }
            writes
        }

        fn metric_names(&self, _: &RateParams) -> Vec<&'static str> {
            vec!["mean_rate"]
        }

        fn validate(&self, p: &mut RateParams) -> Result<(), ParamError> {
            if p.steps == 0 {
                return Err(ParamError::at(Self::ID, "steps", "must be positive"));
            }
            p.per_step = p.rate / f64::from(p.steps);
            Ok(())
        }

        fn check_order(
            &self,
            _: &RateParams,
            available: &BTreeSet<&str>,
        ) -> Result<(), ParamError> {
            if available.contains("height") {
                Ok(())
            } else {
                Err(ParamError {
                    stage: Self::ID.into(),
                    parameter: None,
                    message: "needs height".into(),
                })
            }
        }

        fn run(
            &self,
            ctx: &mut StageContext<'_>,
            p: &RateParams,
        ) -> Result<StageReport, StageError> {
            ctx.write("erosion", Layer::F64(Grid::filled(4, 4, p.rate)))?;
            let mut report = StageReport::default();
            report.values.insert("rate".into(), p.rate);
            Ok(report)
        }

        fn checks(&self, _: &World, _: &RateParams, report: &StageReport) -> Vec<Finding> {
            vec![Finding::warning(format!("{} values", report.values.len()))]
        }

        fn metrics(&self, _: &World, p: &RateParams, _: &StageReport) -> Metrics {
            Metrics([("mean_rate".to_owned(), p.rate)].into())
        }
    }

    fn table(text: &str) -> toml::Table {
        text.parse().unwrap()
    }

    fn rate(params: &dyn crate::stage::StageParams) -> f64 {
        params
            .as_any()
            .downcast_ref::<RateParams>()
            .map(|p| p.rate)
            .unwrap()
    }

    fn per_step(params: &dyn crate::stage::StageParams) -> f64 {
        params
            .as_any()
            .downcast_ref::<RateParams>()
            .map(|p| p.per_step)
            .unwrap()
    }

    #[test]
    fn params_reject_unknown_keys_naming_them() {
        let err = Rate.params(&table("id = \"t\"\nbogus = 1")).unwrap_err();
        assert_eq!(err.stage, "t");
        assert_eq!(err.parameter.as_deref(), Some("bogus"));
        let text = err.to_string();
        assert!(text.contains("`t`") && text.contains("`bogus`"), "{text}");
    }

    #[test]
    fn params_fill_defaults_and_run_validate() {
        let params = Rate.params(&table("id = \"t\"")).unwrap();
        assert!((rate(params.as_ref()) - 2.5).abs() < 1e-12);
        let params = Rate.params(&table("rate = 4.0")).unwrap();
        assert!((rate(params.as_ref()) - 4.0).abs() < 1e-12);
        let err = Rate.params(&table("steps = 0")).unwrap_err();
        assert_eq!(
            err,
            ParamError {
                stage: "t".into(),
                parameter: Some("steps".into()),
                message: "must be positive".into(),
            }
        );
        assert_eq!(
            err.to_string(),
            "stage `t`, parameter `steps`: must be positive"
        );
    }

    #[test]
    fn validate_completes_the_parameters_it_checks() {
        let params = Rate.params(&table("rate = 6.0\nsteps = 4")).unwrap();
        assert!((per_step(params.as_ref()) - 1.5).abs() < 1e-12);
        let params = Rate.params(&table("per_step = 9.0"));
        assert_eq!(
            params.unwrap_err().parameter.as_deref(),
            Some("per_step"),
            "a field filled by validate is not a recipe parameter"
        );
    }

    #[test]
    fn param_errors_at_a_parameter_name_it() {
        assert_eq!(
            ParamError::at("t", "source[0].r", "must be positive"),
            ParamError {
                stage: "t".into(),
                parameter: Some("source[0].r".into()),
                message: "must be positive".into(),
            }
        );
    }

    #[test]
    fn the_blanket_forwards_every_capability() {
        let stage: &dyn Stage = &Rate;
        assert_eq!(stage.id(), "t");
        assert_eq!(stage.summary(), "A test stage with parameters.");
        assert_eq!(stage.kinds(), [MapKind::Terrain]);
        let plain = stage.params(&toml::Table::new()).unwrap();
        let deposit = stage.params(&table("deposit = true")).unwrap();
        assert_eq!(stage.reads(plain.as_ref()), [LayerSpec::f64("height")]);
        assert_eq!(stage.writes(plain.as_ref()), [LayerSpec::f64("erosion")]);
        assert_eq!(
            stage.writes(deposit.as_ref()),
            [LayerSpec::f64("erosion"), LayerSpec::f64("deposition")],
            "the writes depend on the parameters"
        );
        assert_eq!(stage.metric_names(plain.as_ref()), ["mean_rate"]);
        assert!(
            stage
                .check_order(plain.as_ref(), &BTreeSet::from(["height"]))
                .is_ok()
        );
        assert!(stage.check_order(plain.as_ref(), &BTreeSet::new()).is_err());
        let mut h = Harness::new();
        let report = h.with(stage, |ctx| stage.run(ctx, plain.as_ref())).unwrap();
        assert!(h.world.f64("erosion").is_some());
        let findings = stage.checks(&h.world, plain.as_ref(), &report);
        assert_eq!(findings, [Finding::warning("1 values")]);
        let metrics = stage.metrics(&h.world, plain.as_ref(), &report);
        assert_eq!(metrics.0.keys().collect::<Vec<_>>(), ["mean_rate"]);
    }

    #[test]
    fn the_defaults_of_a_typed_stage_add_nothing() {
        let stage: &dyn Stage = &crate::stage::test_support::Step;
        let params = stage.params(&toml::Table::new()).unwrap();
        assert_eq!(stage.metric_names(params.as_ref()), Vec::<&str>::new());
        assert!(stage.check_order(params.as_ref(), &BTreeSet::new()).is_ok());
        let h = Harness::new();
        let report = StageReport::default();
        assert_eq!(
            stage.checks(&h.world, params.as_ref(), &report),
            Vec::<Finding>::new()
        );
        assert_eq!(
            stage.metrics(&h.world, params.as_ref(), &report),
            Metrics::default()
        );
    }

    #[test]
    fn parameters_of_another_stage_are_refused() {
        let stage: &dyn Stage = &Rate;
        let foreign = NoParams {};
        assert_eq!(stage.reads(&foreign), Vec::<LayerSpec>::new());
        assert_eq!(stage.writes(&foreign), Vec::<LayerSpec>::new());
        assert_eq!(stage.metric_names(&foreign), Vec::<&str>::new());
        let err = stage.check_order(&foreign, &BTreeSet::new()).unwrap_err();
        assert_eq!(err.stage, "t");
        let mut h = Harness::new();
        let err = h.with(stage, |ctx| stage.run(ctx, &foreign)).unwrap_err();
        assert!(
            matches!(&err, StageError::Failed { stage, .. } if stage == "t"),
            "{err:?}"
        );
        let report = StageReport::default();
        let findings = stage.checks(&h.world, &foreign, &report);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(
            stage.metrics(&h.world, &foreign, &report),
            Metrics::default()
        );
    }
}
