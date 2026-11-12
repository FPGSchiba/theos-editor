//! `story/reveal-gate`: a beat reveals a clue only once the player understands enough
//! (spec §5.9).

use crate::finding::{Finding, Severity};
use crate::lore::availability::Tier;
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;

use super::{beat_links, clue_records, single_beats};

/// The rule: the `when::` tier of a beat is below the valid `gate` of a clue that the beat
/// reveals. A beat without a valid `when::` and a clue without a valid gate are skipped.
pub(crate) const REVEAL_GATE: LoreRule = LoreRule {
    id: "story/reveal-gate",
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
        let Some(when) = beat.when.as_deref().and_then(Tier::parse) else {
            continue;
        };
        for link in beat_links(script, beat.number, "reveals") {
            // The highest gate above `when` among the clue records with this id.
            let mut gate: Option<Tier> = None;
            for clue in clue_records(ctx, &link.target)? {
                let tier = clue
                    .text("gate")
                    .and_then(|value| value.text.as_deref())
                    .and_then(Tier::parse);
                gate = gate.max(tier.filter(|tier| *tier > when));
            }
            if let Some(gate) = gate {
                let message = format!(
                    "Beat {} reveals {} at {}, but the gate of {} is {}. Raise `when::` of the beat to {} or lower the gate of {}.",
                    beat.number,
                    link.target,
                    when.as_str(),
                    link.target,
                    gate.as_str(),
                    gate.as_str(),
                    link.target
                );
                findings.push(subject.finding(&REVEAL_GATE, link.span, link.position, message));
            }
        }
    }
    Ok(findings)
}
