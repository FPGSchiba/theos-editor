//! `body/attribute-count`: `at::` holds one link.

use super::{BodyInput, BodyRule, problem};
use crate::body::AttributeValue;
use crate::finding::Severity;
use crate::kinds::Problem;

/// The rule.
pub(super) const RULE: BodyRule = BodyRule {
    id: "body/attribute-count",
    severity: Severity::Error,
    run,
};

fn run(input: &BodyInput<'_>) -> Vec<Problem> {
    if input.grammar.beats.is_none() {
        return Vec::new();
    }
    input
        .body
        .beats()
        .iter()
        .flat_map(|beat| &beat.attributes)
        .filter(|attribute| attribute.key == "at")
        .filter_map(|attribute| match &attribute.value {
            AttributeValue::Links(links) => links.get(1),
            _ => None,
        })
        .map(|second| problem(second.span, "`at` holds one link"))
        .collect()
}
