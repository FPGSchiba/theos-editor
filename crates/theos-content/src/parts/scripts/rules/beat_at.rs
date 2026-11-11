//! `story/beat-at`: a beat happens inside the places the script says it covers (spec §5.9).

use crate::finding::{Finding, Severity};
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;

use super::{beat_at_place, inside_any, places_in, single_beats};

/// The rule: the `at::` place of a beat lies inside none of the script's `where` places. A script
/// without a valid `where`, and a beat without `at::`, are not checked.
pub(crate) const BEAT_AT: LoreRule = LoreRule {
    id: "story/beat-at",
    severity: Severity::Error,
    class: RuleClass::Fixed,
    on: On::Kinds(&["script"]),
    run: check,
};

fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(script) = subject else {
        return Ok(Vec::new());
    };
    let wanted = places_in(ctx, script, "where")?;
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let mut findings = Vec::new();
    for beat in single_beats(script) {
        let Some(at) = beat_at_place(ctx, script, beat.number)? else {
            continue;
        };
        if inside_any(ctx, &at, &wanted)? {
            continue;
        }
        let Some(link) = super::beat_links(script, beat.number, "at").next() else {
            continue;
        };
        let message = format!(
            "Beat {} is at {at}, which lies inside none of the places in `where` of {} ({}). Move the beat to one of them or add a place that holds {at} to `where`.",
            beat.number,
            script.id,
            wanted.join(", ")
        );
        findings.push(subject.finding(&BEAT_AT, link.span, link.position, message));
    }
    Ok(findings)
}
