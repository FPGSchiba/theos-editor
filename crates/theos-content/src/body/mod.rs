//! Bodies: one pulldown-cmark pass, the top-level blocks, wikilinks and malformed constructs.

mod events;
mod links;
mod malformed;

pub use links::Link;
pub use malformed::{Malformed, MalformedKind};

use pulldown_cmark::{Event, Tag};

use crate::text::{Span, TextIndex};
use events::Events;

/// A top-level block of a body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The bytes of the block, final newline included.
    pub span: Span,
    /// What the block is.
    pub kind: BlockKind,
}

/// The kind of a [`Block`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    /// A heading, ATX or setext.
    Heading,
    /// Anything else.
    Other,
}

/// A parsed body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body {
    span: Span,
    blocks: Vec<Block>,
    links: Vec<Link>,
    malformed: Vec<Malformed>,
}

impl Body {
    /// Parses the part of the record `index` covers inside `span`; faults become data.
    #[must_use]
    pub fn parse(index: &TextIndex, span: Span) -> Body {
        let events = Events::new(index.text(), span);
        let blocks = blocks(&events);
        let (links, mut malformed) = links::scan(index, &events, &blocks);
        malformed.extend(malformed::scan(index, &events));
        malformed.sort_by_key(|m| m.span.start);
        Body {
            span,
            blocks,
            links,
            malformed,
        }
    }

    /// The bytes of the body in record coordinates.
    #[must_use]
    pub fn span(&self) -> Span {
        self.span
    }

    /// The top-level blocks, in order.
    #[must_use]
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// The well-formed wikilinks, in order.
    #[must_use]
    pub fn links(&self) -> &[Link] {
        &self.links
    }

    /// The malformed constructs, in order.
    #[must_use]
    pub fn malformed(&self) -> &[Malformed] {
        &self.malformed
    }
}

fn blocks(events: &Events<'_>) -> Vec<Block> {
    let mut depth = 0usize;
    let mut blocks = Vec::new();
    for (event, span) in &events.items {
        match event {
            Event::Start(tag) => {
                if depth == 0 {
                    let kind = if matches!(tag, Tag::Heading { .. }) {
                        BlockKind::Heading
                    } else {
                        BlockKind::Other
                    };
                    blocks.push(Block { span: *span, kind });
                }
                depth += 1;
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            _ if depth == 0 => blocks.push(Block {
                span: *span,
                kind: BlockKind::Other,
            }),
            _ => {}
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Record;
    use crate::test_support::record;

    fn at(r: &Record, span: Span) -> &str {
        span.get(r.text()).unwrap_or("<bad span>")
    }

    #[test]
    fn links_carry_exact_spans() {
        let r = record("id: a\n", "x [[a|b|c]] y [[S-004#3]] [[mother-ilse]]\n");
        let links = r.body().links();
        assert_eq!(links.len(), 3);
        let base = r.body_span().start;
        assert_eq!(links[0].target, "a");
        assert_eq!(links[0].shown.as_deref(), Some("b|c"));
        assert_eq!(links[0].shown_span, Some(Span::new(base + 6, base + 9)));
        assert_eq!(links[0].spelling(), "[[a|b|c]]");
        assert_eq!(links[1].target, "S-004");
        assert_eq!(links[1].fragment.as_deref(), Some("3"));
        assert_eq!(links[1].beat(), Some(3));
        assert_eq!(links[1].spelling(), "[[S-004#3]]");
        assert_eq!(links[2].target, "mother-ilse");
        for link in links {
            assert_eq!(at(&r, link.target_span), link.target);
            assert_eq!(at(&r, link.span), link.spelling());
            if let (Some(span), Some(text)) = (link.fragment_span, &link.fragment) {
                assert_eq!(at(&r, span), text);
            }
        }
    }

    #[test]
    fn links_everywhere_but_code() {
        let text = [
            "- item [[a]]",
            "",
            "> quote [[b]]",
            "",
            "# Head [[c]]",
            "",
            "> [!note] title [[d]]",
            "> body",
            "",
            "`[[e]]` and <!-- [[f]] -->",
            "",
            "```",
            "[[g]]",
            "```",
            "",
            "<!--",
            "[[h]]",
            "-->",
            "",
        ]
        .join("\n");
        let r = record("id: a\n", &text);
        let targets: Vec<_> = r.body().links().iter().map(|l| l.target.as_str()).collect();
        assert_eq!(targets, ["a", "b", "c", "d"]);
        assert_eq!(r.body().malformed(), []);
    }

    // AC: CE-10.3
    #[test]
    fn malformed_links_report_their_line() {
        let forms = [
            "[[",
            "[[]]",
            "[[|x]]",
            "[[ x ]]",
            "[[world/tower]]",
            "[[a^b]]",
            "[[S-004#x]]",
            "![[x]]",
        ];
        let r = record("id: a\n", &(forms.join("\n\n") + "\n"));
        let got: Vec<_> = r
            .body()
            .malformed()
            .iter()
            .map(|m| (m.kind.as_str(), m.position.to_string()))
            .collect();
        let want = [
            ("link-unclosed", "4:1"),
            ("link-unclosed", "6:1"),
            ("link-unclosed", "8:1"),
            ("link-target", "10:1"),
            ("link-target", "12:1"),
            ("link-target", "14:1"),
            ("link-target", "16:1"),
            ("embed", "18:1"),
        ];
        assert_eq!(got, want.map(|(k, p)| (k, p.to_owned())));
        assert_eq!(r.body().links(), []);
        let r = record("id: a\n", "è [[a b]]\n");
        assert_eq!(r.body().malformed()[0].position.to_string(), "4:3");
    }

    // AC: CE-10.4
    #[test]
    fn blocks_tile_the_body() {
        let bodies = [
            "",
            "\n",
            "x",
            "Beat 1 — X\n---\ntext\n",
            "text\n\n---\n",
            "- a\n  - b\n    - c\n- d\n",
            "```rust\nfn x() {}\n```\n",
            "<div>\nhtml\n</div>\n\nafter\n",
        ];
        for text in bodies {
            let r = record("id: a\n", text);
            let outer = r.body().span();
            let mut cursor = outer.start;
            for block in r.body().blocks() {
                assert!(block.span.start >= cursor, "{text:?}: overlap");
                assert!(block.span.end <= outer.end, "{text:?}: outside");
                let gap = at(&r, Span::new(cursor, block.span.start));
                assert!(gap.trim().is_empty(), "{text:?}: gap {gap:?}");
                cursor = block.span.end;
            }
            let tail = at(&r, Span::new(cursor, outer.end));
            assert!(tail.trim().is_empty(), "{text:?}: tail {tail:?}");
        }
    }

    #[test]
    fn body_is_parsed_when_the_front_matter_fails() {
        let r = record("a: &x 1\n", "see [[b]]\n");
        assert!(r.front_matter().is_err());
        assert_eq!(r.body().links().len(), 1);
    }
}
