//! The `map-version` kind.

use crate::kinds::{BodyGrammar, IdRule, KindSpec, Placement, Target};
use crate::parts::common;
use crate::parts::version::{self, VERSION_ID};

const OWN: &[Target] = &[Target::kind("map-version")];

/// The spec of the `map-version` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "map-version",
        id: IdRule {
            pattern: "^[a-z0-9]+(-[a-z0-9]+)*-v[0-9]+$",
            says: "the parent's slug, `-v` and a number",
        },
        placement: Placement::UnderParent("map"),
        fields: common::fields(false, &version::fields("map", OWN, false)),
        body: BodyGrammar::PROSE,
        checks: vec![VERSION_ID],
    }
}
