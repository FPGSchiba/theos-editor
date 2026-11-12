//! `formalism/stray`: an equation record that the source does not list.

use crate::finding::{Finding, Severity};
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;
use crate::text::{Position, Span};

/// The rule: an equation record without an equation in the source.
pub(crate) const FORMALISM_STRAY: LoreRule = LoreRule {
    id: "formalism/stray",
    severity: Severity::Error,
    class: RuleClass::Fixed,
    on: On::Kinds(&["equation"]),
    run: check,
};

/// Any equation record is stray when the project has no source. A source that is invalid lists
/// nothing we can trust: `formalism/source` reports it, and no record is called stray.
fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(record) = subject else {
        return Ok(Vec::new());
    };
    // The record reads itself (spec §5.14).
    ctx.records(&record.id)?;
    let listed = match ctx.formalism()? {
        None => false,
        Some(source) => match source.as_ref() {
            Ok(source) => source.equations.iter().any(|e| e.id == record.id),
            Err(_) => return Ok(Vec::new()),
        },
    };
    if listed {
        return Ok(Vec::new());
    }
    let (span, position) = record
        .value_at("id")
        .unwrap_or((Span::default(), Position { line: 1, column: 1 }));
    Ok(vec![subject.finding(
        &FORMALISM_STRAY,
        span,
        position,
        format!(
            "the equation {} is not in canon/formalism/_source.json",
            record.id
        ),
    )])
}
