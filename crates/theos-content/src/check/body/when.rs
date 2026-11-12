//! `body/when`: the value of `when::` is `T1`, `T2` or `T3`.

use super::{BodyInput, BodyRule, problem};
use crate::finding::Severity;
use crate::kinds::{Problem, TIERS};

/// The rule.
pub(super) const RULE: BodyRule = BodyRule {
    id: "body/when",
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
        .filter(|attribute| attribute.key == "when")
        .filter_map(|attribute| {
            let value = attribute.value.spelling();
            (!TIERS.contains(&value.as_str())).then(|| {
                problem(
                    attribute.value_span,
                    format!("`{value}` is not a tier; use T1, T2 or T3"),
                )
            })
        })
        .collect()
}
