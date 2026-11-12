//! The `map` kind.

use crate::kinds::Target;
use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::parts::common::{self, field};

const PICKED: &[Target] = &[Target::kind("map-version")];

/// The spec of the `map` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "map",
        id: common::SLUG,
        placement: Placement::Flat("maps"),
        fields: common::fields(
            false,
            &[
                field("for", FieldType::Links(common::PLACES), true),
                field("type", FieldType::Text, false),
                field("picked", FieldType::Links(PICKED), false),
            ],
        ),
        body: BodyGrammar {
            beats: None,
            beat_tokens: &[],
            notes: Some(PICKED),
            verbatim: false,
        },
        checks: Vec::new(),
    }
}
