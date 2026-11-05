//! Splitting an LF record into front matter and body.

use super::error::RecordError;
use crate::text::Span;

const DELIMITER: &str = "---";

/// Returns `(front, body)`: the lines between the two `---` lines and everything after the
/// closing line. The closing line is the first line after the opening one that is exactly `---`.
pub(super) fn split(text: &str) -> Result<(Span, Span), RecordError> {
    let mut lines = text.split_inclusive('\n');
    let first = lines.next().ok_or(RecordError::NoOpeningLine)?;
    if first.strip_suffix('\n').unwrap_or(first) != DELIMITER {
        return Err(RecordError::NoOpeningLine);
    }
    let front_start = first.len();
    let mut offset = front_start;
    for line in lines {
        let end = offset + line.len();
        if line.strip_suffix('\n').unwrap_or(line) == DELIMITER {
            return Ok((Span::new(front_start, offset), Span::new(end, text.len())));
        }
        offset = end;
    }
    Err(RecordError::NoClosingLine)
}
