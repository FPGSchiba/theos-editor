//! `formalism/source`: `_source.json` is not valid JSON or does not have the format.

use crate::finding::{Finding, Severity};
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;
use crate::text::{Position, Span};

/// The rule: the source file is not valid.
pub(crate) const FORMALISM_SOURCE: LoreRule = LoreRule {
    id: "formalism/source",
    severity: Severity::Error,
    class: RuleClass::Fixed,
    on: On::Input(crate::formalism::SOURCE),
    run: check,
};

/// The problem of the source, at `serde_json`'s position or 1:1.
fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Some(source) = ctx.formalism()? else {
        return Ok(Vec::new());
    };
    let Err(error) = source.as_ref() else {
        return Ok(Vec::new());
    };
    let position = error.position.unwrap_or(Position { line: 1, column: 1 });
    Ok(vec![subject.finding(
        &FORMALISM_SOURCE,
        Span::default(),
        position,
        error.message.clone(),
    )])
}
