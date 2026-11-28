//! Beats as nodes: the heading, the attribute lines directly after it, and the blocks up to the
//! next level-2 heading.

use pulldown_cmark::{Event, Tag};

use super::walk::Walker;
use super::{Inline, Node};
use crate::body::Beat;
use crate::body::beat::is_attribute_line;
use crate::text::Span;

/// What the attribute paragraph of a beat gives.
#[derive(Default)]
struct Read {
    attributes: Vec<(String, Vec<Inline>)>,
    nodes: Vec<Node>,
}

impl Walker<'_> {
    /// The beat whose heading is items `open..=close`, and the item after it; `end` bounds the
    /// walk.
    pub fn beat(&self, open: usize, close: usize, end: usize, beat: &Beat) -> (Vec<Node>, usize) {
        let heading = self.text(open + 1, close);
        let mut read = Read::default();
        let mut at = close + 1;
        let heading_end = self.events.items[open].1.end;
        if let Some((Event::Start(Tag::Paragraph), paragraph)) = self.events.items.get(at)
            && paragraph.start == heading_end
            && is_attribute_line(self.source(*paragraph))
        {
            let paragraph_close = self.events.closing(at);
            read = self.attribute_lines(at, paragraph_close, *paragraph, beat);
            at = paragraph_close + 1;
        }
        while at < end && self.events.items[at].1.start < beat.span.end {
            let (found, next) = self.block(at, end);
            read.nodes.extend(found);
            at = next.max(at + 1);
        }
        let node = Node::Beat {
            heading,
            attributes: read.attributes,
            nodes: read.nodes,
        };
        (vec![node], at)
    }

    /// Reads the attribute run of the paragraph `open..=close`: a parsed line becomes an
    /// attribute, any other line of the run a raw node, and the lines after the run a paragraph.
    fn attribute_lines(&self, open: usize, close: usize, paragraph: Span, beat: &Beat) -> Read {
        let mut read = Read::default();
        let mut offset = paragraph.start;
        for raw in self.source(paragraph).split_inclusive('\n') {
            let line = raw.trim_end_matches('\n');
            if !is_attribute_line(line) {
                break;
            }
            let start = offset;
            offset += raw.len();
            match beat.attributes.iter().find(|a| a.line_span.start == start) {
                Some(attribute) => read.attributes.push((
                    attribute.key.clone(),
                    self.with_window(attribute.value_span).text(open + 1, close),
                )),
                None => read
                    .nodes
                    .push(self.raw(Span::new(start, start + line.len()))),
            }
        }
        let rest = self
            .with_window(Span::new(offset, paragraph.end))
            .text(open + 1, close);
        if !rest.is_empty() {
            read.nodes.push(Node::Paragraph { inlines: rest });
        }
        read
    }
}
