//! `story/says-rejected`: a belief that every holder in the beat rejects cannot be said there
//! (spec §5.9).

use crate::finding::{Finding, Severity};
use crate::lore::lens::rejects;
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;

use super::beat_links;

/// The rule: a `says::` belief that the effective `rejects` of every `with::` holder of the beat
/// contains. A beat without a `with::` holder is not checked; one holder that does not reject the
/// belief is enough.
pub(crate) const SAYS_REJECTED: LoreRule = LoreRule {
    id: "story/says-rejected",
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
    for beat in &script.beats {
        let holders: Vec<&str> = beat_links(script, beat.number, "with")
            .map(|link| link.target.as_str())
            .collect();
        if holders.is_empty() {
            continue;
        }
        let mut lenses = Vec::new();
        for holder in &holders {
            lenses.push(rejects(ctx, holder)?.unwrap_or_default());
        }
        for says in beat_links(script, beat.number, "says") {
            if !lenses
                .iter()
                .all(|rejected| rejected.contains(&says.target))
            {
                continue;
            }
            let who = if holders.len() == 1 {
                "its lens rejects it".to_owned()
            } else {
                "every lens rejects it".to_owned()
            };
            let message = format!(
                "Beat {} has {} say {}, but {who}. Add a holder to `with::` whose lens does not reject {}, or let `says::` name another belief.",
                beat.number,
                holders.join(", "),
                says.target,
                says.target
            );
            findings.push(subject.finding(&SAYS_REJECTED, says.span, says.position, message));
        }
    }
    Ok(findings)
}
