//! Span-based text edits.

use super::span::Span;

/// Replace the bytes of `span` with `replacement` (an insert has an empty span, a delete an empty
/// replacement).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    /// The bytes to replace.
    pub span: Span,
    /// The text that takes their place.
    pub replacement: String,
}

/// Why a set of edits cannot be applied.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum EditError {
    /// The span reaches past the end of the text or ends before it starts.
    #[error("span {}..{} is out of range", span.start, span.end)]
    OutOfRange {
        /// The offending span.
        span: Span,
    },
    /// The span starts or ends inside a multi-byte character.
    #[error("span {}..{} is not on character boundaries", span.start, span.end)]
    NotCharBoundary {
        /// The offending span.
        span: Span,
    },
    /// Two edits touch the same bytes.
    #[error("span {}..{} overlaps {}..{}", second.start, second.end, first.start, first.end)]
    Overlap {
        /// The earlier span.
        first: Span,
        /// The span that starts before `first` ends.
        second: Span,
    },
}

/// Applies `edits` to `text`. The edits are sorted by `(start, end)` with a stable sort, so
/// zero-width inserts at one offset keep their order and come before a deletion that starts there.
///
/// # Errors
///
/// [`EditError::OutOfRange`], [`EditError::NotCharBoundary`] or [`EditError::Overlap`]; nothing
/// is applied then.
pub fn apply_edits(text: &str, edits: &[TextEdit]) -> Result<String, EditError> {
    let mut sorted: Vec<&TextEdit> = edits.iter().collect();
    sorted.sort_by_key(|edit| (edit.span.start, edit.span.end));

    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut previous: Option<Span> = None;
    for edit in sorted {
        let span = edit.span;
        if span.start > span.end || span.end > text.len() {
            return Err(EditError::OutOfRange { span });
        }
        if !text.is_char_boundary(span.start) || !text.is_char_boundary(span.end) {
            return Err(EditError::NotCharBoundary { span });
        }
        if let Some(first) = previous.filter(|first| span.start < first.end) {
            return Err(EditError::Overlap {
                first,
                second: span,
            });
        }
        out.push_str(&text[cursor..span.start]);
        out.push_str(&edit.replacement);
        cursor = span.end;
        previous = Some(span);
    }
    out.push_str(&text[cursor..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(start: usize, end: usize, replacement: &str) -> TextEdit {
        TextEdit {
            span: Span::new(start, end),
            replacement: replacement.to_string(),
        }
    }

    #[test]
    fn edits_apply_in_span_order() {
        let text = "0123456789";
        // Out of order: replace 6..8, insert at 0, delete 2..3.
        let edits = [edit(6, 8, "XY"), edit(0, 0, ">"), edit(2, 3, "")];
        assert_eq!(apply_edits(text, &edits).as_deref(), Ok(">01345XY89"));

        // An insert at 3 and a delete of 3..5 both apply; inserts keep their order.
        let edits = [edit(3, 5, ""), edit(3, 3, "a"), edit(3, 3, "b")];
        assert_eq!(apply_edits(text, &edits).as_deref(), Ok("012ab56789"));

        assert_eq!(apply_edits(text, &[]).as_deref(), Ok(text));

        assert_eq!(
            apply_edits(text, &[edit(1, 4, ""), edit(3, 5, "")]),
            Err(EditError::Overlap {
                first: Span::new(1, 4),
                second: Span::new(3, 5)
            })
        );

        assert_eq!(
            apply_edits("aèb", &[edit(2, 3, "")]),
            Err(EditError::NotCharBoundary {
                span: Span::new(2, 3)
            })
        );
        assert_eq!(
            apply_edits(text, &[edit(8, 11, "")]),
            Err(EditError::OutOfRange {
                span: Span::new(8, 11)
            })
        );
    }
}
