//! The tokens of the Rust text that the scans of the `lints` check read: words, strings and
//! punctuation, without comments, white space and character literals.

use crate::checks::ac_trace::rust_tags::literal_end;

/// A piece of attribute text that matters: a word, a string or a punctuation mark. Comments,
/// white space and character literals are left out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Token<'a> {
    /// Letters, digits and `_`.
    Word(&'a str),
    /// A string literal as written, with its quotes.
    Text(&'a str),
    /// An ASCII character that is none of the above.
    Symbol(u8),
}

/// The tokens of the attribute `text`.
pub(super) fn tokens(text: &str) -> Vec<Token<'_>> {
    located_tokens(text)
        .into_iter()
        .map(|(_, token)| token)
        .collect()
}

/// The tokens of `text`, each with the byte it starts at.
pub(super) fn located_tokens(text: &str) -> Vec<(usize, Token<'_>)> {
    let is_word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
        } else if bytes[i..].starts_with(b"//") {
            i = text[i..].find('\n').map_or(bytes.len(), |end| i + end);
        } else if bytes[i..].starts_with(b"/*") {
            i = text[i + 2..]
                .find("*/")
                .map_or(bytes.len(), |end| i + 2 + end + 2);
        } else if let Some(end) = literal_end(text, i) {
            let literal = &text[i..end];
            if literal.starts_with(['"', 'r']) {
                tokens.push((i, Token::Text(literal)));
            }
            i = end;
        } else if is_word(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_word(bytes[i]) {
                i += 1;
            }
            tokens.push((start, Token::Word(&text[start..i])));
        } else {
            if bytes[i].is_ascii() {
                tokens.push((i, Token::Symbol(bytes[i])));
            }
            i += 1;
        }
    }
    tokens
}
