//! `lore/never-confirm`: nothing reveals a truth whose `target` is `never` (spec §4.1, §5.8).

use crate::finding::{Finding, Severity};
use crate::lore::reveals::{Breach, judge, verb};
use crate::lore::{Context, FactError, LoreRule, On, RecordFacts, Subject};
use crate::rules::RuleClass;

/// The rule: a clue, or a beat that places one, reveals a truth that must never be revealed.
pub(crate) const NEVER_CONFIRM: LoreRule = LoreRule {
    id: "lore/never-confirm",
    severity: Severity::Error,
    class: RuleClass::Guardrail,
    on: On::Kinds(&["clue", "script"]),
    run: check,
};

fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    judge(ctx, subject, &NEVER_CONFIRM, breach)
}

/// A truth with the target `never` is revealed by any clue. A truth without a valid target is
/// skipped: the finding on that fault exists already.
fn breach(clue: &RecordFacts, truth: &RecordFacts) -> Option<Breach> {
    if truth.text("target")?.text.as_deref() != Some("never") {
        return None;
    }
    Some(Breach {
        field: "reveals",
        predicate: format!(
            "{} {}, whose target is `never`: nothing may reveal it. Let a belief voice it instead.",
            verb(clue),
            truth.id
        ),
    })
}
