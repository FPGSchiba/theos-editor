//! Who reveals what (spec §5.8): the truths a clue reveals, the clues that reveal a truth, and
//! the one driver of the two guardrails that judge a clue and the beats that place it.

use std::collections::BTreeSet;
use std::rc::Rc;

use crate::finding::Finding;
use crate::index::Slot;
use crate::lore::availability::State;
use crate::lore::{Context, FactError, LoreRule, RecordFacts, Subject};

/// The records of kind `truth` that the `reveals` link of `clue` names (more than one when the
/// id is duplicated). Empty when the link is missing, invalid or dangling.
///
/// # Errors
///
/// Fails when the store fails.
pub fn truths_of(
    ctx: &mut Context<'_>,
    clue: &RecordFacts,
) -> Result<Vec<Rc<RecordFacts>>, FactError> {
    let reveals = Slot::Field("reveals".to_owned());
    let Some(link) = clue.links_in(&reveals).next() else {
        return Ok(Vec::new());
    };
    Ok(ctx
        .records(&link.target)?
        .into_iter()
        .filter(|record| record.kind.as_deref() == Some("truth"))
        .collect())
}

/// The records of kind `clue` whose `reveals` link names `truth_id`, one per path.
///
/// # Errors
///
/// Fails when the store fails.
pub fn clues_of(ctx: &mut Context<'_>, truth_id: &str) -> Result<Vec<Rc<RecordFacts>>, FactError> {
    let reveals = Slot::Field("reveals".to_owned());
    let mut seen = BTreeSet::new();
    let mut clues = Vec::new();
    for link in ctx.linking_to(truth_id)? {
        if link.slot != reveals || !seen.insert(link.source_path.clone()) {
            continue;
        }
        let found = ctx.records(&link.source_id)?;
        clues.extend(found.into_iter().find(|record| {
            record.path == link.source_path && record.kind.as_deref() == Some("clue")
        }));
    }
    Ok(clues)
}

/// What a clue does to a truth that a guardrail objects to: the field of the clue it sits at and
/// what is wrong, as a predicate of the clue ("hints at T-1, whose …").
pub(crate) struct Breach {
    /// The front-matter field of the clue the finding sits at.
    pub field: &'static str,
    /// The predicate that follows the clue's id in the message.
    pub predicate: String,
}

/// The `moves_to` state of a clue, when valid.
pub(crate) fn moves_to(clue: &RecordFacts) -> Option<State> {
    clue.text("moves_to")
        .and_then(|value| value.text.as_deref())
        .and_then(State::parse)
}

/// How a clue's `moves_to` reads in a message: "hints at", "raises suspicion of", "confirms".
pub(crate) fn verb(clue: &RecordFacts) -> &'static str {
    match moves_to(clue) {
        Some(State::Hinted) => "hints at",
        Some(State::Suspected) => "raises suspicion of",
        Some(State::Confirmed) => "confirms",
        None => "reveals",
    }
}

/// Judges one clue against one truth.
pub(crate) type Judge = fn(&RecordFacts, &RecordFacts) -> Option<Breach>;

/// Runs a guardrail on its subject: a clue is judged against the truth its `reveals` names, a
/// script on each beat's `reveals::` link to a clue. A duplicated truth id breaks the guardrail
/// when any record with it does.
pub(crate) fn judge(
    ctx: &mut Context<'_>,
    subject: &Subject,
    rule: &LoreRule,
    breach: Judge,
) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(record) = subject else {
        return Ok(Vec::new());
    };
    let mut findings = Vec::new();
    match record.kind.as_deref() {
        Some("clue") => {
            if let Some(breach) = breach_of(ctx, record, breach)?
                && let Some((span, position)) = record.value_at(breach.field)
            {
                let message = format!("{} {}", record.id, breach.predicate);
                findings.push(subject.finding(rule, span, position, message));
            }
        }
        Some("script") => {
            let beat = Slot::Beat("reveals".to_owned());
            for link in record.links_in(&beat) {
                let Some(number) = link.beat else { continue };
                let found = ctx.records(&link.target)?;
                let Some(clue) = found.iter().find(|c| c.kind.as_deref() == Some("clue")) else {
                    continue;
                };
                if let Some(breach) = breach_of(ctx, clue, breach)? {
                    let message = format!(
                        "Beat {number} places {}, which {}",
                        clue.id, breach.predicate
                    );
                    findings.push(subject.finding(rule, link.span, link.position, message));
                }
            }
        }
        _ => {}
    }
    Ok(findings)
}

/// The first breach of `clue` against the records of its truth id.
fn breach_of(
    ctx: &mut Context<'_>,
    clue: &RecordFacts,
    breach: Judge,
) -> Result<Option<Breach>, FactError> {
    Ok(truths_of(ctx, clue)?
        .iter()
        .find_map(|truth| breach(clue, truth)))
}
