//! The `tower` kind.

use crate::kinds::{BodyGrammar, KindSpec, Placement};
use crate::parts::common;

/// The spec of the `tower` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "tower",
        id: common::SLUG,
        placement: Placement::Flat("world"),
        fields: common::fields(false, &[]),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
