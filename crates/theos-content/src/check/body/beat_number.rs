//! `body/beat-number`: the beats of a record have different numbers.

use super::{BodyInput, BodyRule, problem};
use crate::finding::Severity;
use crate::kinds::Problem;

/// The rule.
pub(super) const RULE: BodyRule = BodyRule {
    id: "body/beat-number",
    severity: Severity::Error,
    run,
};

fn run(input: &BodyInput<'_>) -> Vec<Problem> {
    if input.grammar.beats.is_none() {
        return Vec::new();
    }
    let beats = input.body.beats();
    beats
        .iter()
        .enumerate()
        .filter(|(i, beat)| beats[..*i].iter().any(|b| b.number == beat.number))
        .map(|(_, beat)| {
            problem(
                beat.heading_span,
                format!("beat {} is numbered twice", beat.number),
            )
        })
        .collect()
}
