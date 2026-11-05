//! Records: a text file with YAML front matter between two `---` lines, followed by a body.

mod error;
mod read;
mod split;

pub use error::{ReadError, RecordEditError, RecordError};
pub use read::{decode, read_record};

use crate::text::{Span, TextEdit, TextIndex, apply_edits, normalize};
use crate::yaml::{FrontMatter, Input, YamlError};

/// A record read from text: its LF text and the spans of front matter and body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    index: TextIndex,
    was_normalized: bool,
    front_span: Span,
    body_span: Span,
    front: Result<FrontMatter, YamlError>,
}

impl Record {
    /// Normalises line endings and splits `input` into front matter and body.
    ///
    /// # Errors
    ///
    /// [`RecordError::ByteOrderMark`] for a leading U+FEFF, [`RecordError::NoOpeningLine`] or
    /// [`RecordError::NoClosingLine`] when the `---` lines are missing.
    pub fn parse(input: &str) -> Result<Record, RecordError> {
        if input.starts_with('\u{FEFF}') {
            return Err(RecordError::ByteOrderMark);
        }
        let (text, was_normalized) = normalize(input);
        let (front_span, body_span) = split::split(&text)?;
        let index = TextIndex::new(text.into_owned());
        let front = FrontMatter::from_input(&Input::new(&index, front_span));
        Ok(Record {
            index,
            was_normalized,
            front_span,
            body_span,
            front,
        })
    }

    /// The LF text of the whole record.
    #[must_use]
    pub fn text(&self) -> &str {
        self.index.text()
    }

    /// The line index of [`Record::text`].
    #[must_use]
    pub fn index(&self) -> &TextIndex {
        &self.index
    }

    /// Whether the input had `\r` line endings that were turned into LF.
    #[must_use]
    pub fn was_normalized(&self) -> bool {
        self.was_normalized
    }

    /// The lines between the two `---` lines (may be empty).
    #[must_use]
    pub fn front_span(&self) -> Span {
        self.front_span
    }

    /// Everything after the closing `---` line (may be empty).
    #[must_use]
    pub fn body_span(&self) -> Span {
        self.body_span
    }

    /// The first violation of the restricted YAML in the front matter, if any.
    #[must_use]
    pub fn front_matter_error(&self) -> Option<&YamlError> {
        self.front.as_ref().err()
    }

    /// The front matter as a model with spans, or the first violation of the restricted YAML.
    ///
    /// # Errors
    ///
    /// The same [`YamlError`] as [`Record::front_matter_error`].
    pub fn front_matter(&self) -> Result<&FrontMatter, &YamlError> {
        self.front.as_ref()
    }

    /// Applies `edits` to [`Record::text`] and parses the result; the new record is LF.
    ///
    /// # Errors
    ///
    /// [`RecordEditError::Edit`] when the edits do not apply, [`RecordEditError::Record`] when
    /// the result is not a record.
    pub fn with_edits(&self, edits: &[TextEdit]) -> Result<Record, RecordEditError> {
        Ok(Record::parse(&apply_edits(self.text(), edits)?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::record;

    fn spans(input: &str) -> Result<(Span, Span), RecordError> {
        Record::parse(input).map(|r| (r.front_span(), r.body_span()))
    }

    #[test]
    fn splits_front_matter_and_body() {
        let r = record("id: a\n", "body\n");
        assert_eq!(r.text(), "---\nid: a\n---\nbody\n");
        assert_eq!(r.front_span(), Span::new(4, 10));
        assert_eq!(r.body_span(), Span::new(14, 19));
    }

    #[test]
    fn split_edges() {
        assert_eq!(spans("---\n---\n"), Ok((Span::new(4, 4), Span::new(8, 8))));
        assert_eq!(
            spans("---\nid: a\n---"),
            Ok((Span::new(4, 10), Span::new(13, 13)))
        );
        assert_eq!(record("id: a\n", "\n").body_span(), Span::new(14, 15));
        assert_eq!(record("id: a\n", "tail").body_span(), Span::new(14, 18));
        let r = record("id: a\n", "x\n---\n");
        assert_eq!(r.front_span(), Span::new(4, 10));
        assert_eq!(r.body_span(), Span::new(14, 20));
    }

    #[test]
    fn delimiters_are_exact() {
        for (input, error, rule) in [
            (
                "--- \nid: a\n---\n",
                RecordError::NoOpeningLine,
                "opening-line",
            ),
            (
                "---\nid: a\n--- \n",
                RecordError::NoClosingLine,
                "closing-line",
            ),
            ("", RecordError::NoOpeningLine, "opening-line"),
        ] {
            let got = Record::parse(input).err();
            assert_eq!(got.as_ref(), Some(&error), "{input:?}");
            assert_eq!(error.rule(), rule);
            assert_eq!(error.position().to_string(), "1:1");
        }
    }

    #[test]
    fn byte_order_mark_is_refused() {
        let error = Record::parse("\u{FEFF}---\n---\n").err();
        assert_eq!(error, Some(RecordError::ByteOrderMark));
        assert_eq!(RecordError::ByteOrderMark.rule(), "bom");
        assert_eq!(RecordError::ByteOrderMark.position().to_string(), "1:1");
    }

    // AC: CE-09.4
    #[test]
    fn crlf_record_reads_like_its_lf_twin() -> Result<(), RecordError> {
        let crlf = Record::parse("---\r\nid: a\r\n---\r\nb\r\n")?;
        let lf = Record::parse("---\nid: a\n---\nb\n")?;
        assert_eq!(crlf.text(), lf.text());
        assert_eq!(crlf.front_span(), lf.front_span());
        assert_eq!(crlf.body_span(), lf.body_span());
        assert!(crlf.was_normalized());
        assert!(!lf.was_normalized());
        Ok(())
    }

    // AC: CE-09.4
    #[test]
    fn edits_return_a_new_lf_record() -> Result<(), Box<dyn std::error::Error>> {
        let crlf = Record::parse("---\r\nid: a\r\n---\r\nb\r\n")?;
        let edit = TextEdit {
            span: Span::new(8, 9),
            replacement: "b".to_owned(),
        };
        let edited = crlf.with_edits(&[edit])?;
        assert_eq!(edited.text(), "---\nid: b\n---\nb\n");
        assert!(!edited.was_normalized());
        Ok(())
    }
}
