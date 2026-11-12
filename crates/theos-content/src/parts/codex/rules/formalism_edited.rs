//! `formalism/edited`: an equation record's bytes differ from what the source renders (CE-16.3).

use crate::finding::{Finding, Severity};
use crate::formalism::render_equation;
use crate::index::sha256_hex;
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;
use crate::text::{Position, Span};

/// The rule: a generated record was edited by hand.
pub(crate) const FORMALISM_EDITED: LoreRule = LoreRule {
    id: "formalism/edited",
    severity: Severity::Error,
    class: RuleClass::Fixed,
    on: On::Kinds(&["equation"]),
    run: check,
};

/// Compares the SHA-256 of the rendering with the stored hash of the file. A record the source
/// does not list, or a source that is missing or invalid, gives no finding here: `stray` and
/// `source` report those.
fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(record) = subject else {
        return Ok(Vec::new());
    };
    // The record reads itself (spec §5.14).
    ctx.records(&record.id)?;
    let Some(source) = ctx.formalism()? else {
        return Ok(Vec::new());
    };
    let Ok(source) = source.as_ref() else {
        return Ok(Vec::new());
    };
    let Some(equation) = source.equations.iter().find(|e| e.id == record.id) else {
        return Ok(Vec::new());
    };
    // `parse` renders every equation, so a source that parsed renders: this cannot fail.
    let Ok(rendered) = render_equation(equation) else {
        return Ok(Vec::new());
    };
    if sha256_hex(rendered.as_bytes()) == record.hash {
        return Ok(Vec::new());
    }
    let (span, position) = record
        .value_at("generated")
        .unwrap_or((Span::default(), Position { line: 1, column: 1 }));
    Ok(vec![subject.finding(
        &FORMALISM_EDITED,
        span,
        position,
        format!(
            "the record {} differs from what canon/formalism/_source.json renders; run `theos formalism` to regenerate it",
            record.id
        ),
    )])
}
