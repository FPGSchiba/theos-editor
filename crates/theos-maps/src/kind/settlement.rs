//! The table of settlement maps. Empty until the settlement stages and package layers arrive;
//! settlement features are numbered per type, like terrain's.

use super::{FrameRule, IdScheme, KindTable, OwnSeed, no_metrics, plain_title};
use crate::child::values::CLASS_FRAME;
use crate::child::{ClassSource, ParentContext, class_frame_of};
use crate::error::RunError;
use crate::features::output;
use crate::recipe::{Plan, RecipeError};
use crate::stage::{LayerSpec, MapKind};
use crate::world::Layer;

/// The settlement entry.
pub(super) const TABLE: KindTable = KindTable {
    kind: MapKind::Settlement,
    layers: &[],
    optional_layers: &[],
    renders: &[],
    types: &[],
    ids: IdScheme::ByType,
    summary_paths: &[],
    metrics: no_metrics,
    features: output::build_set,
    title: plain_title,
    frame: FrameRule::SettlementClass,
    own_seeds: &[OwnSeed {
        spec: LayerSpec::value(CLASS_FRAME, "class_frame"),
        build: class_frame,
    }],
    check_frame: None,
};

/// The class table entry of the map (`class_frame`): the class of the parent feature, or of the
/// `site_terrain` stage of a map without a parent.
fn class_frame(plan: &Plan, parent: Option<&ParentContext>) -> Result<Layer, RunError> {
    let source = match parent {
        Some(parent) => ClassSource::Parent(&parent.feature.ty),
        None => ClassSource::Site,
    };
    let value = class_frame_of(MapKind::Settlement, &plan.recipe, source)?.ok_or_else(|| {
        RecipeError::Invalid {
            recipe: plan.recipe_id.clone(),
            stage: None,
            layer: Some(CLASS_FRAME.to_owned()),
            parameter: None,
            message: "this kind of map has no class table".to_owned(),
        }
    })?;
    Ok(Layer::Value(Box::new(value)))
}
