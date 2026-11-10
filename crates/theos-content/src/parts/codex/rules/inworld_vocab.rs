//! `lore/inworld-vocab`: a developer word in in-world text (spec §4.1, §5.11).

use crate::finding::{Finding, Severity};
use crate::lore::{Context, FactError, LoreRule, On, Segment, Subject};
use crate::rules::RuleClass;
use crate::text::{Position, Span};

/// The rule: each developer word in the in-world text of a record is a finding.
pub(crate) const INWORLD_VOCAB: LoreRule = LoreRule {
    id: "lore/inworld-vocab",
    severity: Severity::Error,
    class: RuleClass::Guardrail,
    on: On::AnyRecord,
    run: check,
};

fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(record) = subject else {
        return Ok(Vec::new());
    };
    // No segments: no settings read, so the record does not depend on the settings file.
    if record.inworld.is_empty() {
        return Ok(Vec::new());
    }
    let Some(words) = ctx.developer_words()? else {
        return Ok(Vec::new());
    };
    let mut findings = Vec::new();
    for segment in &record.inworld {
        for (range, word) in words.find(&segment.text) {
            let (span, position) = locate(segment, range.start, range.end);
            findings.push(subject.finding(
                &INWORLD_VOCAB,
                span,
                position,
                format!(
                    "`{word}` is a developer word, which in-world text cannot use. \
                     Rephrase it in the speaker's words, or add it to `allow` with a reason."
                ),
            ));
        }
    }
    Ok(findings)
}

/// The span of the bytes `start..end` of the segment's text in the record, and the position of
/// its start: the segment's position plus what lies between (L20, Unicode scalar values).
fn locate(segment: &Segment, start: usize, end: usize) -> (Span, Position) {
    let before = segment.text.get(..start).unwrap_or_default();
    let position = match before.rfind('\n') {
        None => Position {
            line: segment.position.line,
            column: segment.position.column + before.chars().count(),
        },
        Some(newline) => Position {
            line: segment.position.line + before.matches('\n').count(),
            column: before
                .get(newline + 1..)
                .unwrap_or_default()
                .chars()
                .count()
                + 1,
        },
    };
    let span = Span {
        start: segment.span.start + start,
        end: segment.span.start + end,
    };
    (span, position)
}
