//! `lore/reveal-order`: the order of reveals (spec §5.7, §5.8): `after`, the order of a truth's
//! states, and cycles of `after`.

use std::rc::Rc;

use crate::finding::{Finding, Severity};
use crate::index::Slot;
use crate::lore::availability::{no_later, of_clue};
use crate::lore::cycle::cycle_through;
use crate::lore::reveals::{clues_of, moves_to, state_verb, truths_of, verb};
use crate::lore::{Context, FactError, LoreRule, On, RecordFacts, Subject};
use crate::rules::RuleClass;

/// The rule: a clue in `after` is found later than this clue, a clue is found before the clues
/// of the state below it, or `after` leads in a cycle.
pub(crate) const REVEAL_ORDER: LoreRule = LoreRule {
    id: "lore/reveal-order",
    severity: Severity::Error,
    class: RuleClass::Configurable,
    on: On::Kinds(&["clue"]),
    run: check,
};

fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(clue) = subject else {
        return Ok(Vec::new());
    };
    let mut findings = after_findings(ctx, subject, clue)?;
    findings.extend(state_finding(ctx, subject, clue)?);
    Ok(findings)
}

/// (a) each `after` clue that cannot be found no later than this clue, and (c) the `after` item
/// that leads into a cycle.
fn after_findings(
    ctx: &mut Context<'_>,
    subject: &Subject,
    clue: &RecordFacts,
) -> Result<Vec<Finding>, FactError> {
    let slot = Slot::Field("after".to_owned());
    let mut findings = Vec::new();
    let mine = of_clue(ctx, clue)?;
    for link in clue.links_in(&slot) {
        for earlier in ctx.records(&link.target)? {
            if earlier.kind.as_deref() != Some("clue") {
                continue;
            }
            let theirs = of_clue(ctx, &earlier)?;
            if no_later(&theirs, &mine) == Some(false) {
                let message = format!(
                    "{} comes after {}, which cannot be found no later than {} ({}: {}; {}: {}). Move one of them or change `after`.",
                    clue.id,
                    earlier.id,
                    clue.id,
                    earlier.id,
                    theirs.describe(),
                    clue.id,
                    mine.describe()
                );
                findings.push(subject.finding(&REVEAL_ORDER, link.span, link.position, message));
                break;
            }
        }
    }
    if let Some(index) = cycle_through(ctx, clue, "after")?
        && let Some(link) = clue.links_in(&slot).nth(index)
    {
        let message = format!(
            "{} is on a cycle of `after`: it would come after itself, through {}. Remove one `after` link of the cycle.",
            clue.id, link.target
        );
        findings.push(subject.finding(&REVEAL_ORDER, link.span, link.position, message));
    }
    Ok(findings)
}

/// (b) the clue moves its truth to a state, the truth has clues of a lower state, and no clue of
/// the nearest such state can be found no later than this clue.
fn state_finding(
    ctx: &mut Context<'_>,
    subject: &Subject,
    clue: &RecordFacts,
) -> Result<Option<Finding>, FactError> {
    let (Some(state), Some((span, position))) = (moves_to(clue), clue.value_at("moves_to")) else {
        return Ok(None);
    };
    let Some(truth) = truths_of(ctx, clue)?.into_iter().next() else {
        return Ok(None);
    };
    let siblings = clues_of(ctx, &truth.id)?;
    let Some(lower) = siblings
        .iter()
        .filter_map(|other| moves_to(other))
        .filter(|other| *other < state)
        .max()
    else {
        return Ok(None);
    };
    let mine = of_clue(ctx, clue)?;
    let nearest: Vec<&Rc<RecordFacts>> = siblings
        .iter()
        .filter(|other| moves_to(other) == Some(lower))
        .collect();
    let mut unknown = false;
    for other in nearest {
        match no_later(&of_clue(ctx, other)?, &mine) {
            Some(true) => return Ok(None),
            Some(false) => {}
            None => unknown = true,
        }
    }
    if unknown {
        return Ok(None);
    }
    let lower_verb = state_verb(lower);
    let message = format!(
        "{} {} {}, but no clue that {lower_verb} it can be found no later than {} ({}). Move a clue of that state earlier or {} later.",
        clue.id,
        verb(clue),
        truth.id,
        clue.id,
        mine.describe(),
        clue.id
    );
    Ok(Some(subject.finding(
        &REVEAL_ORDER,
        span,
        position,
        message,
    )))
}
