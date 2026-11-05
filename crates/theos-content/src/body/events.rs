//! The one pulldown-cmark pass over a body; every grammar reads this list, none re-parses.

use pulldown_cmark::{Event, Options, Parser};

use crate::text::Span;

/// The events of one body with their spans in record coordinates.
pub(crate) struct Events<'a> {
    /// The whole record text; the spans index into it.
    pub source: &'a str,
    /// Every event, in document order, with the bytes it covers.
    pub items: Vec<(Event<'a>, Span)>,
    /// For each item, the index of the top-level block it belongs to (the item that opens a
    /// block belongs to it); built in the same walk as the items, so it cannot disagree.
    pub block_of: Vec<usize>,
}

impl<'a> Events<'a> {
    /// Parses the part of `source` inside `span`; unreadable spans give no events.
    pub fn new(source: &'a str, span: Span) -> Events<'a> {
        let body = span.get(source).unwrap_or("");
        let mut items = Vec::new();
        let mut block_of = Vec::new();
        let mut depth = 0usize;
        let mut blocks = 0usize;
        for (event, range) in Parser::new_ext(body, Options::ENABLE_WIKILINKS).into_offset_iter() {
            if matches!(event, Event::End(_)) {
                depth = depth.saturating_sub(1);
            } else {
                if depth == 0 {
                    blocks += 1;
                }
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                }
            }
            block_of.push(blocks.saturating_sub(1));
            items.push((
                event,
                Span::new(range.start + span.start, range.end + span.start),
            ));
        }
        Events {
            source,
            items,
            block_of,
        }
    }
}
