//! The note title `n<k> · [[version]] · state`, and the note's text.

use super::title::TitleToken;
use super::{CalloutTitle, TitleGrammar, TitleInput, Token};
use crate::body::links::Link;
use crate::body::malformed::{Malformed, MalformedKind};
use crate::text::Span;

/// A parsed `[!note]` callout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// The note id as written, such as `n3`.
    pub id: Token,
    /// The number `k` of `n<k>`.
    pub number: u32,
    /// The version the note is about, when the title names one.
    pub version: Option<Link>,
    /// Where the note stands.
    pub state: NoteState,
    /// The bytes of the whole state token.
    pub state_span: Span,
    /// The content without title line and caption, blocks joined by an empty line: a paragraph
    /// is its lines joined by one space; a list, code block or nested quote keeps its lines as
    /// written, with the quote prefix removed.
    pub text: String,
}

/// Where a note stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoteState {
    /// `open`.
    Open,
    /// `dropped`.
    Dropped,
    /// `resolved by [[id]]`.
    ResolvedBy(Link),
    /// Any other word: kept as data, not an error.
    Unknown(String),
}

impl NoteState {
    /// The state as written in a title.
    #[must_use]
    pub fn spelling(&self) -> String {
        match self {
            NoteState::Open => "open".to_owned(),
            NoteState::Dropped => "dropped".to_owned(),
            NoteState::ResolvedBy(link) => format!("resolved by {}", link.spelling()),
            NoteState::Unknown(word) => word.clone(),
        }
    }
}

/// The grammar of `[!note]`.
pub(crate) struct NoteGrammar;

const RESOLVED_BY: &str = "resolved by ";

impl TitleGrammar for NoteGrammar {
    fn name(&self) -> &'static str {
        "note"
    }

    fn captioned(&self) -> bool {
        true
    }

    fn parse(&self, input: &TitleInput<'_>) -> Result<CalloutTitle, Malformed> {
        let fail = |span: Span, message: &str| {
            Malformed::new(input.index, MalformedKind::CalloutTitle, message, span)
        };
        let mut tokens = input.tokens.iter().peekable();
        let (id, number) = match tokens.next() {
            Some(TitleToken::Bare(word)) => match note_number(&word.text) {
                Some(number) => (word.clone(), number),
                None => return Err(fail(word.span, "a note title starts with `n` and a number")),
            },
            Some(other) => return Err(fail(other.span(), "a note title starts with `n<k>`")),
            None => return Err(fail(input.title_span, "a note title needs `n<k>`")),
        };
        let version = match tokens.next_if(|t| matches!(t, TitleToken::Link(_))) {
            Some(TitleToken::Link(link)) => Some(link.clone()),
            _ => None,
        };
        let Some(state_token) = tokens.next() else {
            return Err(fail(input.title_span, "a note title needs a state"));
        };
        let state = match state_token {
            TitleToken::Bare(word) => match word.text.as_str() {
                "open" => NoteState::Open,
                "dropped" => NoteState::Dropped,
                other => NoteState::Unknown(other.to_owned()),
            },
            TitleToken::Invalid(token) => resolved_by(input, token).ok_or_else(|| {
                fail(token.span, "expected a state, such as `resolved by [[id]]`")
            })?,
            TitleToken::Link(link) => return Err(fail(link.span, "a link is not a note state")),
        };
        if let Some(extra) = tokens.next() {
            return Err(fail(extra.span(), "nothing may follow the note state"));
        }
        Ok(CalloutTitle::Note(Note {
            id,
            number,
            version,
            state,
            state_span: state_token.span(),
            text: note_text(input),
        }))
    }
}

/// `k` of `n<k>`: digits only, fitting `u32`.
fn note_number(word: &str) -> Option<u32> {
    let digits = word.strip_prefix('n')?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// `resolved by ` followed by exactly one link and nothing else.
fn resolved_by(input: &TitleInput<'_>, token: &Token) -> Option<NoteState> {
    let rest = token.text.strip_prefix(RESOLVED_BY)?;
    let start = token.span.start + RESOLVED_BY.len();
    let span = Span::new(start, start + rest.len());
    input
        .links
        .iter()
        .find(|link| link.span == span)
        .map(|link| NoteState::ResolvedBy(link.clone()))
}

/// The text of the content blocks of the callout, separated by one empty line. A paragraph is
/// its trimmed lines joined by one space; any other block keeps its lines as they are, without
/// the quote prefix (the block's own first line starts after it already).
fn note_text(input: &TitleInput<'_>) -> String {
    let source = input.index.text();
    let blocks: Vec<String> = input
        .content
        .iter()
        .filter_map(|block| Some((block, block.span.get(source)?)))
        .map(|(block, text)| {
            // A block that starts after its quote prefix has no prefix on its first line.
            let starts_line = block.span.start == 0
                || source.as_bytes().get(block.span.start - 1) == Some(&b'\n');
            let lines =
                text.trim_end_matches('\n')
                    .lines()
                    .enumerate()
                    .map(|(at, line)| match at {
                        0 if !starts_line => line,
                        _ => strip_quotes(line, input.depth),
                    });
            if block.paragraph {
                lines.map(str::trim).collect::<Vec<_>>().join(" ")
            } else {
                let mut kept: Vec<&str> = lines.collect();
                while kept.last().is_some_and(|line| line.trim().is_empty()) {
                    kept.pop();
                }
                kept.join("\n")
            }
        })
        .collect();
    blocks.join("\n\n")
}

/// Removes up to `depth` `>` markers, each with at most one following space.
fn strip_quotes(line: &str, depth: usize) -> &str {
    let mut rest = line;
    for _ in 0..depth {
        let Some(after) = rest.trim_start_matches(' ').strip_prefix('>') else {
            break;
        };
        rest = after.strip_prefix(' ').unwrap_or(after);
    }
    rest
}
