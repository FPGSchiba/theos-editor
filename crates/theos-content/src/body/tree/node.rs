//! The types of the block tree: block nodes and their inlines.

use crate::body::Link;

/// A block of a record body, ready to draw: nothing in it is HTML or Markdown to parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    /// A heading with its level from 1 to 6.
    Heading {
        /// The level, 1 to 6.
        level: u8,
        /// The text of the heading.
        inlines: Vec<Inline>,
    },
    /// A paragraph, or the text of a tight list item.
    Paragraph {
        /// The text of the paragraph.
        inlines: Vec<Inline>,
    },
    /// A bullet or numbered list.
    List {
        /// Whether the list is numbered.
        ordered: bool,
        /// The items, each a list of blocks.
        items: Vec<Vec<Node>>,
    },
    /// A block quote that is not a callout, or a callout whose title is malformed.
    Quote {
        /// The blocks inside the quote.
        nodes: Vec<Node>,
    },
    /// A fenced or indented code block.
    Code {
        /// The code as written, without fences or indentation.
        text: String,
    },
    /// A thematic break.
    Rule,
    /// A `[!voice]` callout (data model §9.1).
    Voice {
        /// Who speaks.
        speaker: Link,
        /// The beliefs voiced.
        beliefs: Vec<Link>,
        /// The tier, when the title names one.
        tier: Option<String>,
        /// The speech, without the title line and the caption.
        nodes: Vec<Node>,
        /// The caption line after its `— `, if any.
        caption: Option<Vec<Inline>>,
    },
    /// A `[!design]` callout (data model §9.2); a non-empty title is its first paragraph.
    Design {
        /// The remark.
        nodes: Vec<Node>,
    },
    /// A `[!note]` callout (data model §9.3).
    Note {
        /// The state as written, such as `open` or `resolved by [[id]]`.
        state: Option<String>,
        /// The title before the state: the note id and the version link, if any.
        title: Vec<Inline>,
        /// The note's text; a `— ` caption is its last paragraph.
        nodes: Vec<Node>,
    },
    /// A callout of a type without a title grammar.
    Callout {
        /// The type in lower case.
        name: String,
        /// The title after the marker.
        title: Vec<Inline>,
        /// The blocks after the title line.
        nodes: Vec<Node>,
    },
    /// A beat of a script (data model §9.4).
    Beat {
        /// The heading text, `Beat <n> — <name>`.
        heading: Vec<Inline>,
        /// The attribute lines that parse: key and value, in order.
        attributes: Vec<(String, Vec<Inline>)>,
        /// The blocks of the beat up to the next level-2 heading; an attribute line that does
        /// not parse comes first, as [`Node::Raw`].
        nodes: Vec<Node>,
    },
    /// Source text shown as it is: a construct the body reports as malformed, or an HTML block.
    Raw {
        /// The characters as written.
        text: String,
    },
}

/// A run of text inside a block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inline {
    /// Plain text; inline HTML and malformed links are kept here as written.
    Text(String),
    /// Emphasised text.
    Emphasis(Vec<Inline>),
    /// Strong text.
    Strong(Vec<Inline>),
    /// Inline code.
    Code(String),
    /// A hard line break.
    Break,
    /// A well-formed wikilink, as [`crate::body::Body::links`] holds it.
    Link(Link),
    /// A Markdown link or image: its text (an image's alt text) and URL, never fetched.
    External {
        /// The shown text.
        text: String,
        /// The URL as written.
        url: String,
    },
}
