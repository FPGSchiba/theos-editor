//! `version/addresses`: an item of `addresses` names a note the parent does not have (CE-15.5).

use crate::finding::{Finding, Severity};
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;

/// The rule with the id and severity of the findings.
const RULE: LoreRule = addresses(&[]);

/// The rule on the versions `on`: an addressed note is not among the notes of the parent.
pub(crate) const fn addresses(on: &'static [&'static str]) -> LoreRule {
    LoreRule {
        id: "version/addresses",
        severity: Severity::Error,
        class: RuleClass::Fixed,
        on: On::Kinds(on),
        run: check,
    }
}

/// Skips a version whose parent link is invalid or does not resolve to the parent kind: the
/// finding on that fault exists already.
fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(record) = subject else {
        return Ok(Vec::new());
    };
    let Some(parent_kind) = record
        .kind
        .as_deref()
        .and_then(|kind| kind.strip_suffix("-version"))
    else {
        return Ok(Vec::new());
    };
    let items = record.items("addresses");
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let Some(parent_id) = super::parent_id(record, parent_kind) else {
        return Ok(Vec::new());
    };
    let Some(parent) = super::record_of_kind(ctx, &parent_id, parent_kind)? else {
        return Ok(Vec::new());
    };
    let mut findings = Vec::new();
    for item in items {
        let Some(note) = item.text.as_deref() else {
            continue;
        };
        if parent.notes.iter().all(|have| have.id != note) {
            findings.push(subject.finding(
                &RULE,
                item.span,
                item.position,
                format!("`{parent_id}` has no note `{note}`"),
            ));
        }
    }
    Ok(findings)
}
