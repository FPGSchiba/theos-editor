//! Diff of two records: their fields by value and their bodies word by word.

use crate::record::Record;
use crate::text::{Span, normalize};
use crate::yaml::FrontMatter;

use super::fields::{FieldChange, fields};
use super::words::{WordDiff, WordOp, words};

/// The difference between two versions of a record file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordDiff {
    /// Both sides are records (or do not exist).
    Record {
        /// The old side as LF text, empty when it does not exist. Every span of this diff points
        /// into it or into `new`.
        old: String,
        /// The new side as LF text, empty when it does not exist.
        new: String,
        /// The fields, or the word diff of the front matter when a side's YAML is invalid.
        fields: FieldsDiff,
        /// The body, word by word; spans are offsets into the whole record text.
        body: WordDiff,
        /// The texts differ, but no field value and no word does.
        layout_only: bool,
    },
    /// A side is not a record: the whole texts, word by word.
    Text {
        /// The old side as LF text.
        old: String,
        /// The new side as LF text.
        new: String,
        /// The word diff of the two texts.
        words: WordDiff,
    },
}

/// The front matter part of a [`RecordDiff::Record`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldsDiff {
    /// The changed fields.
    Fields(Vec<FieldChange>),
    /// The word diff of the front matter, because a side's YAML is invalid.
    Text(WordDiff),
}

/// One side of the diff, split into its parts.
struct Side {
    text: String,
    front: Option<FrontMatter>,
    front_span: Span,
    body_span: Span,
}

impl Side {
    /// A side that does not exist: no fields, no words.
    fn absent() -> Side {
        Side {
            text: String::new(),
            front: Some(FrontMatter {
                span: Span::new(0, 0),
                fields: Vec::new(),
            }),
            front_span: Span::new(0, 0),
            body_span: Span::new(0, 0),
        }
    }

    /// `Err` with the LF text when `input` is not a record.
    fn read(input: &str) -> Result<Side, String> {
        match Record::parse(input) {
            Ok(record) => Ok(Side {
                text: record.text().to_owned(),
                front: record.front_matter().ok().cloned(),
                front_span: record.front_span(),
                body_span: record.body_span(),
            }),
            Err(_) => Err(normalize(input).0.into_owned()),
        }
    }

    fn slice(&self, span: Span) -> &str {
        span.get(&self.text).unwrap_or_default()
    }
}

/// Moves every span of `diff` by the offsets of the slices it was computed on.
fn shift(diff: WordDiff, old_by: usize, new_by: usize) -> WordDiff {
    let by = |s: Span, n: usize| Span::new(s.start + n, s.end + n);
    let ops = diff
        .ops
        .into_iter()
        .map(|op| match op {
            WordOp::Equal { old, new } => WordOp::Equal {
                old: by(old, old_by),
                new: by(new, new_by),
            },
            WordOp::Delete { old } => WordOp::Delete {
                old: by(old, old_by),
            },
            WordOp::Insert { new } => WordOp::Insert {
                new: by(new, new_by),
            },
        })
        .collect();
    WordDiff { ops }
}

/// Words of the two slices, with spans into the whole texts.
fn words_in(old: &Side, old_span: Span, new: &Side, new_span: Span) -> WordDiff {
    shift(
        words(old.slice(old_span), new.slice(new_span)),
        old_span.start,
        new_span.start,
    )
}

fn unchanged(diff: &WordDiff) -> bool {
    diff.ops.iter().all(|op| matches!(op, WordOp::Equal { .. }))
}

/// Diffs two versions of a record file; `None` is a side that does not exist.
///
/// A side that is not a record (no `---` lines, a byte-order mark) makes the whole diff a
/// [`RecordDiff::Text`]. Both sides are handed back as LF text.
/// `layout_only` compares the bytes as given, so a line-ending change alone counts.
#[must_use]
pub fn record(old: Option<&str>, new: Option<&str>) -> RecordDiff {
    let read = |input: Option<&str>| input.map_or_else(|| Ok(Side::absent()), Side::read);
    let (old_side, new_side) = match (read(old), read(new)) {
        (Ok(o), Ok(n)) => (o, n),
        (o, n) => {
            let text = |r: Result<Side, String>| r.map_or_else(|t| t, |s| s.text);
            let (old, new) = (text(o), text(n));
            let words = words(&old, &new);
            return RecordDiff::Text { old, new, words };
        }
    };
    let fields = match (&old_side.front, &new_side.front) {
        (Some(o), Some(n)) => FieldsDiff::Fields(fields(o, n)),
        _ => FieldsDiff::Text(words_in(
            &old_side,
            old_side.front_span,
            &new_side,
            new_side.front_span,
        )),
    };
    let body = words_in(&old_side, old_side.body_span, &new_side, new_side.body_span);
    let fields_same = match &fields {
        FieldsDiff::Fields(changes) => changes.is_empty(),
        FieldsDiff::Text(diff) => unchanged(diff),
    };
    let layout_only =
        old.unwrap_or_default() != new.unwrap_or_default() && fields_same && unchanged(&body);
    RecordDiff::Record {
        old: old_side.text,
        new: new_side.text,
        fields,
        body,
        layout_only,
    }
}

#[cfg(test)]
mod tests;
