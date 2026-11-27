//! One table per kind of map (spec P29 §5.2): everything that differs between terrain,
//! settlement and interior maps and that the runner, the outputs and the plan look up by
//! [`MapKind`]. A new kind is one module with one [`KindTable`] and one line in [`KINDS`].

mod interior;
mod settlement;
mod terrain;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use geojson::FeatureCollection;
use serde_json::Value;
use theos_map_format::FormatError;

use crate::child::{FrameRule, ParentContext};
use crate::error::RunError;
use crate::features::output::FeaturesError;
use crate::package::PackageLayer;
use crate::recipe::Plan;
use crate::render::Render;
use crate::stage::{LayerSpec, MapKind, Metrics};
use crate::world::{Layer, MapFrame, World};

/// How the ids of a kind's features are numbered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdScheme {
    /// `<type>-<n>`, numbered per type from 1.
    ByType,
    /// `<kind>-<n>`, numbered per feature kind from 1.
    ByKind,
}

/// Builds a value a kind seeds into the world, from the plan and the parent of a child run.
pub(crate) type SeedFn = fn(&Plan, Option<&ParentContext>) -> Result<Layer, RunError>;

/// A world value a kind offers before the first stage besides `parent.*`: its declaration and
/// the function that builds it.
pub(crate) struct OwnSeed {
    /// The name, kind and optionality of the value.
    pub spec: LayerSpec,
    /// Builds the value.
    pub build: SeedFn,
}

/// Refuses a frame a kind cannot use, with the reason.
pub(crate) type FrameCheck = fn(&MapFrame) -> Result<(), String>;

/// The top-level metrics of the summary, from the world and the stages' metrics.
pub(crate) type MetricsFn = fn(&World, &[(String, Metrics)]) -> Result<Value, FormatError>;

/// The features output and how many features each id key has.
pub(crate) type FeaturesFn =
    fn(&World, IdScheme) -> Result<(FeatureCollection, BTreeMap<String, usize>), FeaturesError>;

/// The title line of the people renders, from the recipe id, the seed and the summary metrics.
pub(crate) type TitleFn = fn(recipe: &str, seed: u64, summary: &Value) -> String;

/// Everything that differs between the kinds of map.
pub(crate) struct KindTable {
    /// The kind this entry describes.
    pub kind: MapKind,
    /// The package layers, in order.
    pub layers: &'static [PackageLayer],
    /// Package layers written when the world holds them.
    pub optional_layers: &'static [PackageLayer],
    /// The standard and extra renders of the kind.
    pub renders: &'static [Render],
    /// The feature types as `(kind, type)`; the paths `features.<type>` of constraints.
    pub types: &'static [(&'static str, &'static str)],
    /// How feature ids are numbered.
    pub ids: IdScheme,
    /// The summary paths a constraint may name besides `features.*` and `stages.*`.
    pub summary_paths: &'static [&'static str],
    /// The top-level summary metrics.
    pub metrics: MetricsFn,
    /// Builds the features output.
    pub features: FeaturesFn,
    /// The title line of the people renders.
    pub title: TitleFn,
    /// What decides the frame.
    pub frame: FrameRule,
    /// The world values the kind offers besides `parent.*`, available before the first stage.
    pub own_seeds: &'static [OwnSeed],
    /// Refuses a frame the kind cannot use, with the reason; the plan names `size_km`. `None`
    /// for a kind that takes every frame the recipe checks allow.
    pub check_frame: Option<FrameCheck>,
}

/// One entry per [`MapKind`].
pub(crate) const KINDS: &[KindTable] = &[terrain::TABLE, settlement::TABLE, interior::TABLE];

/// The table of `kind`; a test checks that every kind has one.
pub(crate) fn table(kind: MapKind) -> &'static KindTable {
    KINDS
        .iter()
        .find(|t| t.kind == kind)
        .unwrap_or(&terrain::TABLE)
}

/// The metrics of a kind that has none: an empty object.
pub(super) fn no_metrics(_: &World, _: &[(String, Metrics)]) -> Result<Value, FormatError> {
    theos_map_format::json::to_checked_value(&BTreeMap::<String, f64>::new())
}

/// The title line of a kind without numbers of its own: recipe and seed.
pub(super) fn plain_title(recipe: &str, seed: u64, _: &Value) -> String {
    format!("{recipe} \u{b7} seed {seed}")
}
