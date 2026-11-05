//! The `location` kind.

use crate::kinds::Target;
use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::parts::common::{self, field};

const PARENT: &[Target] = &[Target::kind("floor"), Target::kind("location")];

/// The spec of the `location` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "location",
        id: common::SLUG,
        placement: Placement::UnderParent("parent"),
        fields: common::fields(
            false,
            &[
                field("parent", FieldType::Link(PARENT), true),
                field("type", FieldType::Text, false),
            ],
        ),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
