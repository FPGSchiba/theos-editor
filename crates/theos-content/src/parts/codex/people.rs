//! The `people` kind.

use crate::kinds::Target;
use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::parts::common::{self, field};

const LIVES_IN: &[Target] = &[Target::kind("band"), Target::kind("floor")];

/// The spec of the `people` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "people",
        id: common::SLUG,
        placement: Placement::Flat("people"),
        fields: common::fields(
            false,
            &[
                field("lives_in", FieldType::Links(LIVES_IN), false),
                field("lens", FieldType::Lens, false),
            ],
        ),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
