//! The tokens of the Rust text that the scans of the `lints` check read: words, strings and
//! punctuation, without comments, white space and character literals.

use crate::checks::ac_trace::rust_tags::literal_end;

/// A piece of attribute text that matters: a word, a string or a punctuation mark. Comments,
/// white space and character literals are left out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Token<'a> {
    /// A run of ASCII letters, digits and `_` and of non-ASCII characters (see `located_tokens`).
    Word(&'a str),
    /// A string literal as written, with its quotes.
    Text(&'a str),
    /// An ASCII character that is none of the above; a non-ASCII character is never a symbol.
    Symbol(u8),
}

/// The byte after the block comment that starts at `start`. Block comments nest, as in rustc: a
/// scanner that ended one at its first `*/` could take a quote inside the comment for the start
/// of a string and read real code as the string's content. A comment that is never closed ends
/// at the end of the text.
fn block_comment_end(bytes: &[u8], start: usize) -> usize {
    let mut depth = 1_usize;
    let mut i = start + 2;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"/*") {
            depth += 1;
            i += 2;
        } else if bytes[i..].starts_with(b"*/") {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    bytes.len()
}

/// The character at byte `i` of `text`, which must be a character boundary.
fn char_at(text: &str, i: usize) -> Option<char> {
    text.get(i..)?.chars().next()
}

/// White space as rustc reads it: ASCII white space, vertical tab, and the other characters of
/// Unicode's `Pattern_White_Space` (`U+0085`, `U+200E`, `U+200F`, `U+2028`, `U+2029`).
fn is_space(c: char) -> bool {
    c.is_ascii_whitespace()
        || matches!(
            c,
            '\u{b}' | '\u{85}' | '\u{200e}' | '\u{200f}' | '\u{2028}' | '\u{2029}'
        )
}

/// Whether `c` belongs to a word: an ASCII letter, digit or `_`, or any non-ASCII character that
/// is not white space. A non-ASCII character is never dropped: `testé` is one word, and so is
/// `tеst` with a Cyrillic `е`, so neither can pass as `test`. rustc reads them as identifiers
/// (or rejects the file), and no rule word is spelled with one.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || (!c.is_ascii() && !is_space(c))
}

/// The tokens of `text`, each with the byte it starts at.
///
/// Per byte class: white space (see `is_space`) separates tokens; `//` and nested `/* */`
/// comments and character literals are skipped; strings and raw strings are `Text`; ASCII
/// letters, digits, `_` and every other non-ASCII character join into `Word`s; any other ASCII
/// byte, control characters included, is a `Symbol`. Nothing is dropped without being white
/// space, a comment or a character literal.
pub(super) fn located_tokens(text: &str) -> Vec<(usize, Token<'_>)> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let Some(c) = char_at(text, i) else {
            i += 1;
            continue;
        };
        if is_space(c) {
            i += c.len_utf8();
        } else if bytes[i..].starts_with(b"//") {
            i = text[i..].find('\n').map_or(bytes.len(), |end| i + end);
        } else if bytes[i..].starts_with(b"/*") {
            i = block_comment_end(bytes, i);
        } else if let Some(end) = literal_end(text, i) {
            let literal = &text[i..end];
            if literal.starts_with(['"', 'r']) {
                tokens.push((i, Token::Text(literal)));
            }
            i = end;
        } else if is_word(c) {
            let start = i;
            while let Some(c) = char_at(text, i).filter(|c| is_word(*c)) {
                i += c.len_utf8();
            }
            tokens.push((start, Token::Word(&text[start..i])));
        } else {
            tokens.push((i, Token::Symbol(bytes[i])));
            i += 1;
        }
    }
    tokens
}
