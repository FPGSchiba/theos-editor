//! The values a plan treats as present before its first stage (spec P29 §5.2 item 5): the
//! parent's layers, features and feature in a child plan, and the kind's own seeded values.

use std::collections::BTreeMap;

use super::Planning;
use crate::child::values::{PARENT_FEATURE, PARENT_FEATURES, PARENT_PREFIX};
use crate::kind;
use crate::stage::{LayerSpec, MapKind};
use crate::world::LayerKind;

/// What is available before the first stage.
pub(super) struct Offered {
    /// The names offered, with their kinds.
    pub names: BTreeMap<String, LayerKind>,
    /// `describe` accepts a read of any `parent.*` name.
    pub open: bool,
}

impl Offered {
    /// What `planning` of a `kind` map offers.
    pub fn of(kind: MapKind, planning: Planning<'_>) -> Self {
        let mut names: BTreeMap<String, LayerKind> = kind::table(kind)
            .own_seeds
            .iter()
            .map(|spec| (spec.name.to_owned(), spec.kind))
            .collect();
        if let Planning::Child(parent) = planning {
            for entry in &parent.package.manifest.layers {
                if entry.level.is_none() {
                    names.insert(format!("{PARENT_PREFIX}{}", entry.name), LayerKind::F64);
                }
            }
            names.insert(
                PARENT_FEATURES.to_owned(),
                LayerKind::Value("parent_features"),
            );
            names.insert(
                PARENT_FEATURE.to_owned(),
                LayerKind::Value("parent_feature"),
            );
        }
        Self {
            names,
            open: matches!(planning, Planning::Describe),
        }
    }

    /// Whether a read of `spec` that nothing offers and no stage wrote is accepted.
    pub fn accepts(&self, spec: &LayerSpec) -> bool {
        self.open && spec.name.starts_with(PARENT_PREFIX)
    }
}
