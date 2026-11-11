//! `story/reveal-where`: a beat reveals a clue where the clue can be found (spec §5.9).

use crate::finding::{Finding, Severity};
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;

use super::{beat_at_place, beat_links, clue_records, inside_any, places_in, single_beats};

/// The rule: the `at::` place of a beat lies inside none of the valid `where` places of a clue
/// that the beat reveals. A beat without `at::` and a clue without a valid `where` are skipped.
pub(crate) const REVEAL_WHERE: LoreRule = LoreRule {
    id: "story/reveal-where",
    severity: Severity::Error,
    class: RuleClass::Fixed,
    on: On::Kinds(&["script"]),
    run: check,
};

fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(script) = subject else {
        return Ok(Vec::new());
    };
    let mut findings = Vec::new();
    for beat in single_beats(script) {
        let Some(at) = beat_at_place(ctx, script, beat.number)? else {
            continue;
        };
        for link in beat_links(script, beat.number, "reveals") {
            let mut missed: Option<Vec<String>> = None;
            for clue in clue_records(ctx, &link.target)? {
                let wanted = places_in(ctx, &clue, "where")?;
                if !wanted.is_empty() && !inside_any(ctx, &at, &wanted)? {
                    missed = Some(wanted);
                    break;
                }
            }
            if let Some(wanted) = missed {
                let message = format!(
                    "Beat {} reveals {} at {at}, but {} can be found only in {}. Move the beat inside one of those places or change `where` of {}.",
                    beat.number,
                    link.target,
                    link.target,
                    wanted.join(", "),
                    link.target
                );
                findings.push(subject.finding(&REVEAL_WHERE, link.span, link.position, message));
            }
        }
    }
    Ok(findings)
}
