//! One Rust file as the checks read it: its text, its syntax tree and its tokens.
//!
//! `syn` and `proc-macro2` do the reading (`proc-macro2` with span locations, so every token
//! knows its line and its byte range in the text); nothing here lexes Rust.

use std::borrow::Cow;
use std::str::FromStr;

use proc_macro2::{Span, TokenStream};

use crate::verdict::Finding;

/// A Rust file, read once.
pub struct Source {
    /// The file, repo-relative with `/`.
    pub path: String,
    /// The text, with a byte-order mark and shebang line as they are in the file.
    pub text: String,
    /// The syntax tree, or `None` when the text does not parse.
    pub file: Option<syn::File>,
    /// The tokens of the text, or `None` when it does not parse. Their byte ranges and lines
    /// match [`Source::text`].
    pub tokens: Option<TokenStream>,
}

/// Reads `bytes` as the Rust file `path`. The finding says the file does not parse; its message
/// never quotes the text.
#[must_use]
pub fn parse_source(path: &str, bytes: &[u8]) -> (Source, Option<Finding>) {
    let unreadable = |text: String, line: Option<u32>| {
        let source = Source {
            path: path.to_owned(),
            text,
            file: None,
            tokens: None,
        };
        let place = line.map_or_else(String::new, |line| format!(" (line {line})"));
        let finding = Finding {
            file: Some(path.to_owned()),
            line,
            message: format!(
                "this file does not parse as Rust{place}; rustc would not compile it either"
            ),
        };
        (source, Some(finding))
    };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return unreadable(String::from_utf8_lossy(bytes).into_owned(), None);
    };
    let file = match syn::parse_file(text) {
        Ok(file) => file,
        Err(error) => return unreadable(text.to_owned(), Some(line_of(error.span()))),
    };
    let tokens = match TokenStream::from_str(&blank_prelude(text)) {
        Ok(tokens) => tokens,
        Err(error) => return unreadable(text.to_owned(), Some(line_of(error.span()))),
    };
    let source = Source {
        path: path.to_owned(),
        text: text.to_owned(),
        file: Some(file),
        tokens: Some(tokens),
    };
    (source, None)
}

/// The line of the start of `span`, counting from 1.
#[must_use]
pub fn line_of(span: Span) -> u32 {
    u32::try_from(span.start().line).unwrap_or(u32::MAX)
}

/// `text` with a leading byte-order mark and shebang line replaced by spaces of the same byte
/// length, which is what `syn::parse_file` skips; the tokens of the result sit at the same byte
/// offsets and lines as in `text`.
pub(crate) fn blank_prelude(text: &str) -> Cow<'_, str> {
    let mark_len = if text.starts_with('\u{feff}') {
        '\u{feff}'.len_utf8()
    } else {
        0
    };
    let rest = &text[mark_len..];
    let shebang_len = if rest.starts_with("#!") && !rest[2..].trim_start().starts_with('[') {
        rest.find('\n').unwrap_or(rest.len())
    } else {
        0
    };
    let blank = mark_len + shebang_len;
    if blank == 0 {
        return Cow::Borrowed(text);
    }
    Cow::Owned(format!("{}{}", " ".repeat(blank), &text[blank..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn a_file_that_does_not_parse_is_a_finding_with_its_line() -> TestResult {
        let (source, finding) = parse_source("crates/a/src/lib.rs", b"fn ok() {}\n\nfn {\n");
        assert!(source.file.is_none() && source.tokens.is_none());
        let finding = finding.ok_or("no finding")?;
        assert_eq!(finding.file.as_deref(), Some("crates/a/src/lib.rs"));
        assert_eq!(finding.line, Some(3));
        assert_eq!(
            finding.message,
            "this file does not parse as Rust (line 3); rustc would not compile it either"
        );
        Ok(())
    }

    #[test]
    fn a_file_that_is_not_utf8_is_a_finding() -> TestResult {
        let (source, finding) = parse_source("a.rs", b"fn a() {} // \xff\n");
        assert!(source.file.is_none());
        let finding = finding.ok_or("no finding")?;
        assert_eq!(finding.line, None);
        assert!(
            finding
                .message
                .starts_with("this file does not parse as Rust;")
        );
        Ok(())
    }

    #[test]
    fn a_byte_order_mark_and_a_shebang_are_blanked_with_their_length() {
        let text = "\u{feff}#!/usr/bin/env run\nfn a() {}\n";
        let blank = blank_prelude(text);
        assert_eq!(blank.len(), text.len());
        assert!(blank.starts_with(' ') && blank.ends_with("\nfn a() {}\n"));
        let (source, finding) = parse_source("a.rs", text.as_bytes());
        assert!(finding.is_none() && source.tokens.is_some());
        // An inner attribute is not a shebang.
        assert!(matches!(blank_prelude("#![deny(x)]\n"), Cow::Borrowed(_)));
    }

    #[test]
    fn tokens_know_their_byte_range_in_the_text() -> TestResult {
        let text = "\u{feff}fn a() {}\n";
        let (source, _) = parse_source("a.rs", text.as_bytes());
        let first = source
            .tokens
            .ok_or("no tokens")?
            .into_iter()
            .next()
            .ok_or("no token")?;
        assert_eq!(&text[first.span().byte_range()], "fn");
        Ok(())
    }
}
