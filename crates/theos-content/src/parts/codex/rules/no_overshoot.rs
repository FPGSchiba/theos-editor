//! `lore/no-overshoot`: a clue does not move a truth past its `target` (spec §5.8).

use crate::finding::{Finding, Severity};
use crate::lore::availability::State;
use crate::lore::reveals::{Breach, judge, moves_to};
use crate::lore::{Context, FactError, LoreRule, On, RecordFacts, Subject};
use crate::rules::RuleClass;

/// The rule: a clue, or a beat that places one, moves a truth past its target.
pub(crate) const NO_OVERSHOOT: LoreRule = LoreRule {
    id: "lore/no-overshoot",
    severity: Severity::Error,
    class: RuleClass::Guardrail,
    on: On::Kinds(&["clue", "script"]),
    run: check,
};

fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    judge(ctx, subject, &NO_OVERSHOOT, breach)
}

/// Judges the targets `hinted` and `suspected` only: `confirmed` cannot be passed and `never` is
/// the other rule's. A clue or truth without a valid state is skipped. The message says what to
/// change (spec §5.8), naming the clue, so that it reads the same on the clue and on a beat.
fn breach(clue: &RecordFacts, truth: &RecordFacts) -> Option<Breach> {
    let state = moves_to(clue)?;
    let target = State::parse(truth.text("target")?.text.as_deref()?)?;
    if state <= target {
        return None;
    }
    Some(Breach {
        field: "moves_to",
        predicate: format!(
            "moves {truth} to {state}, past its target {target}. Lower the `moves_to` of {clue} \
             to {target} or raise the target of {truth}.",
            truth = truth.id,
            clue = clue.id,
            state = state.as_str(),
            target = target.as_str()
        ),
    })
}
