//! `link/parent-cycle`: the `parent` chain of a record leads back to it.

use crate::finding::{Finding, Severity};
use crate::lore::cycle::cycle_through;
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;

/// The rule: a record's `parent` chain leads back to the record. Only the records on the cycle
/// carry it.
pub(crate) const PARENT_CYCLE: LoreRule = LoreRule {
    id: "link/parent-cycle",
    severity: Severity::Error,
    class: RuleClass::Fixed,
    on: On::Kinds(&["band", "floor", "location", "faction", "npc"]),
    run: check,
};

fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(record) = subject else {
        return Ok(Vec::new());
    };
    let Some(index) = cycle_through(ctx, record, "parent")? else {
        return Ok(Vec::new());
    };
    let slot = crate::index::Slot::Field("parent".to_owned());
    let Some(link) = record.links_in(&slot).nth(index) else {
        return Ok(Vec::new());
    };
    Ok(vec![subject.finding(
        &PARENT_CYCLE,
        link.span,
        link.position,
        format!("the parent chain of `{}` leads back to it", record.id),
    )])
}
