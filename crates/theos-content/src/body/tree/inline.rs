//! Inlines: the running text of a block, its wikilinks matched by span with `Body::links`.

use pulldown_cmark::{Event, LinkType, Tag};

use super::Inline;
use super::walk::Walker;
use crate::text::Span;

impl Walker<'_> {
    /// The inlines of items `from..to`, merged and without spaces at both ends: the text of a
    /// paragraph, heading, title, caption or attribute value.
    pub fn text(&self, from: usize, to: usize) -> Vec<Inline> {
        let mut inlines = self.inlines(from, to);
        if let Some(Inline::Text(first)) = inlines.first_mut() {
            *first = first.trim_start().to_owned();
        }
        if let Some(Inline::Text(last)) = inlines.last_mut() {
            *last = last.trim_end().to_owned();
        }
        inlines.retain(|inline| !matches!(inline, Inline::Text(t) if t.is_empty()));
        inlines
    }

    /// The inlines of items `from..to` inside the window, adjacent text merged.
    fn inlines(&self, from: usize, to: usize) -> Vec<Inline> {
        let mut found = Vec::new();
        let mut at = from;
        while at < to {
            let (event, span) = &self.events.items[at];
            let span = *span;
            at += 1;
            match event {
                Event::Start(tag) => {
                    let close = self.events.closing(at - 1);
                    if self.overlaps(span) {
                        found.extend(self.container(tag, span, at, close));
                    }
                    at = close + 1;
                }
                Event::Text(text) if self.inside(span) => {
                    found.push(Inline::Text(text.to_string()));
                }
                Event::Code(code) if self.overlaps(span) => {
                    found.push(Inline::Code(code.to_string()));
                }
                Event::SoftBreak if self.inside(span) => found.push(Inline::Text(" ".to_owned())),
                Event::HardBreak if self.inside(span) => found.push(Inline::Break),
                Event::End(_) | Event::SoftBreak | Event::HardBreak => {}
                // Text that only partly lies in the window, inline HTML and anything else stay
                // text exactly as written.
                _ if self.overlaps(span) => found.push(Inline::Text(self.clipped(span).to_owned())),
                _ => {}
            }
        }
        merge(found)
    }

    /// The inlines of the container opened by `tag`, whose children are items `from..close`.
    fn container(&self, tag: &Tag<'_>, span: Span, from: usize, close: usize) -> Vec<Inline> {
        let children = || self.inlines(from, close);
        let written = || vec![Inline::Text(self.source(span).to_owned())];
        match tag {
            Tag::Emphasis => vec![Inline::Emphasis(children())],
            Tag::Strong => vec![Inline::Strong(children())],
            Tag::Link {
                link_type: LinkType::WikiLink { .. },
                ..
            } => match self.body.links().iter().find(|link| link.span == span) {
                Some(link) => vec![Inline::Link(link.clone())],
                None => written(),
            },
            Tag::Image {
                link_type: LinkType::WikiLink { .. },
                ..
            } => written(),
            Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. } => vec![Inline::External {
                text: plain(&children()),
                url: dest_url.to_string(),
            }],
            _ => children(),
        }
    }

    /// The part of `span` inside the window, as written.
    fn clipped(&self, span: Span) -> &str {
        let start = span.start.max(self.window.start);
        let end = span.end.min(self.window.end).max(start);
        self.source(Span::new(start, end))
    }
}

/// `inlines` with adjacent text runs joined.
fn merge(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut merged: Vec<Inline> = Vec::with_capacity(inlines.len());
    for inline in inlines {
        match (merged.last_mut(), inline) {
            (Some(Inline::Text(previous)), Inline::Text(next)) => previous.push_str(&next),
            (_, inline) => merged.push(inline),
        }
    }
    merged
}

/// The characters of `inlines` without their marks: the text of a Markdown link or image.
fn plain(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(|inline| match inline {
            Inline::Text(text) | Inline::Code(text) | Inline::External { text, .. } => text.clone(),
            Inline::Emphasis(inner) | Inline::Strong(inner) => plain(inner),
            Inline::Break => " ".to_owned(),
            Inline::Link(link) => link.shown.clone().unwrap_or_else(|| link.target.clone()),
        })
        .collect()
}
