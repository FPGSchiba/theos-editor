//! The `truth` kind.

use crate::kinds::IdRule;
use crate::kinds::Target;
use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::parts::common::{self, field};

const TRUTH: &[Target] = &[Target::kind("truth")];
const EXPLAINING: &[Target] = &[Target::narrowed("topic", "track", "explaining")];
const EQUATION: &[Target] = &[Target::kind("equation")];

/// The spec of the `truth` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "truth",
        id: IdRule {
            pattern: "^T-[0-9]{3,}$",
            says: "`T-` and at least three digits",
        },
        placement: Placement::Flat("lore/truths"),
        fields: common::fields(
            true,
            &[
                field(
                    "target",
                    FieldType::Enum(&["hinted", "suspected", "confirmed", "never"]),
                    true,
                ),
                field("requires", FieldType::Links(TRUTH), false),
                field("topic", FieldType::Link(EXPLAINING), false),
                field("cites", FieldType::Links(EQUATION), false),
            ],
        ),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
