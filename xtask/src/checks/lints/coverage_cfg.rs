//! The coverage rule of the `lints` check: no attribute takes code out of the coverage floor.
//!
//! cargo-llvm-cov builds the measured binaries with `--cfg coverage` (and the convention
//! `coverage_nightly`), so `#[cfg(not(coverage))]` on an item removes its lines from the floor
//! while clippy, test and the shipped binaries still build it. `#[coverage(off)]` does the same
//! on a nightly toolchain.

use super::fixture_routes::line_at;
use super::tokens::{Token, located_tokens};
use crate::verdict::Finding;

const COVERAGE_FOUND: &str = "`coverage` or `coverage_nightly` in a `cfg(…)` or `cfg_attr(…)` \
                              attribute, or a `coverage` attribute: it takes code out of the \
                              coverage floor; the code must be built and measured like the rest";

/// The findings for attributes in the Rust source `text` of the file `file` that could take code
/// out of the coverage measure, in file order.
///
/// An attribute is found when its path has a segment `coverage`, or when it is `cfg` or
/// `cfg_attr` and the word `coverage` or `coverage_nightly` stands anywhere in it (inside `not`,
/// `all`, `any`, at any depth). `#[cfg(not(test))]` is allowed. The whole text is scanned, so an
/// attribute after other code on a line and one over several lines are found; words in comments
/// and strings do not count. `cfg!(coverage)` in an expression is not an attribute: both of its
/// branches are compiled and measured.
#[must_use]
pub fn scan_coverage_cfg(file: &str, text: &str) -> Vec<Finding> {
    let tokens = located_tokens(text);
    let mut findings = Vec::new();
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
        let body = &body[..body_len(body)];
        if is_coverage_attribute(body) {
            findings.push(Finding {
                file: Some(file.to_owned()),
                line: Some(line_at(text, offset)),
                message: COVERAGE_FOUND.to_owned(),
            });
        }
    }
    findings
}

/// The number of tokens of an attribute body up to its closing `]`; all of them if it is never
/// closed.
fn body_len(body: &[(usize, Token<'_>)]) -> usize {
    let mut depth = 1_usize;
    for (position, (_, token)) in body.iter().enumerate() {
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

/// Whether the attribute with the tokens `body` is one that this rule forbids.
fn is_coverage_attribute(body: &[(usize, Token<'_>)]) -> bool {
    let is_coverage =
        |token: &Token<'_>| matches!(token, Token::Word("coverage" | "coverage_nightly"));
    // The path: words and `::` up to the first token that is neither.
    let path: Vec<&Token<'_>> = body
        .iter()
        .map(|(_, token)| token)
        .take_while(|token| matches!(token, Token::Word(_) | Token::Symbol(b':')))
        .collect();
    if path
        .iter()
        .any(|token| matches!(token, Token::Word("coverage")))
    {
        return true;
    }
    let configures = path
        .iter()
        .any(|token| matches!(token, Token::Word("cfg" | "cfg_attr")));
    configures && body.iter().any(|(_, token)| is_coverage(token))
}

#[cfg(test)]
mod tests {
    use super::super::testing::{lines_of, source};
    use super::*;

    fn lines(text: &str) -> Vec<Option<u32>> {
        lines_of(&scan_coverage_cfg("a.rs", text))
    }

    // AC: CE-08.8
    #[test]
    fn coverage_cfgs_are_found_at_any_depth() {
        for attribute in [
            "#[cfg(coverage)]",
            "#[cfg(not(coverage))]",
            "#[cfg(coverage_nightly)]",
            "#[cfg(not(coverage_nightly))]",
            "#[cfg(any(test, coverage_nightly))]",
            "#[cfg(all(unix, any(feature = \"x\", not(coverage))))]",
            "#![cfg(not(coverage))]",
            "#[cfg_attr(coverage, derive(Debug))]",
            "#![cfg_attr(coverage_nightly, feature(coverage_attribute))]",
            "#[cfg_attr(test, cfg(not(coverage)))]",
            "#[coverage(off)]",
            "#![coverage(off)]",
            "#[core::prelude::coverage(off)]",
        ] {
            let text = source(&[attribute, "fn f() {}"]);
            assert_eq!(lines(&text), [Some(1)], "{attribute}");
        }
    }

    // AC: CE-08.8
    #[test]
    fn an_attribute_after_code_or_over_several_lines_is_found() {
        assert_eq!(
            lines(&source(&["fn a() {} #[cfg(not(coverage))] fn b() {}"])),
            [Some(1)]
        );
        let text = source(&[
            "fn a() {}",
            "#[cfg(",
            "    not(coverage)",
            ")]",
            "fn b() {}",
        ]);
        assert_eq!(lines(&text), [Some(2)]);
        let text = source(&["macro_rules! m { () => { #[cfg(not(coverage))] fn x() {} } }"]);
        assert_eq!(lines(&text), [Some(1)]);
    }

    // AC: CE-08.8
    #[test]
    fn other_cfgs_comments_and_strings_pass() {
        for text in [
            "#[cfg(not(test))]\nfn f() {}\n",
            "#[cfg(test)]\nmod tests {}\n",
            "#[cfg_attr(test, derive(Debug))]\nstruct S;\n",
            "#[cfg(feature = \"coverage\")]\nfn f() {}\n",
            "#[doc = \"use cfg(coverage) to ...\"]\nfn f() {}\n",
            "/// Not `#[cfg(not(coverage))]`.\nfn f() {}\n",
            "// #[cfg(coverage)]\nfn f() {}\n",
            "/* #[coverage(off)] */\nfn f() {}\n",
            "const S: &str = \"#[cfg(coverage)]\";\n",
            "fn f() -> bool { cfg!(coverage) }\n",
            "fn coverage() {}\nfn g() { coverage(); }\n",
        ] {
            assert_eq!(lines(text), Vec::<Option<u32>>::new(), "{text}");
        }
    }
}
