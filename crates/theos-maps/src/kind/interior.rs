//! The table of interior maps. Empty except for its frame rule; phase P34 fills it. Interior
//! features are numbered per feature kind.

use super::{FrameRule, IdScheme, KindTable, no_metrics, plain_title};
use crate::features::output;
use crate::stage::MapKind;

/// The interior entry.
pub(super) const TABLE: KindTable = KindTable {
    kind: MapKind::Interior,
    layers: &[],
    optional_layers: &[],
    renders: &[],
    types: &[],
    ids: IdScheme::ByKind,
    summary_paths: &[],
    metrics: no_metrics,
    features: output::build_set,
    title: plain_title,
    frame: FrameRule::RecipeChild,
    own_seeds: &[],
};
