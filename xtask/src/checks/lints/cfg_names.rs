//! The cfg rule of the `lints` check: `cfg` and `cfg_attr` may use only names that every check
//! builds the same way, and no attribute takes code out of the coverage floor.
//!
//! Each check builds the workspace under its own cfgs: clippy sets `clippy`, cargo-llvm-cov sets
//! `coverage`, a profile sets `debug_assertions`, `overflow_checks`, `ub_checks` and `panic`, and
//! a `build.rs` can set any name. Code under `#[cfg(not(clippy))]` is never seen by clippy and
//! `#[cfg(not(coverage))]` is never counted, while both ship. A list of forbidden names loses to
//! the next name, so the rule is an allowlist: `test`, `doc`, `unix`, `windows`,
//! `feature = "…"` and the keys that start with `target_` (`target_os = "…"` and the like),
//! combined with `all`, `any` and `not`. Anything else, or a predicate that cannot be read, is a
//! finding. An attribute whose path is `coverage` (`#[coverage(off)]`) is a finding too.
//!
//! `cfg!(…)` in an expression is not an attribute: both of its branches are compiled and
//! measured, so it is out of scope.

use super::attribute_reader::{Attribute, read_attributes};
use super::tokens::Token;
use crate::verdict::Finding;

/// The longest name that a finding repeats.
const NAME_LIMIT: usize = 40;

/// What is wrong with an attribute.
enum Problem {
    /// A cfg name that is not allowed; it is a word of letters, digits and `_`.
    Name(String),
    /// A predicate or an attribute that cannot be read.
    Syntax,
}

/// The findings of the cfg rule for the Rust source `text` of the file `file`, in file order.
///
/// Every attribute is read ([`read_attributes`]), so one after other code on a line, one over
/// several lines and one in a macro body are found; words in comments and strings do not count.
/// For `cfg(…)` and `cfg_attr(…)` the predicate is parsed (also in a `cfg` nested in a
/// `cfg_attr`); a name outside the allowlist, or a predicate that does not parse, is a finding.
#[must_use]
pub fn scan_cfg_names(file: &str, text: &str) -> Vec<Finding> {
    read_attributes(text)
        .iter()
        .filter_map(|attribute| {
            let message = problem(attribute)?;
            Some(Finding::at(file, attribute.line, message))
        })
        .collect()
}

/// The message for `attribute`, if it breaks the rule.
fn problem(attribute: &Attribute<'_>) -> Option<String> {
    match check_attribute(&attribute.body) {
        Ok(()) => None,
        Err(Problem::Name(name)) => {
            let name: String = name.chars().take(NAME_LIMIT).collect();
            Some(format!(
                "`{name}` in a `cfg(…)` or `cfg_attr(…)` attribute, or a `coverage` attribute: \
                 only `test`, `doc`, `unix`, `windows`, `feature = \"…\"` and `target_*` may be \
                 used, with `all`, `any` and `not`, so that every check builds and measures the \
                 same code; another cfg needs a gate change approved by Jann"
            ))
        }
        Err(Problem::Syntax) => Some(
            "a `cfg(…)` or `cfg_attr(…)` predicate that cannot be read: only `test`, `doc`, \
             `unix`, `windows`, `feature = \"…\"` and `target_* = \"…\"` may be used, with `all`, \
             `any` and `not`; another cfg needs a gate change approved by Jann"
                .to_owned(),
        ),
    }
}

