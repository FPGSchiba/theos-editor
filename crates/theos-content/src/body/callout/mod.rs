//! Callouts `> [!type] title`: recognised once from the shared events, their titles read by a
//! registry of title grammars (one module per callout type, one line in [`GRAMMARS`]).

mod design;
mod note;
mod scan;
mod title;
mod voice;

#[cfg(test)]
mod tests;

pub use design::Design;
pub use note::{Note, NoteState};
pub use voice::Voice;

pub(crate) use scan::scan;

use super::links::Link;
use super::malformed::Malformed;
use crate::text::{Span, TextIndex};
use title::TitleToken;

/// A callout: a block quote whose first line starts with `[!type]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Callout {
    /// The bytes of the whole block quote.
    pub span: Span,
    /// How many block quotes enclose it, itself included: 1 at the top level.
    pub depth: usize,
    /// The type in lower case.
    pub name: String,
    /// The bytes of the type as written, without `!`.
    pub name_span: Span,
    /// The fold marker after `]`, if any.
    pub fold: Option<Fold>,
    /// The rest of the first line after one optional space; may be empty.
    pub title_span: Span,
    /// From the start of the second line to the end of the callout; may be empty.
    pub content_span: Span,
    /// The last line of the last paragraph when it starts with `— `, from the dash to the line
    /// end.
    pub caption_span: Option<Span>,
    /// The parsed title; `None` when the title is malformed (a report exists).
    pub title: Option<CalloutTitle>,
}

/// The fold marker of a callout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fold {
    /// `+`: open by default.
    Open,
    /// `-`: closed by default.
    Closed,
}

/// A parsed callout title, by callout type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalloutTitle {
    /// A voice block: speaker, beliefs and tier.
    Voice(Voice),
    /// A design remark.
    Design(Design),
    /// A note with its state and text.
    Note(Note),
    /// A type without a grammar: plain data.
    Other,
}

/// A bare word of a title with its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    /// The word.
    pub text: String,
    /// Its bytes.
    pub span: Span,
}

/// A block of a callout's content: a direct child of the quote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ContentBlock {
    /// The block's bytes, without the title line and the caption line.
    pub span: Span,
    /// Whether the block is a paragraph.
    pub paragraph: bool,
}

/// What a title grammar reads.
pub(crate) struct TitleInput<'a> {
    /// The record's index, for positions.
    pub index: &'a TextIndex,
    /// The bytes of the title line after the marker.
    pub title_span: Span,
    /// The title split at ` · `.
    pub tokens: &'a [TitleToken],
    /// The links inside the title.
    pub links: &'a [Link],
    /// The blocks of the content without the title line and the caption line.
    pub content: &'a [ContentBlock],
    /// How many block quotes enclose the callout, itself included.
    pub depth: usize,
}

/// The grammar of one callout type.
pub(crate) trait TitleGrammar {
    /// The callout type in lower case.
    fn name(&self) -> &'static str;
    /// Whether callouts of this type end in a `— ` caption line.
    fn captioned(&self) -> bool;
    /// Reads the title.
    fn parse(&self, input: &TitleInput<'_>) -> Result<CalloutTitle, Malformed>;
}

/// The registry: one line per callout type with a title grammar.
const GRAMMARS: &[&dyn TitleGrammar] = &[
    &voice::VoiceGrammar,
    &design::DesignGrammar,
    &note::NoteGrammar,
];

/// The grammar registered for the callout type `name`.
fn grammar(name: &str) -> Option<&'static dyn TitleGrammar> {
    GRAMMARS.iter().copied().find(|g| g.name() == name)
}
