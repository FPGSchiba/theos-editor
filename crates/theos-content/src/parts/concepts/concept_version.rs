//! The `concept-version` kind.

use crate::kinds::{BodyGrammar, IdRule, KindSpec, Placement, Target};
use crate::parts::common;
use crate::parts::version::{self, VERSION_ID};

const OWN: &[Target] = &[Target::kind("concept-version")];

/// The spec of the `concept-version` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "concept-version",
        id: IdRule {
            pattern: "^[a-z0-9]+(-[a-z0-9]+)*-v[0-9]+$",
            says: "the parent's slug, `-v` and a number",
        },
        placement: Placement::UnderParent("concept"),
        fields: common::fields(false, &version::fields("concept", OWN, true)),
        body: BodyGrammar::PROSE,
        checks: vec![VERSION_ID],
    }
}
