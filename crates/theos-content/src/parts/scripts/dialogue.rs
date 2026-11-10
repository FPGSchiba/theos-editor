//! The `dialogue` kind.

use crate::kinds::Target;
use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::parts::common::{self, field};

const SPEAKER: &[Target] = &[
    Target::kind("npc"),
    Target::kind("faction"),
    Target::kind("people"),
];

/// The spec of the `dialogue` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "dialogue",
        id: common::SLUG,
        placement: Placement::Flat("dialogue"),
        fields: common::fields(false, &[field("speaker", FieldType::Link(SPEAKER), true)]),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
