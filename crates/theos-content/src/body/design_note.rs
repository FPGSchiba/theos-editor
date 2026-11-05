//! `Design note:` lines and the unified list of design notes.
//!
//! Following data model §9.2, a design note is the part of a paragraph from the first line that
//! starts with `Design note:` (after the block-quote prefix and leading spaces) to the paragraph's
//! end. The paragraph may start with the marker or carry it on a later line.

use pulldown_cmark::{Event, Tag};

use super::callout::{Callout, CalloutTitle};
use super::events::Events;
use crate::text::Span;

const MARKER: &str = "Design note:";

/// A designer remark: a `[!design]` callout or a `Design note:` paragraph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DesignNote {
    /// The bytes of the callout or paragraph.
    pub span: Span,
    /// Which form it takes.
    pub form: DesignNoteForm,
}

/// The form of a [`DesignNote`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesignNoteForm {
    /// A `[!design]` callout: the index into the body's callouts.
    Callout(usize),
    /// A paragraph, or the end of one, from a line that starts with `Design note:`.
    Paragraph {
        /// The bytes of `Design note:`.
        marker: Span,
    },
}

/// Every design note of a body in document order: `[!design]` callouts, and `Design note:` lines
/// of paragraphs at the top level or inside a callout.
pub(crate) fn scan(events: &Events<'_>, callouts: &[Callout]) -> Vec<DesignNote> {
    let mut notes: Vec<DesignNote> = callouts
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c.title, Some(CalloutTitle::Design(_))))
        .map(|(i, c)| DesignNote {
            span: c.span,
            form: DesignNoteForm::Callout(i),
        })
        .collect();
    // Per open container: (is a block quote, is a callout).
    let mut open: Vec<(bool, bool)> = Vec::new();
    for (event, span) in &events.items {
        match event {
            Event::Start(tag) => {
                let is_quote = matches!(tag, Tag::BlockQuote(_));
                let is_callout = is_quote && callouts.iter().any(|c| c.span.start == span.start);
                let in_scope = open.is_empty() || open.iter().any(|&(_, callout)| callout);
                if matches!(tag, Tag::Paragraph) && in_scope {
                    let depth = open.iter().filter(|&&(quote, _)| quote).count();
                    if let Some(marker) = note_line(events.source, *span, depth) {
                        notes.push(DesignNote {
                            span: Span::new(marker.start, span.end),
                            form: DesignNoteForm::Paragraph { marker },
                        });
                    }
                }
                open.push((is_quote, is_callout));
            }
            Event::End(_) => {
                open.pop();
            }
            _ => {}
        }
    }
    notes.sort_by_key(|n| n.span.start);
    notes
}

/// The marker bytes of the first line of `paragraph` that starts with `Design note:`, after the
/// block-quote prefix of `depth` levels and leading spaces are stripped.
fn note_line(source: &str, paragraph: Span, depth: usize) -> Option<Span> {
    let text = paragraph.get(source)?;
    let mut offset = 0;
    for (n, line) in text.split_inclusive('\n').enumerate() {
        // The first line of a paragraph already starts after its prefix.
        let mut rest = line;
        if n > 0 {
            for _ in 0..depth {
                rest = rest.trim_start_matches(' ');
                rest = rest.strip_prefix('>').unwrap_or(rest);
            }
        }
        let content = rest.trim_start_matches(' ');
        if content.starts_with(MARKER) {
            let start = paragraph.start + offset + (line.len() - content.len());
            return Some(Span::new(start, start + MARKER.len()));
        }
        offset += line.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::body::{BlockKind, DesignNoteForm};
    use crate::test_support::record;

    // AC: CE-10.2
    #[test]
    fn design_note_paragraphs_are_found() {
        let r = record(
            "id: a\n",
            "Design note: the player first meets\nthis belief at F4.\n",
        );
        let notes = r.body().design_notes();
        assert_eq!(notes.len(), 1);
        let DesignNoteForm::Paragraph { marker } = notes[0].form else {
            panic!("not a paragraph note");
        };
        assert_eq!(marker.get(r.text()), Some("Design note:"));
        assert_eq!(
            notes[0].span.get(r.text()).map(str::trim_end),
            Some("Design note: the player first meets\nthis belief at F4.")
        );
        assert_eq!(r.body().blocks()[0].kind, BlockKind::DesignNote(0));

        let r = record(
            "id: a\n",
            "> [!voice] [[a]]\n> Speech.\n>\n> Design note: inside a voice block.\n\n\
             > Design note: inside a plain quote.\n",
        );
        let notes = r.body().design_notes();
        assert_eq!(notes.len(), 1);
        assert!(matches!(notes[0].form, DesignNoteForm::Paragraph { .. }));
    }

    // AC: CE-10.2
    #[test]
    fn a_note_line_inside_a_paragraph_is_a_design_note() {
        let r = record(
            "id: a\n",
            "> [!voice] [[a]]\n> Speech.\n> Design note: remark\n> goes on.\n",
        );
        let notes = r.body().design_notes();
        assert_eq!(notes.len(), 1);
        let DesignNoteForm::Paragraph { marker } = notes[0].form else {
            panic!("not a paragraph note");
        };
        assert_eq!(marker.get(r.text()), Some("Design note:"));
        assert_eq!(
            notes[0].span.get(r.text()).map(str::trim_end),
            Some("Design note: remark\n> goes on.")
        );

        let r = record("id: a\n", "Speech.\nDesign note: remark.\n");
        let notes = r.body().design_notes();
        // The note line is later in the paragraph: the block stays `Other`.
        assert_eq!(notes.len(), 1);
        let DesignNoteForm::Paragraph { marker } = notes[0].form else {
            panic!("not a paragraph note");
        };
        assert_eq!(marker.get(r.text()), Some("Design note:"));
        assert_eq!(
            notes[0].span.get(r.text()).map(str::trim_end),
            Some("Design note: remark.")
        );
        assert_eq!(r.body().blocks()[0].kind, BlockKind::Other);
    }

    // AC: CE-10.2
    #[test]
    fn a_voice_keeps_its_caption_after_a_note_line() {
        let r = record(
            "id: a\n",
            "> [!voice] [[a]]\n> Speech.\n> Design note: remark.\n>\n> — Caption\n",
        );
        assert_eq!(r.body().design_notes().len(), 1);
        let caption = r.body().callouts()[0].caption_span;
        assert_eq!(caption.and_then(|c| c.get(r.text())), Some("— Caption"));
    }
}
