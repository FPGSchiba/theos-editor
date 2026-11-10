//! The `floor` kind.

use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::kinds::{IdRule, Target};
use crate::parts::common::{self, field};

const BAND: &[Target] = &[Target::kind("band")];

/// The spec of the `floor` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "floor",
        id: IdRule {
            pattern: "^f[0-9]{2}[a-z0-9]*(-[a-z0-9]+)*$",
            says: "`f`, two digits, then an optional slug part",
        },
        placement: Placement::UnderParent("parent"),
        fields: common::fields(
            true,
            &[
                field("parent", FieldType::Link(BAND), true),
                field("number", FieldType::Integer { min: 1, max: 100 }, true),
            ],
        ),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
