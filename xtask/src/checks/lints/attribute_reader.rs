//! The one reader of attributes that every attribute rule of the `lints` check uses.
//!
//! It works on the token stream, so it finds every `#[…]` and `#![…]` of a source wherever it
//! stands: at the start of a line or after other code, in a one-line `mod m { #![…] … }`, in a
//! function body, in a `macro_rules!` arm, over several lines and with brackets nested in it.
//! Strings, character literals and comments (doc comments too) are not tokens, so what they hold
//! never makes an attribute.

use super::fixture_routes::line_at;
use super::tokens::{Token, located_tokens};

/// An attribute of a source file.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Attribute<'a> {
    /// The line of the `#`, counted from 1.
    pub line: u32,
    /// The tokens between the brackets, nested brackets included. An attribute that is never
    /// closed has all the tokens that follow it.
    pub body: Vec<Token<'a>>,
}

/// Every attribute of the Rust source `text`, in file order. An attribute inside the body of
/// another (a macro that holds one) is listed too.
pub(super) fn read_attributes(text: &str) -> Vec<Attribute<'_>> {
    let tokens = located_tokens(text);
    let mut attributes = Vec::new();
    for (index, &(offset, token)) in tokens.iter().enumerate() {
        if token != Token::Symbol(b'#') {
            continue;
        }
        let mut rest = tokens.get(index + 1..).unwrap_or_default();
        if let [(_, Token::Symbol(b'!')), tail @ ..] = rest {
            rest = tail;
        }
        let [(_, Token::Symbol(b'[')), body @ ..] = rest else {
            continue;
        };
        let body: Vec<Token<'_>> = body.iter().map(|&(_, token)| token).collect();
        let length = body_len(&body);
        attributes.push(Attribute {
            line: line_at(text, offset),
            body: body[..length].to_vec(),
        });
    }
    attributes
}

/// The number of tokens of an attribute body up to its closing `]`; all of them if it is never
/// closed.
fn body_len(body: &[Token<'_>]) -> usize {
    let mut depth = 1_usize;
    for (position, token) in body.iter().enumerate() {
        match token {
            Token::Symbol(b'[') => depth += 1,
            Token::Symbol(b']') => {
                depth -= 1;
                if depth == 0 {
                    return position;
                }
            }
            _ => {}
        }
    }
    body.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<u32> {
        read_attributes(text)
            .iter()
            .map(|attribute| attribute.line)
            .collect()
    }

    #[test]
    fn inner_and_outer_attributes_are_read_at_any_position() {
        assert_eq!(
            lines("#[a]\n#![b]\nfn f() { #[c] let x = 1; }\n"),
            [1, 2, 3]
        );
        assert_eq!(lines("mod m { #![a] fn f() {} }"), [1]);
        assert_eq!(lines("macro_rules! m { () => { #[a] fn x() {} } }"), [1]);
    }

    #[test]
    fn an_attribute_over_several_lines_has_the_line_of_its_hash() {
        let attributes = read_attributes("fn a() {}\n#[a(\n    b[0],\n    c\n)]\nfn f() {}\n");
        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].line, 2);
        assert!(attributes[0].body.contains(&Token::Word("c")));
        assert!(!attributes[0].body.contains(&Token::Word("f")));
    }

    #[test]
    fn strings_comments_and_lone_hashes_are_no_attributes() {
        for text in [
            "\"#[a]\"",
            "r#\"#[a]\"#",
            "// #[a]",
            "/* #[a] */",
            "/// #[a]",
            "#!",
            "fn f() { let c = '#'; }",
        ] {
            assert!(lines(text).is_empty(), "{text}");
        }
    }

    #[test]
    fn an_attribute_that_is_never_closed_takes_the_rest() {
        let attributes = read_attributes("#[a(b");
        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].body.len(), 3);
    }
}
