//! The block walk: turns the events of one range into nodes, dispatching callouts and beats to
//! their own modules.

use pulldown_cmark::{Event, Tag};

use super::Node;
use crate::body::events::Events;
use crate::body::{Body, MalformedKind};
use crate::text::Span;

/// Walks the shared events of a body. Only the bytes inside `window` are shown: a callout's
/// content leaves out its title line and caption this way.
pub(super) struct Walker<'a> {
    /// The events of the body.
    pub events: &'a Events<'a>,
    /// The body the events belong to: links, callouts, beats and malformed constructs.
    pub body: &'a Body,
    /// The bytes that are shown.
    pub window: Span,
}

/// Malformed kinds that cover a whole block, which is then shown as [`Node::Raw`].
const BLOCK_FAULTS: &[MalformedKind] = &[
    MalformedKind::BeatHeading,
    MalformedKind::MisplacedAttributes,
];

impl<'a> Walker<'a> {
    /// The same walker showing only `window`.
    pub fn with_window(&self, window: Span) -> Walker<'a> {
        Walker {
            events: self.events,
            body: self.body,
            window,
        }
    }

    /// Whether `span` lies inside the window.
    pub fn inside(&self, span: Span) -> bool {
        self.window.start <= span.start && span.end <= self.window.end
    }

    /// Whether `span` shares a byte with the window.
    pub fn overlaps(&self, span: Span) -> bool {
        span.start < self.window.end && self.window.start < span.end
    }

    /// The source text of `span`, or an empty string for a span that does not fit.
    pub fn source(&self, span: Span) -> &'a str {
        span.get(self.events.source).unwrap_or_default()
    }

    /// The nodes of the blocks in items `from..to`.
    pub fn nodes(&self, from: usize, to: usize) -> Vec<Node> {
        let mut nodes = Vec::new();
        let mut at = from;
        while at < to {
            let (found, next) = self.block(at, to);
            nodes.extend(found);
            at = next.max(at + 1);
        }
        nodes
    }

    /// The nodes of the block at item `at` and the item after it; `end` bounds a beat.
    pub fn block(&self, at: usize, end: usize) -> (Vec<Node>, usize) {
        let (event, span) = &self.events.items[at];
        if is_inline(event) {
            return self.inline_run(at, end);
        }
        let tag = match event {
            Event::Start(tag) => tag,
            Event::Rule if self.overlaps(*span) => return (vec![Node::Rule], at + 1),
            _ => return (Vec::new(), at + 1),
        };
        let close = self.events.closing(at);
        let next = close + 1;
        if !self.overlaps(*span) {
            return (Vec::new(), next);
        }
        let node = match tag {
            Tag::Heading { .. } | Tag::Paragraph if self.is_faulty(*span) => Some(self.raw(*span)),
            Tag::Heading { level, .. } => {
                if let Some(beat) = self
                    .body
                    .beats()
                    .iter()
                    .find(|b| b.heading_span.start == span.start)
                {
                    return self.beat(at, close, end, beat);
                }
                Some(Node::Heading {
                    level: *level as u8,
                    inlines: self.text(at + 1, close),
                })
            }
            Tag::Paragraph => Some(self.text(at + 1, close))
                .filter(|inlines| !inlines.is_empty())
                .map(|inlines| Node::Paragraph { inlines }),
            Tag::BlockQuote(_) => Some(
                match self.body.callouts().iter().find(|c| c.span == *span) {
                    Some(callout) => self.callout(at, close, callout),
                    None => Node::Quote {
                        nodes: self.nodes(at + 1, close),
                    },
                },
            ),
            Tag::CodeBlock(_) => Some(Node::Code {
                text: self.texts(at + 1, close),
            }),
            Tag::HtmlBlock => Some(Node::Raw {
                text: self.texts(at + 1, close).trim_end_matches('\n').to_owned(),
            }),
            Tag::List(start) => Some(Node::List {
                ordered: start.is_some(),
                items: self.items(at + 1, close),
            }),
            Tag::Item => return (self.nodes(at + 1, close), next),
            _ => Some(self.raw(*span)),
        };
        (node.into_iter().collect(), next)
    }

    /// A paragraph of the inline events from `at` up to the first block event.
    fn inline_run(&self, at: usize, end: usize) -> (Vec<Node>, usize) {
        let mut next = at;
        while next < end && is_inline(&self.events.items[next].0) {
            next = match self.events.items[next].0 {
                Event::Start(_) => self.events.closing(next) + 1,
                _ => next + 1,
            };
        }
        let next = next.max(at + 1);
        let inlines = self.text(at, next);
        let nodes = if inlines.is_empty() {
            Vec::new()
        } else {
            vec![Node::Paragraph { inlines }]
        };
        (nodes, next)
    }

    /// The items of the list whose events are `from..to`.
    fn items(&self, from: usize, to: usize) -> Vec<Vec<Node>> {
        let mut items = Vec::new();
        let mut at = from;
        while at < to {
            let close = self.events.closing(at);
            if matches!(self.events.items[at].0, Event::Start(Tag::Item)) {
                items.push(self.nodes(at + 1, close));
            }
            at = close.max(at) + 1;
        }
        items
    }

    /// The text and HTML events of `from..to`, joined: the content of a code or HTML block.
    fn texts(&self, from: usize, to: usize) -> String {
        self.events.items[from..to]
            .iter()
            .filter_map(|(event, _)| match event {
                Event::Text(text) | Event::Html(text) => Some(&**text),
                _ => None,
            })
            .collect()
    }

    /// Whether a block-wide malformed construct lies inside `span`.
    fn is_faulty(&self, span: Span) -> bool {
        self.body.malformed().iter().any(|m| {
            BLOCK_FAULTS.contains(&m.kind) && span.start <= m.span.start && m.span.end <= span.end
        })
    }

    /// `span` as written, without its final line break.
    pub fn raw(&self, span: Span) -> Node {
        Node::Raw {
            text: self.source(span).trim_end_matches('\n').to_owned(),
        }
    }
}

/// Whether the tag opens an inline container.
fn is_inline_tag(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Emphasis
            | Tag::Strong
            | Tag::Strikethrough
            | Tag::Superscript
            | Tag::Subscript
            | Tag::Link { .. }
            | Tag::Image { .. }
    )
}

/// Whether the event belongs to running text.
fn is_inline(event: &Event<'_>) -> bool {
    match event {
        Event::Start(tag) => is_inline_tag(tag),
        Event::Text(_)
        | Event::Code(_)
        | Event::InlineHtml(_)
        | Event::InlineMath(_)
        | Event::SoftBreak
        | Event::HardBreak
        | Event::FootnoteReference(_)
        | Event::TaskListMarker(_) => true,
        _ => false,
    }
}
