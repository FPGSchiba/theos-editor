//! The `clue` kind.

use crate::kinds::IdRule;
use crate::kinds::{BodyGrammar, FieldType, KindSpec, Placement};
use crate::kinds::{TIERS, Target};
use crate::parts::common::{self, field};

const TRUTH: &[Target] = &[Target::kind("truth")];
const CLUE: &[Target] = &[Target::kind("clue")];

/// The spec of the `clue` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "clue",
        id: IdRule {
            pattern: "^C-[0-9]{3,}$",
            says: "`C-` and at least three digits",
        },
        placement: Placement::Flat("lore/clues"),
        fields: common::fields(
            true,
            &[
                field("reveals", FieldType::Link(TRUTH), true),
                field(
                    "moves_to",
                    FieldType::Enum(&["hinted", "suspected", "confirmed"]),
                    true,
                ),
                field("where", FieldType::Links(common::PLACES), true),
                field("gate", FieldType::Enum(TIERS), true),
                field("after", FieldType::Links(CLUE), false),
                field("vector", FieldType::Text, true),
            ],
        ),
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}
