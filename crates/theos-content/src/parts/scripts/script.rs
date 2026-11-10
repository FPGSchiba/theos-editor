//! The `script` kind.

use crate::kinds::IdRule;
use crate::kinds::{BeatAttribute, Target};
use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::parts::common::{self, field};

const BEATS: &[BeatAttribute] = &[
    BeatAttribute {
        key: "at",
        targets: &[Target::kind("location"), Target::kind("floor")],
    },
    BeatAttribute {
        key: "with",
        targets: &[
            Target::kind("npc"),
            Target::kind("faction"),
            Target::kind("people"),
        ],
    },
    BeatAttribute {
        key: "says",
        targets: &[Target::kind("belief")],
    },
    BeatAttribute {
        key: "reveals",
        targets: &[Target::kind("clue")],
    },
];

/// The spec of the `script` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "script",
        id: IdRule {
            pattern: "^S-[0-9]{3,}$",
            says: "`S-` and at least three digits",
        },
        placement: Placement::Flat("story/scripts"),
        fields: common::fields(
            true,
            &[field("where", FieldType::Links(common::PLACES), false)],
        ),
        body: BodyGrammar {
            beats: Some(BEATS),
            beat_tokens: &["when"],
            notes: None,
            verbatim: false,
        },
        checks: Vec::new(),
    }
}
