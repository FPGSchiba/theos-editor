//! Splits a callout title into its ` · `-separated tokens.

use super::Token;
use crate::body::links::Link;
use crate::text::{Span, TextIndex};

const SEPARATOR: &str = " · ";

/// One token of a title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TitleToken {
    /// Exactly one wikilink.
    Link(Link),
    /// A non-empty run without whitespace, `[`, `]` or `·`.
    Bare(Token),
    /// Anything else: empty, several words, a stray bracket or separator.
    Invalid(Token),
}

impl TitleToken {
    /// The bytes of the token.
    pub fn span(&self) -> Span {
        match self {
            TitleToken::Link(link) => link.span,
            TitleToken::Bare(token) | TitleToken::Invalid(token) => token.span,
        }
    }
}

/// Splits the title at ` · ` outside the spans of `links`; an empty title has no tokens.
pub(crate) fn split(index: &TextIndex, title_span: Span, links: &[Link]) -> Vec<TitleToken> {
    let Some(title) = title_span.get(index.text()) else {
        return Vec::new();
    };
    if title.is_empty() {
        return Vec::new();
    }
    let inside_link = |at: usize| {
        links
            .iter()
            .any(|l| l.span.start < title_span.start + at + 1 && title_span.start + at < l.span.end)
    };
    let mut tokens = Vec::new();
    let mut from = 0;
    for (at, _) in title.match_indices(SEPARATOR) {
        if at < from || inside_link(at) {
            continue;
        }
        tokens.push(classify(index, title_span.start, from, at, links));
        from = at + SEPARATOR.len();
    }
    tokens.push(classify(index, title_span.start, from, title.len(), links));
    tokens
}

fn classify(index: &TextIndex, base: usize, from: usize, to: usize, links: &[Link]) -> TitleToken {
    let span = Span::new(base + from, base + to);
    if let Some(link) = links.iter().find(|l| l.span == span) {
        return TitleToken::Link(link.clone());
    }
    let text = span.get(index.text()).unwrap_or_default();
    let token = Token {
        text: text.to_owned(),
        span,
    };
    let is_word = !text.is_empty()
        && !text
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '[' | ']' | '·'));
    if is_word {
        TitleToken::Bare(token)
    } else {
        TitleToken::Invalid(token)
    }
}
