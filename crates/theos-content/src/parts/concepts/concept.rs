//! The `concept` kind.

use crate::kinds::Target;
use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::parts::common::{self, field};

const FOR: &[Target] = &[
    Target::kind("tower"),
    Target::kind("band"),
    Target::kind("floor"),
    Target::kind("location"),
    Target::kind("people"),
    Target::kind("faction"),
    Target::kind("npc"),
    Target::kind("thing"),
];
const PICKED: &[Target] = &[Target::kind("concept-version")];

/// The spec of the `concept` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "concept",
        id: common::SLUG,
        placement: Placement::Flat("concepts"),
        fields: common::fields(
            false,
            &[
                field("for", FieldType::Links(FOR), true),
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
