//! The table of settlement maps. Empty until the settlement stages and package layers arrive;
//! settlement features are numbered per type, like terrain's.

use super::{FrameRule, IdScheme, KindTable, no_metrics, plain_title, seeds_nothing};
use crate::features::output;
use crate::stage::MapKind;

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
    frame: FrameRule::Recipe,
    seeded: seeds_nothing,
};
