//! The `faction` kind.

use crate::kinds::Target;
use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::parts::common::{self, field};

const PEOPLE: &[Target] = &[Target::kind("people")];

/// The spec of the `faction` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "faction",
        id: common::SLUG,
        placement: Placement::UnderParent("parent"),
        fields: common::fields(
            false,
            &[
                field("parent", FieldType::Link(PEOPLE), true),
                field("lens", FieldType::Lens, false),
            ],
        ),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
