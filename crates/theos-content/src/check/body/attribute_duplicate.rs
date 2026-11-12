//! `body/attribute-duplicate`: a key appears once in a beat.

use super::{BodyInput, BodyRule, problem};
use crate::finding::Severity;
use crate::kinds::Problem;

/// The rule.
pub(super) const RULE: BodyRule = BodyRule {
    id: "body/attribute-duplicate",
    severity: Severity::Error,
    run,
};

fn run(input: &BodyInput<'_>) -> Vec<Problem> {
    if input.grammar.beats.is_none() {
        return Vec::new();
    }
    let mut found = Vec::new();
    for beat in input.body.beats() {
        for (i, attribute) in beat.attributes.iter().enumerate() {
            if beat.attributes[..i].iter().any(|a| a.key == attribute.key) {
                found.push(problem(
                    attribute.key_span,
                    format!("`{}` appears twice in beat {}", attribute.key, beat.number),
                ));
            }
        }
    }
    found
}
