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
    // The clues that bring this truth to its target, in order; read once, for the first item.
    let mut bringing: Option<Vec<Placed>> = None;
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
                "{} requires {}, whose target is {target}, so {} can never be confirmed. Set the target of {} to `confirmed` or drop it from `requires`.",
                truth.id, first.id, first.id, first.id
            ))
        } else if let Some(state) = own_target(truth) {
            if bringing.is_none() {
                bringing = Some(bringing_clues(ctx, &truth.id, state)?);
            }
            late_clue(ctx, bringing.as_deref().unwrap_or_default(), &first.id)?.map(|clue| {
                format!(
                    "{} requires {}, but no clue that confirms {} can be found no later than {} ({}), which brings {} to its target. Move a clue that confirms {} earlier or {} later.",
                    truth.id,
                    first.id,
                    first.id,
                    clue.id,
                    clue.at.describe(),
                    truth.id,
                    first.id,
                    clue.id
                )
            })
        } else {
            None
        };
        if let Some(message) = message {
            findings.push(subject.finding(&REQUIRES, link.span, link.position, message));
        }
        if cycle == Some(index) {
            let message = format!(
                "{} is on a cycle of `requires`: it requires itself, through {}. Remove one link of the cycle.",
                truth.id, link.target
            );
            findings.push(subject.finding(&REQUIRES, link.span, link.position, message));
        }
    }
    Ok(findings)
}

/// The valid target of the truth itself; `never` and a missing or invalid value give none.
fn own_target(truth: &RecordFacts) -> Option<State> {
    truth
        .text("target")
        .and_then(|value| value.text.as_deref())
        .and_then(State::parse)
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

/// The clues of the truth `id` whose `moves_to` is `target`, ordered by floor, gate and id.
fn bringing_clues(
    ctx: &mut Context<'_>,
    id: &str,
    target: State,
) -> Result<Vec<Placed>, FactError> {
    let mut bringing = Vec::new();
    for clue in clues_of(ctx, id)? {
        if moves_to(&clue) == Some(target) {
            bringing.push(Placed {
                at: of_clue(ctx, &clue)?,
                id: clue.id.clone(),
            });
        }
    }
    bringing.sort_by(by_availability);
    Ok(bringing)
}

/// The first of the `bringing` clues that no clue confirming `required` can be found no later
/// than. A clue whose availability is unknown next to a confirming clue is skipped.
fn late_clue<'a>(
    ctx: &mut Context<'_>,
    bringing: &'a [Placed],
    required: &str,
) -> Result<Option<&'a Placed>, FactError> {
    let mut confirming = Vec::new();
    for clue in clues_of(ctx, required)? {
        if moves_to(&clue) == Some(State::Confirmed) {
            confirming.push(of_clue(ctx, &clue)?);
        }
    }
    Ok(bringing.iter().find(|clue| {
        let verdicts: Vec<Option<bool>> = confirming
            .iter()
            .map(|early| no_later(early, &clue.at))
            .collect();
        !verdicts.contains(&Some(true)) && !verdicts.contains(&None)
    }))
}
