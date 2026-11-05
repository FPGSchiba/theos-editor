//! Line and column positions over an LF text.

use std::fmt;

use super::span::Span;

/// A 1-based line and column; the column counts Unicode scalar values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    /// Line, from 1.
    pub line: usize,
    /// Column, from 1, in Unicode scalar values (a tab is one column).
    pub column: usize,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Why an offset or a position cannot be converted.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum PositionError {
    /// The offset is past the end of the text.
    #[error("offset {offset} is past the end of the text ({len} bytes)")]
    OutOfRange {
        /// The offending byte offset.
        offset: usize,
        /// Length of the text in bytes.
        len: usize,
    },
    /// The offset falls inside a multi-byte character.
    #[error("offset {offset} is not on a character boundary")]
    NotCharBoundary {
        /// The offending byte offset.
        offset: usize,
    },
    /// The text has no such line.
    #[error("no line {line}")]
    NoSuchLine {
        /// The requested line.
        line: usize,
    },
    /// The line has no such column.
    #[error("line {line} has no column {column}")]
    NoSuchColumn {
        /// The requested line.
        line: usize,
        /// The requested column.
        column: usize,
    },
}

/// An LF text together with the byte offset where each line starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextIndex {
    text: String,
    line_starts: Vec<usize>,
}

impl TextIndex {
    /// Indexes `text`, which must already use LF line endings (see [`super::normalize`]).
    #[must_use]
    pub fn new(text: String) -> TextIndex {
        let line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        TextIndex { text, line_starts }
    }

    /// The indexed text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The position of a byte offset; the end of the text is a valid offset.
    ///
    /// # Errors
    ///
    /// [`PositionError::OutOfRange`] past the end, [`PositionError::NotCharBoundary`] inside a
    /// character.
    pub fn position(&self, offset: usize) -> Result<Position, PositionError> {
        if offset > self.text.len() {
            return Err(PositionError::OutOfRange {
                offset,
                len: self.text.len(),
            });
        }
        if !self.text.is_char_boundary(offset) {
            return Err(PositionError::NotCharBoundary { offset });
        }
        let line = self.line_starts.partition_point(|&start| start <= offset);
        let start = self.line_starts[line - 1];
        let column = self.text[start..offset].chars().count() + 1;
        Ok(Position { line, column })
    }

    /// The byte offset of a position; the column after a line's last character is valid.
    ///
    /// # Errors
    ///
    /// [`PositionError::NoSuchLine`] or [`PositionError::NoSuchColumn`].
    pub fn offset(&self, position: Position) -> Result<usize, PositionError> {
        let Position { line, column } = position;
        let span = self
            .line_span(line)
            .ok_or(PositionError::NoSuchLine { line })?;
        let content = span.get(&self.text).unwrap_or_default();
        let content = content.strip_suffix('\n').unwrap_or(content);
        let skip = column
            .checked_sub(1)
            .filter(|skip| *skip <= content.chars().count())
            .ok_or(PositionError::NoSuchColumn { line, column })?;
        let within = content
            .char_indices()
            .nth(skip)
            .map_or(content.len(), |(i, _)| i);
        Ok(span.start + within)
    }

    /// The span of a line (from 1) including its `'\n'`, if it has one.
    #[must_use]
    pub fn line_span(&self, line: usize) -> Option<Span> {
        let start = *self.line_starts.get(line.checked_sub(1)?)?;
        let end = self
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(self.text.len());
        Some(Span::new(start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIXED: &str = "title: Mère Ilse ö 😀 x\nkind: npc";

    fn pos(line: usize, column: usize) -> Position {
        Position { line, column }
    }

    #[test]
    fn position_counts_unicode_scalar_values() {
        let index = TextIndex::new(MIXED.to_string());
        assert_eq!(MIXED.as_bytes()[26], b'x');
        assert_eq!(index.position(26), Ok(pos(1, 22)));
        assert_eq!(index.position(28), Ok(pos(2, 1)));
        assert_eq!(index.offset(pos(2, 1)), Ok(28));
        assert_eq!(pos(2, 1).to_string(), "2:1");
    }

    #[test]
    fn tab_is_one_column() {
        let index = TextIndex::new("a\tb".to_string());
        assert_eq!(index.position(2), Ok(pos(1, 3)));
    }

    #[test]
    fn offsets_and_positions_round_trip() {
        for text in ["è\u{00A0}e\u{0301}😀\n\tx\n", "è\u{00A0}e\u{0301}😀\n\tx"] {
            let index = TextIndex::new(text.to_string());
            for offset in (0..=text.len()).filter(|o| text.is_char_boundary(*o)) {
                let position = index.position(offset).unwrap();
                assert_eq!(index.offset(position), Ok(offset), "offset {offset}");
            }
        }
    }

    #[test]
    fn off_boundary_and_out_of_range_are_errors() {
        let index = TextIndex::new(MIXED.to_string());
        assert_eq!(
            index.position(9),
            Err(PositionError::NotCharBoundary { offset: 9 })
        );
        assert_eq!(
            index.position(MIXED.len() + 1),
            Err(PositionError::OutOfRange {
                offset: MIXED.len() + 1,
                len: MIXED.len()
            })
        );
        assert_eq!(
            index.offset(pos(9, 1)),
            Err(PositionError::NoSuchLine { line: 9 })
        );
        assert_eq!(
            index.offset(pos(1, 99)),
            Err(PositionError::NoSuchColumn {
                line: 1,
                column: 99
            })
        );
    }

    #[test]
    fn line_spans_include_the_newline() {
        let index = TextIndex::new("ab\ncd".to_string());
        assert_eq!(index.line_span(1), Some(Span::new(0, 3)));
        assert_eq!(index.line_span(2), Some(Span::new(3, 5)));
        assert_eq!(index.line_span(3), None);
        assert_eq!(index.line_span(0), None);
    }
}
