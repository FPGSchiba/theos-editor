//! Callouts as nodes: the title from the callout's title span, the content after the title line
//! and the caption from the caption span.

use super::walk::Walker;
use super::{Inline, Node};
use crate::body::{Callout, CalloutTitle};
use crate::text::Span;

/// The separator of title tokens.
const SEPARATOR: &str = " · ";
/// The start of a caption line.
const DASH: &str = "— ";

impl Walker<'_> {
    /// The node of `callout`, whose quote is items `open..=close`.
    pub fn callout(&self, open: usize, close: usize, callout: &Callout) -> Node {
        let first = self
            .events
            .items
            .get(open + 1)
            .map_or(callout.span, |item| item.1);
        let line_end = self
            .source(first)
            .find('\n')
            .map_or(first.end, |at| first.start + at);
        let content_start = (line_end + 1).min(callout.span.end);
        let until = |end: usize| {
            self.with_window(Span::new(content_start, end))
                .nodes(open + 1, close)
        };
        let in_window = |span: Span| self.with_window(span).text(open + 1, close);
        let caption_start = callout.caption_span.map_or(callout.span.end, |c| c.start);
        match &callout.title {
            None => {
                let marker = callout.name_span.start.saturating_sub(2);
                let mut nodes = vec![self.raw(Span::new(marker, line_end))];
                nodes.extend(until(callout.span.end));
                Node::Quote { nodes }
            }
            Some(CalloutTitle::Voice(voice)) => Node::Voice {
                speaker: voice.speaker.clone(),
                beliefs: voice.beliefs.clone(),
                tier: voice.tier.as_ref().map(|tier| tier.text.clone()),
                nodes: until(caption_start),
                caption: callout
                    .caption_span
                    .map(|span| without_dash(in_window(span))),
            },
            Some(CalloutTitle::Design(_)) => {
                let title = in_window(callout.title_span);
                let mut nodes = Vec::new();
                if !title.is_empty() {
                    nodes.push(Node::Paragraph { inlines: title });
                }
                nodes.extend(until(callout.span.end));
                Node::Design { nodes }
            }
            Some(CalloutTitle::Note(note)) => {
                let title = callout.title_span;
                let before = self.source(Span::new(title.start, note.state_span.start));
                let end = title.start + before.trim_end_matches(SEPARATOR).len();
                let mut nodes = until(caption_start);
                if let Some(span) = callout.caption_span {
                    nodes.push(Node::Paragraph {
                        inlines: in_window(span),
                    });
                }
                Node::Note {
                    state: Some(note.state.spelling()),
                    title: in_window(Span::new(title.start, end)),
                    nodes,
                }
            }
            Some(CalloutTitle::Other) => Node::Callout {
                name: callout.name.clone(),
                title: in_window(callout.title_span),
                nodes: until(callout.span.end),
            },
        }
    }
}

/// The caption without its leading dash.
fn without_dash(mut inlines: Vec<Inline>) -> Vec<Inline> {
    if let Some(Inline::Text(first)) = inlines.first_mut()
        && let Some(rest) = first.strip_prefix(DASH)
    {
        *first = rest.to_owned();
    }
    inlines
}
