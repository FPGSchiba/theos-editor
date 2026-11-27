//! The table of interior maps (spec P34 §5.10): the package layers, the feature types, the
//! summary of the mission. Interior features are numbered per feature kind.

use std::collections::BTreeSet;

use serde_json::{Map, Value};
use theos_map_format::FormatError;

use super::{FrameRule, IdScheme, KindTable, plain_title};
use crate::features::interior;
use crate::package::INTERIOR_LAYERS;
use crate::recipe::PlannedStage;
use crate::stage::{MapKind, Metrics, TypedStage};
use crate::stages::interior::mission::{Mission, MissionParams};
use crate::stages::interior::mission_core::MissionReport;
use crate::world::{ValueType, World};

/// The interior entry.
pub(super) const TABLE: KindTable = KindTable {
    kind: MapKind::Interior,
    layers: INTERIOR_LAYERS,
    optional_layers: &[],
    renders: crate::render::interior::RENDERS,
    types: TYPES,
    recipe_types: room_types,
    ids: IdScheme::ByKind,
    summary_paths: &[],
    metrics: interior_metrics,
    features: interior::build,
    title: plain_title,
    frame: FrameRule::RecipeChild,
    own_seeds: &[],
    check_frame: Some(crate::stages::interior::units::check_cell_size),
    check_plan: Some(crate::stages::interior::fit::check_plan),
};

/// The static feature types of an interior: the door, connection and route types. The room
/// types come from the recipe ([`room_types`]).
const TYPES: &[(&str, &str)] = &[
    ("door", "open"),
    ("door", "locked"),
    ("door", "drop"),
    ("connection", "open"),
    ("connection", "locked"),
    ("connection", "oneway"),
    ("route", "critical"),
];

/// The room types of the recipe's program: every `[[stage.room]]` type of its `mission` stages,
/// sorted, each once.
fn room_types(stages: &[PlannedStage]) -> Vec<String> {
    stages
        .iter()
        .filter(|s| s.id == Mission::ID)
        .filter_map(|s| s.params.as_any().downcast_ref::<MissionParams>())
        .flat_map(|p| p.room.iter().map(|r| r.ty.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// The top-level summary of an interior: `mission.entries`, one object per rule entry with its
/// rule id, asked, applied and undone (ruling R33). The analysis metrics are the `playability`
/// stage's, under `stages`.
fn interior_metrics(world: &World, _: &[(String, Metrics)]) -> Result<Value, FormatError> {
    let mut out = Map::new();
    if let Some(report) = world.value::<MissionReport>(MissionReport::NAME) {
        out.insert(
            "mission.entries".to_owned(),
            theos_map_format::json::to_checked_value(&report.per_entry)?,
        );
    }
    Ok(Value::Object(out))
}
