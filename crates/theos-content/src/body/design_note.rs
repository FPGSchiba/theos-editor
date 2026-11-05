//! `Design note:` paragraphs and the unified list of design notes.

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
    /// A paragraph that starts with `Design note:`.
    Paragraph {
        /// The bytes of `Design note:`.
        marker: Span,
    },
}

/// Every design note of a body in document order: `[!design]` callouts, and paragraphs starting
/// with `Design note:` at the top level or inside a callout.
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
    // One flag per open container: whether it is a callout.
    let mut open: Vec<bool> = Vec::new();
    for (event, span) in &events.items {
        match event {
            Event::Start(tag) => {
                let is_callout = matches!(tag, Tag::BlockQuote(_))
                    && callouts.iter().any(|c| c.span.start == span.start);
                let in_scope = open.is_empty() || open.contains(&true);
                let starts_note = matches!(tag, Tag::Paragraph)
                    && in_scope
                    && span
                        .get(events.source)
                        .is_some_and(|p| p.starts_with(MARKER));
                if starts_note {
                    notes.push(DesignNote {
                        span: *span,
                        form: DesignNoteForm::Paragraph {
                            marker: Span::new(span.start, span.start + MARKER.len()),
                        },
                    });
                }
                open.push(is_callout);
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
}
