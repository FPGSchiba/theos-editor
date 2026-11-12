//! `body/attribute-unknown`: a beat key the kind does not have (a warning).

use super::{BodyInput, BodyRule, problem};
use crate::finding::Severity;
use crate::kinds::Problem;

/// The rule.
pub(super) const RULE: BodyRule = BodyRule {
    id: "body/attribute-unknown",
    severity: Severity::Warning,
    run,
};

fn run(input: &BodyInput<'_>) -> Vec<Problem> {
    let Some(links) = input.grammar.beats else {
        return Vec::new();
    };
    let known =
        |key: &str| links.iter().any(|a| a.key == key) || input.grammar.beat_tokens.contains(&key);
    input
        .body
        .beats()
        .iter()
        .flat_map(|beat| &beat.attributes)
        .filter(|attribute| !known(&attribute.key))
        .map(|attribute| {
            problem(
                attribute.key_span,
                format!("`{}` is not an attribute of a beat", attribute.key),
            )
        })
        .collect()
}
