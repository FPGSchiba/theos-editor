//! `version/from`: `from` names a version of another concept or map.

use crate::finding::{Finding, Severity};
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;

/// The rule with the id and severity of the findings.
const RULE: LoreRule = from(&[]);

/// The rule on the versions `on`: the version `from` names has another parent.
pub(crate) const fn from(on: &'static [&'static str]) -> LoreRule {
    LoreRule {
        id: "version/from",
        severity: Severity::Error,
        class: RuleClass::Fixed,
        on: On::Kinds(on),
        run: check,
    }
}

/// Skips a version without valid parent or `from` links, and a `from` that names no version of
/// the same kind or one without a parent link.
fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(record) = subject else {
        return Ok(Vec::new());
    };
    let Some(kind) = record.kind.as_deref() else {
        return Ok(Vec::new());
    };
    let Some(parent_kind) = kind.strip_suffix("-version") else {
        return Ok(Vec::new());
    };
    let slot = crate::index::Slot::Field("from".to_owned());
    let Some(link) = record.links_in(&slot).next() else {
        return Ok(Vec::new());
    };
    let Some(own_parent) = super::parent_id(record, parent_kind) else {
        return Ok(Vec::new());
    };
    let Some(origin) = super::record_of_kind(ctx, &link.target, kind)? else {
        return Ok(Vec::new());
    };
    let Some(origin_parent) = super::parent_id(&origin, parent_kind) else {
        return Ok(Vec::new());
    };
    if origin_parent == own_parent {
        return Ok(Vec::new());
    }
    Ok(vec![subject.finding(
        &RULE,
        link.span,
        link.position,
        format!(
            "`{}` is a version of `{origin_parent}`, not of `{own_parent}`",
            link.target
        ),
    )])
}
