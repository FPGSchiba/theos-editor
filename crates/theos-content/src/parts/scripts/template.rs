//! The `template` kind.

use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::parts::common::{self, field};

/// The spec of the `template` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "template",
        id: common::SLUG,
        placement: Placement::Flat("templates"),
        fields: common::fields(
            false,
            &[field(
                "purpose",
                FieldType::Enum(&["dialogue", "description"]),
                true,
            )],
        ),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
