//! The table of terrain maps: P08's layers, renders, types and metrics.

use serde_json::Value;
use theos_map_format::FormatError;

use super::{FrameRule, IdScheme, KindTable};
use crate::features::TYPES;
use crate::features::output;
use crate::metrics;
use crate::package::{OMEGA_LAYER, TERRAIN_LAYERS};
use crate::render::{self, RENDERS};
use crate::stage::{MapKind, Metrics, TypedStage};
use crate::stages::terrain::travel::Travel;
use crate::world::World;

/// The terrain entry.
pub(super) const TABLE: KindTable = KindTable {
    kind: MapKind::Terrain,
    layers: TERRAIN_LAYERS,
    optional_layers: &[OMEGA_LAYER],
    renders: RENDERS,
    types: TYPES,
    ids: IdScheme::ByType,
    summary_paths: metrics::PATHS,
    metrics: terrain_metrics,
    features: output::build,
    title: render::title_line,
    frame: FrameRule::Recipe,
    own_seeds: &[],
    check_frame: None,
    check_plan: None,
};

/// The parity metrics; the Chime walk comes from the `travel` stage's metrics.
fn terrain_metrics(world: &World, stages: &[(String, Metrics)]) -> Result<Value, FormatError> {
    let travel = stages
        .iter()
        .find(|(id, _)| id == Travel::ID)
        .map(|(_, metrics)| metrics);
    metrics::compute(world, travel)
}
