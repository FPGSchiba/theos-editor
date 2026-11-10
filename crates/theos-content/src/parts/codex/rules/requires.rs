//! `lore/requires`: a truth that requires another one can have it confirmed first (spec §5.8).

use std::cmp::Ordering;
use std::rc::Rc;

use crate::finding::{Finding, Severity};
use crate::index::Slot;
use crate::lore::availability::{Availability, State, no_later, of_clue};
use crate::lore::cycle::cycle_through;
use crate::lore::reveals::{clues_of, moves_to};
use crate::lore::{Context, FactError, LoreRule, On, RecordFacts, Subject};
use crate::rules::RuleClass;

/// The rule: for each `Y` in a truth `X`'s `requires`, `Y` has a target other than `confirmed`,
/// no clue that confirms `Y` can be found no later than a clue that brings `X` to its target, or
/// `X` is on a cycle of `requires`.
pub(crate) const REQUIRES: LoreRule = LoreRule {
    id: "lore/requires",
    severity: Severity::Error,
    class: RuleClass::Configurable,
    on: On::Kinds(&["truth"]),
    run: check,
};

fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(truth) = subject else {
        return Ok(Vec::new());
    };
    let slot = Slot::Field("requires".to_owned());
    let mut findings = Vec::new();
    let cycle = cycle_through(ctx, truth, "requires")?;
    for (index, link) in truth.links_in(&slot).enumerate() {
        let required: Vec<Rc<RecordFacts>> = ctx
            .records(&link.target)?
            .into_iter()
            .filter(|record| record.kind.as_deref() == Some("truth"))
            .collect();
        let Some(first) = required.first() else {
            continue;
        };
        let not_confirmed = required.iter().find_map(|record| {
            let target = record.text("target")?.text.clone()?;
            (target != "confirmed").then_some(target)
        });
        let message = if let Some(target) = not_confirmed {
            Some(format!(
                "{} requires {}, whose target is {target}: {} can never be confirmed.",
                truth.id, first.id, first.id
            ))
        } else {
            late_clue(ctx, &truth.id, &first.id)?.map(|clue| {
                format!(
                    "{} requires {}, but no clue that confirms {} can be found no later than {clue}, which brings {} to its target.",
                    truth.id, first.id, first.id, truth.id
                )
            })
        };
        if let Some(message) = message {
            findings.push(subject.finding(&REQUIRES, link.span, link.position, message));
        }
        if cycle == Some(index) {
            let message = format!(
                "{} is on a cycle of `requires`: it requires itself, through {}.",
                truth.id, link.target
            );
            findings.push(subject.finding(&REQUIRES, link.span, link.position, message));
        }
    }
    Ok(findings)
}

/// A clue with its availability.
struct Placed {
    id: String,
    at: Availability,
}

/// Orders by floor (unknown last), gate (unknown last), then id.
fn by_availability(a: &Placed, b: &Placed) -> Ordering {
    let floor = |p: &Placed| p.at.floor.as_ref().map_or(u32::MAX, |floor| floor.number);
    floor(a)
        .cmp(&floor(b))
        .then_with(|| match (a.at.gate, b.at.gate) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        })
        .then_with(|| a.id.cmp(&b.id))
}

/// The id of the first clue, by floor, gate and id, that brings a truth with the id `required_by`
/// to its target while no clue that confirms `required` can be found no later than it. A clue
/// whose availability is unknown next to a confirming clue is skipped.
fn late_clue(
    ctx: &mut Context<'_>,
    required_by: &str,
    required: &str,
) -> Result<Option<String>, FactError> {
    let same_id = ctx.records(required_by)?;
    let targets: Vec<State> = same_id
        .iter()
        .filter_map(|record| {
            record
                .text("target")?
                .text
                .as_deref()
                .and_then(State::parse)
        })
        .collect();
    let mut confirming = Vec::new();
    for clue in clues_of(ctx, required)? {
        if moves_to(&clue) == Some(State::Confirmed) {
            confirming.push(of_clue(ctx, &clue)?);
        }
    }
    let mut bringing = Vec::new();
    for clue in clues_of(ctx, required_by)? {
        if moves_to(&clue).is_some_and(|state| targets.contains(&state)) {
            bringing.push(Placed {
                at: of_clue(ctx, &clue)?,
                id: clue.id.clone(),
            });
        }
    }
    bringing.sort_by(by_availability);
    Ok(bringing
        .into_iter()
        .find(|clue| {
            let verdicts: Vec<Option<bool>> = confirming
                .iter()
                .map(|early| no_later(early, &clue.at))
                .collect();
            !verdicts.contains(&Some(true)) && !verdicts.contains(&None)
        })
        .map(|clue| clue.id))
}
