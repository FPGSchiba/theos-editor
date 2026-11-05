//! Malformed constructs as data: what is wrong, where, and the position of its start.

use pulldown_cmark::utils::TextMergeWithOffset;
use pulldown_cmark::{Event, LinkType, Tag, TagEnd};

use super::events::Events;
use crate::text::{Position, Span, TextIndex};

/// The kind of a malformed construct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MalformedKind {
    /// `[[` without a well-formed link after it.
    LinkUnclosed,
    /// A link target or fragment outside the alphabet.
    LinkTarget,
    /// An embed `![[x]]`.
    Embed,
    /// A callout title that does not parse.
    CalloutTitle,
    /// A beat heading that does not parse.
    BeatHeading,
    /// A beat attribute that does not parse.
    BeatAttribute,
    /// Attributes where no beat heading precedes them.
    MisplacedAttributes,
}

impl MalformedKind {
    /// The kebab-case name used in reports and expectation files.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            MalformedKind::LinkUnclosed => "link-unclosed",
            MalformedKind::LinkTarget => "link-target",
            MalformedKind::Embed => "embed",
            MalformedKind::CalloutTitle => "callout-title",
            MalformedKind::BeatHeading => "beat-heading",
            MalformedKind::BeatAttribute => "beat-attribute",
            MalformedKind::MisplacedAttributes => "misplaced-attributes",
        }
    }
}

/// A malformed construct in a body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Malformed {
    /// What is wrong.
    pub kind: MalformedKind,
    /// A human-readable description.
    pub message: String,
    /// The bytes of the construct.
    pub span: Span,
    /// The line and column of the start of `span`.
    pub position: Position,
}

impl Malformed {
    /// Builds a report for `span`, looking up its start position in `index`.
    pub(crate) fn new(
        index: &TextIndex,
        kind: MalformedKind,
        message: impl Into<String>,
        span: Span,
    ) -> Malformed {
        let position = index
            .position(span.start)
            .unwrap_or(Position { line: 1, column: 1 });
        Malformed {
            kind,
            message: message.into(),
            span,
            position,
        }
    }
}

/// Whether the character at `at` is escaped: an odd number of backslashes stands before it.
fn is_escaped(source: &str, at: usize) -> bool {
    let before = source.get(..at).unwrap_or_default();
    let backslashes = before.len() - before.trim_end_matches('\\').len();
    backslashes % 2 == 1
}

/// Embeds and unclosed `[[`, read from the events.
pub(crate) fn scan(index: &TextIndex, events: &Events<'_>) -> Vec<Malformed> {
    let mut found = Vec::new();
    for (event, span) in &events.items {
        if let Event::Start(Tag::Image {
            link_type: LinkType::WikiLink { .. },
            ..
        }) = event
        {
            found.push(Malformed::new(
                index,
                MalformedKind::Embed,
                "embeds `![[…]]` are not supported",
                *span,
            ));
        }
    }
    let merged = TextMergeWithOffset::new(
        events
            .items
            .iter()
            .map(|(event, span)| (event.clone(), span.start..span.end)),
    );
    // pulldown-cmark reports the lines of a code block as text events; they hold no links.
    let mut in_code_block = false;
    for (event, range) in merged {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                continue;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                continue;
            }
            Event::Text(_) if !in_code_block => {}
            _ => continue,
        }
        let Some(source) = events.source.get(range.clone()) else {
            continue;
        };
        for (at, _) in source.match_indices("[[") {
            let start = range.start + at;
            if is_escaped(events.source, start) {
                continue;
            }
            found.push(Malformed::new(
                index,
                MalformedKind::LinkUnclosed,
                "`[[` does not start a link",
                Span::new(start, start + 2),
            ));
        }
    }
    found
}
