//! The `band` kind.

use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::kinds::{TIERS, Target};
use crate::parts::common::{self, field};

const TOWER: &[Target] = &[Target::kind("tower")];

/// The spec of the `band` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "band",
        id: common::SLUG,
        placement: Placement::Flat("world"),
        fields: common::fields(
            false,
            &[
                field("parent", FieldType::Link(TOWER), true),
                field("floors", FieldType::FloorRange, true),
                field("mode", FieldType::Text, false),
                field("mood", FieldType::Text, false),
                field("belief_stage", FieldType::Text, false),
                field("tier", FieldType::Enum(TIERS), false),
            ],
        ),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
