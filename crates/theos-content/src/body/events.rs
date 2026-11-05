//! The one pulldown-cmark pass over a body; every grammar reads this list, none re-parses.

use pulldown_cmark::{Event, Options, Parser};

use crate::text::Span;

/// The events of one body with their spans in record coordinates.
pub(crate) struct Events<'a> {
    /// The whole record text; the spans index into it.
    pub source: &'a str,
    /// Every event, in document order, with the bytes it covers.
    pub items: Vec<(Event<'a>, Span)>,
}

impl<'a> Events<'a> {
    /// Parses the part of `source` inside `span`; unreadable spans give no events.
    pub fn new(source: &'a str, span: Span) -> Events<'a> {
        let body = span.get(source).unwrap_or("");
        let items = Parser::new_ext(body, Options::ENABLE_WIKILINKS)
            .into_offset_iter()
            .map(|(event, range)| {
                (
                    event,
                    Span::new(range.start + span.start, range.end + span.start),
                )
            })
            .collect();
        Events { source, items }
    }
}
