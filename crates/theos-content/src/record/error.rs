//! Errors of reading and splitting records.

use std::path::PathBuf;

use crate::text::{EditError, Position};

/// Why a text is not a record.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum RecordError {
    /// The text starts with U+FEFF.
    #[error("1:1: the record starts with a byte order mark")]
    ByteOrderMark,
    /// The first line is not exactly `---`.
    #[error("1:1: a record starts with a `---` line")]
    NoOpeningLine,
    /// No later line is exactly `---`.
    #[error("1:1: front matter has no closing `---` line")]
    NoClosingLine,
}

impl RecordError {
    /// The id of the rule this error belongs to.
    #[must_use]
    pub fn rule(&self) -> &'static str {
        match self {
            RecordError::ByteOrderMark => "bom",
            RecordError::NoOpeningLine => "opening-line",
            RecordError::NoClosingLine => "closing-line",
        }
    }

    /// Where the problem is; always the start of the record.
    #[must_use]
    pub fn position(&self) -> Position {
        Position { line: 1, column: 1 }
    }
}

/// Why a record file cannot be read.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// The file could not be read.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The bytes are not UTF-8.
    #[error("{position}: invalid UTF-8")]
    Utf8 {
        /// Position of the first invalid byte, in the normalised valid prefix.
        position: Position,
    },
    /// The text is not a record.
    #[error(transparent)]
    Record(#[from] RecordError),
}

/// Why edits cannot produce a record.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum RecordEditError {
    /// The edits do not apply.
    #[error(transparent)]
    Edit(#[from] EditError),
    /// The edited text is not a record.
    #[error(transparent)]
    Record(#[from] RecordError),
}