/// Judges one attribute body: a `coverage` path, `cfg(…)`, and `cfg_attr(…)` with the attributes
/// it applies, which are judged the same way.
fn check_attribute(body: &[Token<'_>]) -> Result<(), Problem> {
    // The path: words and `::` up to the first token that is neither.
    let after = body
        .iter()
        .position(|token| !matches!(token, Token::Word(_) | Token::Symbol(b':')))
        .unwrap_or(body.len());
    let words = body[..after].iter().filter_map(|token| match token {
        Token::Word(word) => Some(*word),
        _ => None,
    });
    let mut last = None;
    for word in words {
        if word == "coverage" {
            return Err(Problem::Name("coverage".to_owned()));
        }
        last = Some(word);
    }
    if last != Some("cfg") && last != Some("cfg_attr") {
        return Ok(());
    }
    let [Token::Symbol(b'('), inner @ .., Token::Symbol(b')')] = &body[after..] else {
        return Err(Problem::Syntax);
    };
    let rest = predicate(inner)?;
    if last == Some("cfg") {
        return if rest.is_empty() {
            Ok(())
        } else {
            Err(Problem::Syntax)
        };
    }
    match rest {
        [] => Ok(()),
        [Token::Symbol(b','), items @ ..] => {
            split_items(items).into_iter().try_for_each(check_attribute)
        }
        _ => Err(Problem::Syntax),
    }
}

/// `tokens` split at the commas that are not inside brackets; empty pieces are left out.
fn split_items<'a, 't>(tokens: &'a [Token<'t>]) -> Vec<&'a [Token<'t>]> {
    let mut items = Vec::new();
    let mut depth = 0_usize;
    let mut start = 0;
    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Symbol(b'(' | b'[' | b'{') => depth += 1,
            Token::Symbol(b')' | b']' | b'}') => depth = depth.saturating_sub(1),
            Token::Symbol(b',') if depth == 0 => {
                items.push(&tokens[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    items.push(&tokens[start..]);
    items.retain(|item| !item.is_empty());
    items
}

/// Parses one predicate at the start of `tokens` and returns what follows it.
fn predicate<'a, 't>(tokens: &'a [Token<'t>]) -> Result<&'a [Token<'t>], Problem> {
    match tokens {
        [Token::Word("all" | "any"), Token::Symbol(b'('), rest @ ..] => {
            let mut rest = rest;
            loop {
                match rest {
                    [Token::Symbol(b')'), tail @ ..] => return Ok(tail),
                    [] => return Err(Problem::Syntax),
                    _ => {}
                }
                rest = predicate(rest)?;
                if let [Token::Symbol(b','), tail @ ..] = rest {
                    rest = tail;
                }
            }
        }
        [Token::Word("not"), Token::Symbol(b'('), rest @ ..] => match predicate(rest)? {
            [Token::Symbol(b')'), tail @ ..] => Ok(tail),
            _ => Err(Problem::Syntax),
        },
        [
            Token::Word("feature"),
            Token::Symbol(b'='),
            Token::Text(_),
            rest @ ..,
        ] => Ok(rest),
        [
            Token::Word(key),
            Token::Symbol(b'='),
            Token::Text(_),
            rest @ ..,
        ] if key.starts_with("target_") => Ok(rest),
        [Token::Word("test" | "doc" | "unix" | "windows"), rest @ ..] => Ok(rest),
        // Allowed names in the wrong shape.
        [Token::Word("all" | "any" | "not" | "feature"), ..] => Err(Problem::Syntax),
        [Token::Word(key), ..] if key.starts_with("target_") => Err(Problem::Syntax),
        [Token::Word(name), ..] => Err(Problem::Name((*name).to_owned())),
        _ => Err(Problem::Syntax),
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{lines_of, source};
    use super::*;

    fn lines(text: &str) -> Vec<Option<u32>> {
        lines_of(&scan_cfg_names("a.rs", text))
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

    // AC: CE-08.8
    #[test]
    fn names_outside_the_allowlist_are_found() {
        for attribute in [
            "#[cfg(not(clippy))]",
            "#[cfg(clippy)]",
            "#[cfg(debug_assertions)]",
            "#[cfg(not(debug_assertions))]",
            "#[cfg(any(test, miri))]",
            "#[cfg_attr(overflow_checks, inline)]",
            "#[cfg(ub_checks)]",
            "#[cfg(panic = \"abort\")]",
            "#[cfg(my_flag)]",
            "#[cfg(all(unix, not(docsrs)))]",
            "#[cfg(true)]",
            "#[cfg_attr(test, cfg(not(clippy)))]",
            "#[cfg_attr(test, derive(Debug), cfg(miri))]",
            "#![cfg(my_flag)]",
        ] {
            let text = source(&[attribute, "fn f() {}"]);
            assert_eq!(lines(&text), [Some(1)], "{attribute}");
        }
        let findings = scan_cfg_names(
            "a.rs",
            "#[cfg(not(clippy))] fn f() {}
",
        );
        assert!(findings[0].message.contains("`clippy`"), "{findings:?}");
        assert!(findings[0].message.contains("gate change"), "{findings:?}");
    }

    // AC: CE-08.8
    #[test]
    fn names_in_the_allowlist_pass() {
        for attribute in [
            "#[cfg(test)]",
            "#[cfg(not(test))]",
            "#[cfg(doc)]",
            "#[cfg(unix)]",
            "#[cfg(windows)]",
            "#[cfg(all(unix, feature = \"x\"))]",
            "#[cfg(any())]",
            "#[cfg(all(test, ))]",
            "#[cfg(target_os = \"windows\")]",
            "#[cfg(target_family = \"unix\")]",
            "#[cfg(target_arch = \"x86_64\")]",
            "#[cfg(target_env = \"msvc\")]",
            "#[cfg(target_pointer_width = \"64\")]",
            "#[cfg(target_endian = \"little\")]",
            "#[cfg(target_vendor = \"apple\")]",
            "#[cfg(target_has_atomic = \"64\")]",
            "#[cfg(any(windows, all(not(unix), target_os = \"macos\")))]",
            "#[cfg_attr(test, derive(Debug))]",
            "#[cfg_attr(feature = \"x\", derive(Debug), derive(Clone),)]",
            "#[cfg_attr(unix, cfg(test))]",
            "#[cfg_attr(test)]",
            "#[cfg(feature = r#\"x\"#)]",
        ] {
            let text = source(&[attribute, "fn f() {}"]);
            assert_eq!(lines(&text), Vec::<Option<u32>>::new(), "{attribute}");
        }
    }

    // AC: CE-08.8
    #[test]
    fn a_predicate_that_cannot_be_read_is_found() {
        for attribute in [
            "#[cfg]",
            "#[cfg()]",
            "#[cfg(test, unix)]",
            "#[cfg(feature)]",
            "#[cfg(feature = x)]",
            "#[cfg(target_os)]",
            "#[cfg(target_os = windows)]",
            "#[cfg(not())]",
            "#[cfg(not(test, unix))]",
            "#[cfg(all(test)",
            "#[cfg(test = \"x\")]",
            "#[cfg($pred)]",
            "#[cfg_attr()]",
            "#[cfg_attr(unix unix, derive(Debug))]",
        ] {
            let text = source(&[attribute, "fn f() {}"]);
            assert_eq!(lines(&text), [Some(1)], "{attribute}");
        }
    }

    // AC: CE-08.8
    #[test]
    fn a_finding_names_a_word_and_never_other_text() {
        let findings = scan_cfg_names(
            "a.rs",
            "#[cfg(not(clippy_::x))] fn f() {}
",
        );
        assert!(!findings[0].message.contains("::"), "{findings:?}");
        let long = "a".repeat(200);
        let findings = scan_cfg_names(
            "a.rs",
            &format!(
                "#[cfg({long})] fn f() {{}}
"
            ),
        );
        assert!(findings[0].message.len() < 600, "{findings:?}");
        assert!(!findings[0].message.contains(&long), "{findings:?}");
    }
}
