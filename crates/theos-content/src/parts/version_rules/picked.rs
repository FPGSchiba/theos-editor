//! `version/picked`: an item of `picked` names a version of another concept or map.

use crate::finding::{Finding, Severity};
use crate::index::Slot;
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;

/// The rule with the id and severity of the findings.
const RULE: LoreRule = picked(&[]);

/// The rule on the concepts or maps `on`: a picked version belongs to another parent.
pub(crate) const fn picked(on: &'static [&'static str]) -> LoreRule {
    LoreRule {
        id: "version/picked",
        severity: Severity::Error,
        class: RuleClass::Fixed,
        on: On::Kinds(on),
        run: check,
    }
}

/// Skips an item that names no version of the matching kind or whose version has no parent link.
fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(record) = subject else {
        return Ok(Vec::new());
    };
    let Some(kind) = record.kind.as_deref() else {
        return Ok(Vec::new());
    };
    let version_kind = format!("{kind}-version");
    let mut findings = Vec::new();
    for link in record.links_in(&Slot::Field("picked".to_owned())) {
        let Some(version) = super::record_of_kind(ctx, &link.target, &version_kind)? else {
            continue;
        };
        let Some(parent) = super::parent_id(&version, kind) else {
            continue;
        };
        if parent != record.id {
            findings.push(subject.finding(
                &RULE,
                link.span,
                link.position,
                format!(
                    "`{}` is a version of `{parent}`, not of `{}`",
                    link.target, record.id
                ),
            ));
        }
    }
    Ok(findings)
}
