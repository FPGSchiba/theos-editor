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
    for (at, (event, span)) in events.items.iter().enumerate() {
        match event {
            Event::Start(tag) => {
                let is_quote = matches!(tag, Tag::BlockQuote(_));
                let is_callout = is_quote && callouts.iter().any(|c| c.span.start == span.start);
                let in_scope = open.is_empty() || open.iter().any(|&(_, callout)| callout);
                let text = match tag {
                    Tag::Paragraph => Some(*span),
                    Tag::Item => tight_item_text(&events.items, at),
                    _ => None,
                };
                if let Some(text) = text.filter(|_| in_scope) {
                    let depth = open.iter().filter(|&&(quote, _)| quote).count();
                    if let Some(marker) = note_line(events.source, text, depth) {
                        notes.push(DesignNote {
                            span: Span::new(marker.start, text.end),
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

/// The item's own text when it is tight (pulldown-cmark gives it no `Paragraph`): from the event
/// after `Start(Item)` to the first list, block quote, code block, heading or HTML block nested
/// in it, else to the item's end. `None` for a loose or empty item.
fn tight_item_text(items: &[(Event<'_>, Span)], at: usize) -> Option<Span> {
    let (first, first_span) = items.get(at + 1)?;
    if matches!(first, Event::End(_) | Event::Start(Tag::Paragraph)) {
        return None;
    }
    let item = items[at].1;
    let mut end = item.end;
    let mut depth = 0usize;
    for (event, span) in &items[at + 1..] {
        match event {
            Event::Start(
                Tag::List(_)
                | Tag::BlockQuote(_)
                | Tag::CodeBlock(_)
                | Tag::Heading { .. }
                | Tag::HtmlBlock,
            ) if depth == 0 => {
                end = span.start;
                break;
            }
            Event::Start(_) => depth += 1,
            Event::End(_) if depth == 0 => break,
            Event::End(_) => depth -= 1,
            _ => {}
        }
    }
    Some(Span::new(first_span.start, end))
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
                rest = rest.trim_start_matches([' ', '\t']);
                rest = rest.strip_prefix('>').unwrap_or(rest);
                rest = rest.trim_start_matches([' ', '\t']);
            }
        }
        let content = rest.trim_start_matches([' ', '\t']);
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
    use crate::text::Span;

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

    /// Checks that `body` holds exactly one note: a paragraph note from `Design note: x`.
    fn one_note(body: &str) {
        let r = record("id: a\n", body);
        let m = r.text().find("Design note:").unwrap_or(0);
        let x = "Design note: x\n";
        let notes = r.body().design_notes();
        assert_eq!(notes.len(), 1, "{body:?}");
        assert_eq!(notes[0].span, Span::new(m, m + x.len()), "{body:?}");
        assert_eq!(
            notes[0].form,
            DesignNoteForm::Paragraph {
                marker: Span::new(m, m + 12)
            },
            "{body:?}"
        );
    }

    // AC: CE-10.2
    #[test]
    fn a_note_line_in_a_tight_list_item_is_found() {
        one_note("> [!voice] [[a]]\n> - Design note: x\n");
    }

    // AC: CE-10.2
    #[test]
    fn a_lazy_note_line_of_a_tight_item_is_found() {
        one_note("> [!voice] [[a]]\n> - item\n>   Design note: x\n");
    }

    // AC: CE-10.2
    #[test]
    fn a_note_line_in_a_later_tight_item_is_found() {
        one_note("> [!voice] [[a]]\n> Speech.\n>\n> - one\n> - Design note: x\n");
    }

    // AC: CE-10.2
    #[test]
    fn a_tab_after_the_quote_marker_is_stripped() {
        one_note("> [!voice] [[a]]\n>\tDesign note: x\n");
    }

    // AC: CE-10.2
    #[test]
    fn a_note_line_in_a_nested_tight_item_is_found_once() {
        one_note("> [!voice] [[a]]\n> - a\n>   - Design note: x\n");
    }

    // AC: CE-10.2
    #[test]
    fn loose_list_twins_are_unchanged() {
        one_note("> [!voice] [[a]]\n> - Design note: x\n>\n> - two\n");
        one_note("> [!voice] [[a]]\n> - one\n>\n> - Design note: x\n");
    }
}
