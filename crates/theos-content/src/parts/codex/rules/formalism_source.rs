//! `formalism/source`: `_source.json` is not valid JSON or does not have the format.

use crate::finding::{Finding, Severity};
use crate::lore::{Context, FactError, LoreRule, On, Subject};
use crate::rules::RuleClass;
use crate::text::{Position, Span, TextIndex};

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

/// The problem of the source: an empty span at `serde_json`'s place, with the position of that
/// span's start, or 0..0 at 1:1 where `serde_json` gives none (a format error found after
/// parsing, a source that is not UTF-8).
fn check(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Some(source) = ctx.formalism()? else {
        return Ok(Vec::new());
    };
    let Err(error) = source.as_ref() else {
        return Ok(Vec::new());
    };
    let placed = match (error.position, subject) {
        (Some(reported), Subject::Input(input)) => input.text.as_ref().map(|text| {
            let lines = TextIndex::new(text.clone());
            let offset = offset_of(&lines, reported);
            (
                Span::new(offset, offset),
                lines.position(offset).unwrap_or(START),
            )
        }),
        _ => None,
    };
    let (span, position) = placed.unwrap_or((Span::default(), START));
    Ok(vec![subject.finding(
        &FORMALISM_SOURCE,
        span,
        position,
        error.message.clone(),
    )])
}

/// The byte offset of `serde_json`'s line and byte column (from 1, counted in bytes of the
/// line): the start of the line plus the column minus one, clamped to the line's content and
/// moved back to a character boundary. The stored text is not normalized, so a CR before an
/// LF stays in the content; `serde_json` counts lines by LF as `TextIndex` does.
fn offset_of(lines: &TextIndex, reported: Position) -> usize {
    let text = lines.text();
    let Some(line) = lines.line_span(reported.line) else {
        return text.len();
    };
    let content = line.get(text).unwrap_or_default();
    let content = content.strip_suffix('\n').unwrap_or(content);
    let mut within = reported.column.saturating_sub(1).min(content.len());
    while !content.is_char_boundary(within) {
        within -= 1;
    }
    line.start + within
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str, line: usize, column: usize) -> usize {
        offset_of(&TextIndex::new(text.to_owned()), Position { line, column })
    }

    #[test]
    fn a_byte_column_becomes_an_offset_past_non_ascii_characters() {
        // `é` is 2 bytes and `😀` is 4: byte column 7 is the `x` after them.
        let text = "ab\n\u{e9}\u{1f600}x\n";
        assert_eq!(at(text, 2, 7), 3 + 6);
        assert_eq!(&text[at(text, 2, 7)..][..1], "x");
    }

    #[test]
    fn a_column_inside_a_character_moves_back_to_its_start() {
        let text = "\u{1f600}x";
        assert_eq!(at(text, 1, 3), 0);
        assert_eq!(at(text, 1, 5), 4);
    }

    #[test]
    fn a_column_past_the_line_or_zero_is_clamped() {
        let text = "ab\ncd";
        assert_eq!(at(text, 1, 40), 2, "before the LF");
        assert_eq!(at(text, 2, 40), 5, "the end of the last line");
        assert_eq!(at(text, 2, 0), 3);
        assert_eq!(at(text, 9, 1), text.len(), "no such line");
    }

    #[test]
    fn the_end_after_a_final_newline_is_the_end_of_the_text() {
        let text = "ab\n";
        assert_eq!(at(text, 2, 1), 3);
        assert_eq!(at(text, 2, 0), 3);
    }
}
