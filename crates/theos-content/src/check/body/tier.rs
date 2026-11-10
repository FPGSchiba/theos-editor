//! `body/tier`: the tier of a voice title is `T1`, `T2` or `T3`.

use super::{BodyInput, BodyRule, problem};
use crate::body::CalloutTitle;
use crate::finding::Severity;
use crate::kinds::{Problem, TIERS};

/// The rule.
pub(super) const RULE: BodyRule = BodyRule {
    id: "body/tier",
    severity: Severity::Error,
    run,
};

fn run(input: &BodyInput<'_>) -> Vec<Problem> {
    input
        .body
        .callouts()
        .iter()
        .filter_map(|callout| match &callout.title {
            Some(CalloutTitle::Voice(voice)) => voice.tier.as_ref(),
            _ => None,
        })
        .filter(|tier| !TIERS.contains(&tier.text.as_str()))
        .map(|tier| {
            problem(
                tier.span,
                format!("`{}` is not a tier; use T1, T2 or T3", tier.text),
            )
        })
        .collect()
}
