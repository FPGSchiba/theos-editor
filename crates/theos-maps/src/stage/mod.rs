//! The Stage contract: every step of map generation is a [`Stage`] (spec §5.3, architecture
//! §11).
//!
//! A stage declares the layers it reads and writes for its parameters and the names of its
//! metrics before anything runs. It runs in a [`StageContext`] that hands out only its declared
//! layers, and after the run its checks and metrics see the world, the parameters and the
//! [`StageReport`]. Stages implement [`TypedStage`]; one blanket impl makes each a [`Stage`]. The
//! [`Registry`] lists them, one line per stage in [`crate::stages::register`].

mod context;
mod error;
mod layer_spec;
mod params;
mod registry;
mod report;
#[cfg(test)]
mod test_support;
mod typed;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub use context::{StageContext, StepInfo};
pub use error::{Access, ParamError, StageError};
pub use layer_spec::LayerSpec;
pub use params::{Check, MAX_COUNT, MAX_MAGNITUDE, MIN_MAGNITUDE, StageParams};
pub use registry::Registry;
pub use report::{Finding, Metrics, Severity, StageReport};
pub use typed::TypedStage;

use crate::world::World;

/// The kinds of map a stage applies to; recipes name them in snake case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapKind {
    /// A terrain map.
    Terrain,
}

impl std::fmt::Display for MapKind {
    /// The kind as recipes spell it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Terrain => f.write_str("terrain"),
        }
    }
}

/// One step of map generation, object safe so that the registry and the runner can hold any
/// stage. Stages implement [`TypedStage`], which gives them this trait.
pub trait Stage: Send + Sync {
    /// The id recipes use, `^[a-z][a-z0-9_]*$`.
    fn id(&self) -> &'static str;

    /// One line for `describe` and the docs.
    fn summary(&self) -> &'static str;

    /// The map kinds the stage applies to.
    fn kinds(&self) -> &'static [MapKind];

    /// Parses and checks the stage's parameters from its recipe table; unknown keys are errors.
    ///
    /// # Errors
    /// [`ParamError`] naming the stage and the parameter.
    fn params(&self, table: &toml::Table) -> Result<Box<dyn StageParams>, ParamError>;

    /// The layers the stage reads with `params` (name, kind, optional).
    fn reads(&self, params: &dyn StageParams) -> Vec<LayerSpec>;

    /// The layers the stage writes with `params`.
    fn writes(&self, params: &dyn StageParams) -> Vec<LayerSpec>;

    /// The names of the metrics the stage reports with `params`, known before anything runs.
    fn metric_names(&self, params: &dyn StageParams) -> Vec<&'static str>;

    /// Checks the order of nested steps against the layers `available` before this stage.
    ///
    /// # Errors
    /// [`ParamError`] naming the step and what is wrong: a layer no earlier step writes, a layer
    /// declared as two kinds, or a step out of its allowed order; also, with no parameter, when
    /// `params` are the parameters of another stage.
    fn check_order(
        &self,
        params: &dyn StageParams,
        available: &BTreeSet<&str>,
    ) -> Result<(), ParamError>;

    /// Runs the stage. The context hands out only the declared layers; the report carries values
    /// that are not layers.
    ///
    /// # Errors
    /// [`StageError`] when the stage breaks its declaration, is cancelled or fails.
    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &dyn StageParams,
    ) -> Result<StageReport, StageError>;

    /// Invariants checked after the run (finite values, ranges, balances).
    fn checks(&self, world: &World, params: &dyn StageParams, report: &StageReport)
    -> Vec<Finding>;

    /// The numbers the stage reports into the summary, under the names of `metric_names`.
    fn metrics(&self, world: &World, params: &dyn StageParams, report: &StageReport) -> Metrics;
}

#[cfg(test)]
mod tests {
    use super::MapKind;

    #[test]
    fn map_kinds_are_snake_case_in_recipes() {
        assert_eq!(
            serde_json::to_value(MapKind::Terrain).unwrap(),
            serde_json::json!("terrain")
        );
        let kind: MapKind = serde_json::from_value(serde_json::json!("terrain")).unwrap();
        assert_eq!(kind, MapKind::Terrain);
    }
}
