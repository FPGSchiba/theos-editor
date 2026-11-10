//! The `thing` kind.

use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::parts::common::{self, field};

/// The spec of the `thing` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "thing",
        id: common::SLUG,
        placement: Placement::Flat("things"),
        fields: common::fields(
            false,
            &[
                field("type", FieldType::Text, true),
                field("where", FieldType::Links(common::PLACES), false),
            ],
        ),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
