//! `formalism/missing`: an equation of the source without its record.

use crate::finding::{Finding, Severity};
use crate::formalism::FOLDER;
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;
use crate::text::{Position, Span};

/// The rule: an equation of `_source.json` has no record.
pub(crate) const FORMALISM_MISSING: LoreRule = LoreRule {
    id: "formalism/missing",
    severity: Severity::Error,
    class: RuleClass::Fixed,
    on: On::Input(crate::formalism::SOURCE),
    run: check,
};

/// One finding per listed equation that no record has the id of, at 1:1 of the source.
fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Some(source) = ctx.formalism()? else {
        return Ok(Vec::new());
    };
    let Ok(source) = source.as_ref() else {
        return Ok(Vec::new());
    };
    let mut findings = Vec::new();
    for equation in &source.equations {
        if ctx.records(&equation.id)?.is_empty() {
            findings.push(subject.finding(
                &FORMALISM_MISSING,
                Span::default(),
                Position { line: 1, column: 1 },
                format!(
                    "the equation {} has no record; run `theos formalism` to write {FOLDER}/{}.md",
                    equation.id, equation.id
                ),
            ));
        }
    }
    Ok(findings)
}
