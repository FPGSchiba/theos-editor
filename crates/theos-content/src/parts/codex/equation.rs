//! The `equation` kind.

use crate::kinds::IdRule;
use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::parts::common::{self, field};

/// The spec of the `equation` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "equation",
        id: IdRule {
            pattern: "^E[0-9]+$",
            says: "`E` and a number",
        },
        placement: Placement::Flat("canon/formalism"),
        fields: common::fields(
            false,
            &[
                field("source", FieldType::Url, true),
                field("generated", FieldType::True, true),
            ],
        ),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
