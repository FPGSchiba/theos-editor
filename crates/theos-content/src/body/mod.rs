//! Bodies: one pulldown-cmark pass, the top-level blocks, wikilinks and malformed constructs.

mod beat;
mod callout;
mod design_note;
mod events;
mod links;
mod malformed;

pub use beat::{Attribute, AttributeValue, Beat};
pub use callout::{Callout, CalloutTitle, Design, Fold, Note, NoteState, Token, Voice};
pub use design_note::{DesignNote, DesignNoteForm};
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
    /// A top-level callout: the index into [`Body::callouts`].
    Callout(usize),
    /// A top-level `Design note:` paragraph: the index into [`Body::design_notes`].
    DesignNote(usize),
    /// A beat heading: the index into [`Body::beats`].
    BeatHeading(usize),
    /// The attribute paragraph of a beat: the index into [`Body::beats`].
    BeatAttributes(usize),
    /// Anything else.
    Other,
}

/// A parsed body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body {
    span: Span,
    blocks: Vec<Block>,
    links: Vec<Link>,
    callouts: Vec<Callout>,
    design_notes: Vec<DesignNote>,
    beats: Vec<Beat>,
    malformed: Vec<Malformed>,
}

impl Body {
    /// Parses the part of the record `index` covers inside `span`; faults become data.
    #[must_use]
    pub fn parse(index: &TextIndex, span: Span) -> Body {
        let events = Events::new(index.text(), span);
        let (links, mut malformed) = links::scan(index, &events);
        malformed.extend(malformed::scan(index, &events));
        let (callouts, bad_titles) = callout::scan(index, &events, &links);
        malformed.extend(bad_titles);
        let design_notes = design_note::scan(&events, &callouts);
        let (beats, bad_beats) = beat::scan(index, &events, &links, span.end);
        malformed.extend(bad_beats);
        let blocks = blocks(&events, &callouts, &design_notes, &beats);
        malformed.sort_by_key(|m| m.span.start);
        Body {
            span,
            blocks,
            links,
            callouts,
            design_notes,
            beats,
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

    /// Every callout at any depth, in document order.
    #[must_use]
    pub fn callouts(&self) -> &[Callout] {
        &self.callouts
    }

    /// Every design note, in document order.
    #[must_use]
    pub fn design_notes(&self) -> &[DesignNote] {
        &self.design_notes
    }

    /// The beats of a script, in order.
    #[must_use]
    pub fn beats(&self) -> &[Beat] {
        &self.beats
    }

    /// The malformed constructs, in order.
    #[must_use]
    pub fn malformed(&self) -> &[Malformed] {
        &self.malformed
    }
}

fn blocks(
    events: &Events<'_>,
    callouts: &[Callout],
    notes: &[DesignNote],
    beats: &[Beat],
) -> Vec<Block> {
    let mut blocks = Vec::new();
    for ((event, span), &owner) in events.items.iter().zip(&events.block_of) {
        if owner != blocks.len() {
            continue;
        }
        let kind = match event {
            Event::Start(Tag::Heading { .. }) => beats
                .iter()
                .position(|b| b.heading_span.start == span.start)
                .map_or(BlockKind::Heading, BlockKind::BeatHeading),
            Event::Start(Tag::BlockQuote(_)) => callouts
                .iter()
                .position(|c| c.depth == 1 && c.span.start == span.start)
                .map_or(BlockKind::Other, BlockKind::Callout),
            Event::Start(Tag::Paragraph) => match blocks.last() {
                Some(&Block {
                    span: heading,
                    kind: BlockKind::BeatHeading(i),
                }) if heading.end == span.start && !beats[i].attributes.is_empty() => {
                    BlockKind::BeatAttributes(i)
                }
                _ => notes
                    .iter()
                    .position(|n| {
                        n.span.start == span.start
                            && matches!(n.form, DesignNoteForm::Paragraph { .. })
                    })
                    .map_or(BlockKind::Other, BlockKind::DesignNote),
            },
            _ => BlockKind::Other,
        };
        blocks.push(Block { span: *span, kind });
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
            "> [!note] n1 · [[d]] · open",
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
    fn links_know_their_block() {
        let text = "para [[a]]

# Head [[c]]

- item [[d]]

> quote [[e]]

end [[f]]
";
        let r = record(
            "id: a
", text,
        );
        let blocks = r.body().blocks();
        assert_eq!(blocks.len(), 5);
        let got: Vec<_> = r
            .body()
            .links()
            .iter()
            .map(|l| (l.target.as_str(), l.block))
            .collect();
        assert_eq!(got, [("a", 0), ("c", 1), ("d", 2), ("e", 3), ("f", 4)]);
        for link in r.body().links() {
            let block = blocks[link.block].span;
            assert!(block.start <= link.span.start && link.span.end <= block.end);
        }
        assert_eq!(blocks[1].kind, BlockKind::Heading);
        assert_eq!(blocks[r.body().links()[1].block].kind, BlockKind::Heading);
    }

    #[test]
    fn body_is_parsed_when_the_front_matter_fails() {
        let r = record("a: &x 1\n", "see [[b]]\n");
        assert!(r.front_matter().is_err());
        assert_eq!(r.body().links().len(), 1);
    }
}
