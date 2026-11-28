//! The body of a record as the Codex draws it: P02's block tree ([`theos_content::body::tree`])
//! with each wikilink resolved to the record it names. Nothing in it is HTML.

use serde::Serialize;
use theos_content::body::{Inline, Node};

use super::resolve::{Resolver, ViewLink};

/// A block of a record body, as [`Node`], with resolved links.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewNode {
    /// A heading.
    Heading {
        /// The level, 1 to 6.
        level: u8,
        /// The text of the heading.
        inlines: Vec<ViewInline>,
    },
    /// A paragraph, or the text of a tight list item.
    Paragraph {
        /// The text of the paragraph.
        inlines: Vec<ViewInline>,
    },
    /// A bullet or numbered list.
    List {
        /// Whether the list is numbered.
        ordered: bool,
        /// The items, each a list of blocks.
        items: Vec<Vec<ViewNode>>,
    },
    /// A block quote that is not a callout, or a callout whose title is malformed.
    Quote {
        /// The blocks inside the quote.
        nodes: Vec<ViewNode>,
    },
    /// A fenced or indented code block.
    Code {
        /// The code as written.
        text: String,
    },
    /// A thematic break.
    Rule,
    /// A `[!voice]` callout.
    Voice {
        /// Who speaks.
        speaker: Box<ViewLink>,
        /// The beliefs voiced.
        beliefs: Vec<ViewLink>,
        /// The tier, when the title names one.
        tier: Option<String>,
        /// The speech.
        nodes: Vec<ViewNode>,
        /// The caption line after its `— `, if any.
        caption: Option<Vec<ViewInline>>,
    },
    /// A `[!design]` callout.
    Design {
        /// The remark.
        nodes: Vec<ViewNode>,
    },
    /// A `[!note]` callout.
    Note {
        /// The state as written.
        state: Option<String>,
        /// The title before the state.
        title: Vec<ViewInline>,
        /// The note's text.
        nodes: Vec<ViewNode>,
    },
    /// A callout of a type without a title grammar.
    Callout {
        /// The type in lower case.
        name: String,
        /// The title after the marker.
        title: Vec<ViewInline>,
        /// The blocks after the title line.
        nodes: Vec<ViewNode>,
    },
    /// A beat of a script.
    Beat {
        /// The heading text.
        heading: Vec<ViewInline>,
        /// The attribute lines that parse, in order.
        attributes: Vec<BeatRow>,
        /// The blocks of the beat.
        nodes: Vec<ViewNode>,
    },
    /// Source text shown as it is: a malformed construct or an HTML block.
    Raw {
        /// The characters as written.
        text: String,
    },
}

/// One attribute line of a beat.
#[derive(Clone, Debug, Serialize)]
pub struct BeatRow {
    /// The key before `::`.
    pub key: String,
    /// The value.
    pub value: Vec<ViewInline>,
}

/// A run of text inside a block, as [`Inline`], with resolved links.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewInline {
    /// Plain text.
    Text {
        /// The text.
        text: String,
    },
    /// Emphasised text.
    Emphasis {
        /// The text inside.
        inlines: Vec<ViewInline>,
    },
    /// Strong text.
    Strong {
        /// The text inside.
        inlines: Vec<ViewInline>,
    },
    /// Inline code.
    Code {
        /// The code.
        text: String,
    },
    /// A hard line break.
    Break,
    /// A wikilink chip.
    Link(ViewLink),
    /// A Markdown link or image: its text and URL, never fetched.
    External {
        /// The shown text.
        text: String,
        /// The URL as written.
        url: String,
    },
}

/// The view of `nodes`, every link resolved by `resolver`.
#[must_use]
pub fn nodes(nodes: &[Node], resolver: &Resolver) -> Vec<ViewNode> {
    nodes
        .iter()
        .map(|node| self::node(node, resolver))
        .collect()
}

fn node(node: &Node, r: &Resolver) -> ViewNode {
    match node {
        Node::Heading { level, inlines } => ViewNode::Heading {
            level: *level,
            inlines: self::inlines(inlines, r),
        },
        Node::Paragraph { inlines } => ViewNode::Paragraph {
            inlines: self::inlines(inlines, r),
        },
        Node::List { ordered, items } => ViewNode::List {
            ordered: *ordered,
            items: items.iter().map(|item| nodes(item, r)).collect(),
        },
        Node::Quote { nodes: inner } => ViewNode::Quote {
            nodes: nodes(inner, r),
        },
        Node::Code { text } => ViewNode::Code { text: text.clone() },
        Node::Rule => ViewNode::Rule,
        Node::Voice {
            speaker,
            beliefs,
            tier,
            nodes: inner,
            caption,
        } => ViewNode::Voice {
            speaker: Box::new(r.body_link(speaker)),
            beliefs: beliefs.iter().map(|b| r.body_link(b)).collect(),
            tier: tier.clone(),
            nodes: nodes(inner, r),
            caption: caption.as_ref().map(|c| inlines(c, r)),
        },
        Node::Design { nodes: inner } => ViewNode::Design {
            nodes: nodes(inner, r),
        },
        Node::Note {
            state,
            title,
            nodes: inner,
        } => ViewNode::Note {
            state: state.clone(),
            title: inlines(title, r),
            nodes: nodes(inner, r),
        },
        Node::Callout {
            name,
            title,
            nodes: inner,
        } => ViewNode::Callout {
            name: name.clone(),
            title: inlines(title, r),
            nodes: nodes(inner, r),
        },
        Node::Beat {
            heading,
            attributes,
            nodes: inner,
        } => ViewNode::Beat {
            heading: inlines(heading, r),
            attributes: attributes
                .iter()
                .map(|(key, value)| BeatRow {
                    key: key.clone(),
                    value: inlines(value, r),
                })
                .collect(),
            nodes: nodes(inner, r),
        },
        Node::Raw { text } => ViewNode::Raw { text: text.clone() },
    }
}

fn inlines(inlines: &[Inline], r: &Resolver) -> Vec<ViewInline> {
    inlines
        .iter()
        .map(|inline| self::inline(inline, r))
        .collect()
}

fn inline(inline: &Inline, r: &Resolver) -> ViewInline {
    match inline {
        Inline::Text(text) => ViewInline::Text { text: text.clone() },
        Inline::Emphasis(inner) => ViewInline::Emphasis {
            inlines: inlines(inner, r),
        },
        Inline::Strong(inner) => ViewInline::Strong {
            inlines: inlines(inner, r),
        },
        Inline::Code(text) => ViewInline::Code { text: text.clone() },
        Inline::Break => ViewInline::Break,
        Inline::Link(link) => ViewInline::Link(r.body_link(link)),
        Inline::External { text, url } => ViewInline::External {
            text: text.clone(),
            url: url.clone(),
        },
    }
}
