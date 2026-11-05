//! The `npc` kind.

use crate::kinds::Target;
use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::parts::common::{self, field};

const PARENT: &[Target] = &[Target::kind("faction"), Target::kind("people")];
const HOME: &[Target] = &[Target::kind("location"), Target::kind("floor")];

/// The spec of the `npc` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "npc",
        id: common::SLUG,
        placement: Placement::UnderParent("parent"),
        fields: common::fields(
            false,
            &[
                field("parent", FieldType::Link(PARENT), true),
                field("home", FieldType::Link(HOME), false),
                field("role", FieldType::Text, false),
                field("lens", FieldType::Lens, false),
            ],
        ),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
