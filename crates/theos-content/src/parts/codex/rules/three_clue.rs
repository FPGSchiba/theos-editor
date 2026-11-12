//! `lore/three-clue`: a truth has fewer than three clues that bring it to its target (spec §5.8).

use crate::finding::{Finding, Severity};
use crate::lore::availability::State;
use crate::lore::reveals::{clues_of, moves_to};
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;

/// The rule: a truth with target `hinted`, `suspected` or `confirmed` has fewer than three clues
/// whose `moves_to` is its target. Draft and canon clues count alike.
pub(crate) const THREE_CLUE: LoreRule = LoreRule {
    id: "lore/three-clue",
    severity: Severity::Warning,
    class: RuleClass::Configurable,
    on: On::Kinds(&["truth"]),
    run: check,
};

fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(truth) = subject else {
        return Ok(Vec::new());
    };
    let Some(target) = truth
        .text("target")
        .and_then(|value| value.text.as_deref())
        .and_then(State::parse)
    else {
        return Ok(Vec::new());
    };
    let Some((span, position)) = truth.value_at("target") else {
        return Ok(Vec::new());
    };
    let count = clues_of(ctx, &truth.id)?
        .iter()
        .filter(|clue| moves_to(clue) == Some(target))
        .count();
    let (start, verb) = match count {
        0 => ("No clue", "moves"),
        1 => ("Only one clue", "moves"),
        2 => ("Only two clues", "move"),
        _ => return Ok(Vec::new()),
    };
    let message = format!(
        "{start} {verb} {} to {}. The three-clue rule asks for three.",
        truth.id,
        target.as_str()
    );
    Ok(vec![subject.finding(&THREE_CLUE, span, position, message)])
}
