//! The `belief` kind.

use crate::kinds::IdRule;
use crate::kinds::Target;
use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::parts::common::{self, field};

const TRUTH: &[Target] = &[Target::kind("truth")];
const IN_WORLD: &[Target] = &[Target::narrowed("topic", "track", "in-world")];

/// The spec of the `belief` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "belief",
        id: IdRule {
            pattern: "^B-[0-9]{3,}$",
            says: "`B-` and at least three digits",
        },
        placement: Placement::Flat("lore/beliefs"),
        fields: common::fields(
            true,
            &[
                field("held_by", FieldType::Links(common::HOLDERS), true),
                field("distorts", FieldType::Link(TRUTH), false),
                field(
                    "accuracy",
                    FieldType::Enum(&["false", "partial", "close"]),
                    false,
                ),
                field("topic", FieldType::Link(IN_WORLD), false),
            ],
        ),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
