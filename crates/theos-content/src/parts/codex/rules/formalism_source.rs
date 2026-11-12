//! `formalism/source`: `_source.json` is not valid JSON or does not have the format.

use crate::finding::{Finding, Severity};
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;
use crate::text::Position;

/// The rule: the source file is not valid.
pub(crate) const FORMALISM_SOURCE: LoreRule = LoreRule {
    id: "formalism/source",
    severity: Severity::Error,
    class: RuleClass::Fixed,
    on: On::Input(crate::formalism::SOURCE),
    run: check,
};

/// 1:1, where the source gives no position.
const START: Position = Position { line: 1, column: 1 };

/// The problem of the source, at the span and position of its error: `serde_json`'s place, or
/// 0..0 at 1:1 where it gives none (a format error found after parsing, a source that is not
/// UTF-8).
fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Some(source) = ctx.formalism()? else {
        return Ok(Vec::new());
    };
    let Err(error) = source.as_ref() else {
        return Ok(Vec::new());
    };
    let span = error.span.unwrap_or_default();
    let position = error.position.unwrap_or(START);
    Ok(vec![subject.finding(
        &FORMALISM_SOURCE,
        span,
        position,
        error.message.clone(),
    )])
}
