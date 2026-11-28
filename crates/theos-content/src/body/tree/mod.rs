//! A record body as a typed block tree, built from the crate's one pulldown-cmark pass and the
//! body's links, callouts, beats and malformed constructs. The UI draws it; it never parses
//! Markdown or renders HTML.
//!
//! - Wikilinks become [`Inline::Link`] by matching their span with `Body::links`; a link the
//!   body reports as malformed stays text as written.
//! - Callout titles, contents and captions come from the callout's spans; a callout whose title
//!   is malformed is a [`Node::Quote`] whose first node is the title line as [`Node::Raw`].
//! - A heading or paragraph that holds a malformed beat heading or misplaced attribute lines is
//!   [`Node::Raw`] with its source text.
//! - Inline HTML is [`Inline::Text`] and an HTML block is [`Node::Raw`], both as written.

mod beat;
mod callout;
mod inline;
mod node;
mod walk;

#[cfg(test)]
mod tests;

pub use node::{Inline, Node};

use crate::body::events::Events;
use crate::record::Record;
use walk::Walker;

/// The body of `record` as a block tree.
#[must_use]
pub fn tree(record: &Record) -> Vec<Node> {
    let body = record.body();
    let events = Events::new(record.text(), body.span());
    let walker = Walker {
        events: &events,
        body,
        window: body.span(),
    };
    walker.nodes(0, events.items.len())
}
