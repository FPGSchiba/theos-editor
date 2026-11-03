//! The attribute scan of the `lints` check: an `allow(…)`, an `expect(…)` without a reason and
//! an `ignore` without one, in the attributes of a Rust source, and a `path` of an attribute that
//! leads into a fixtures folder (which `fixture_routes.rs` judges).

use super::attribute_reader::read_attributes;
use super::fixture_routes::path_problem;
use super::tokens::Token;
use crate::verdict::Finding;

const ALLOW_FOUND: &str = "`allow(…)` in an attribute: a lint is not silenced; fix the cause, or \
                           use `#[expect(…, reason = \"…\")]` for a lint that is expected";
const EXPECT_FOUND: &str = "`expect(…)` without `reason = \"…\"`: say why the lint is expected";
const IGNORE_FOUND: &str = "`ignore` without a reason: write `#[ignore = \"…\"]`";

/// The findings for the attributes of the Rust source `text` of the file `file`, in file order.
///
/// An attribute is found when it has `allow(` (also inside `cfg_attr`), when it has `expect(`
/// and no `reason = "…"` with something in it, or when it is `ignore` (also inside `cfg_attr`)
/// and has no `= "…"` with something in it. A `path = …` of an attribute (also inside `cfg_attr`)
/// is found when it leads into a `tests/fixtures/` folder, out of the `src/` of its crate, or
/// cannot be followed. The attributes come from the token stream (`attribute_reader.rs`): inner and
/// outer, anywhere in a line, over several lines, in function and macro bodies. Only whole words
/// count, and words in comments and strings do not.
#[must_use]
pub fn scan_attributes(file: &str, text: &str) -> Vec<Finding> {
    read_attributes(text)
        .iter()
        .flat_map(|attribute| judge(file, attribute.line, &attribute.body))
        .collect()
}

/// The findings for the attribute with the tokens `tokens`, which starts on `line`.
fn judge(file: &str, line: u32, tokens: &[Token<'_>]) -> Vec<Finding> {
    // The first word is the name of the attribute: `ignore`, or `cfg_attr` that has `ignore` in it.
    let name = tokens
        .iter()
        .position(|token| matches!(token, Token::Word(_)));
    let conditional = name.is_some_and(|index| tokens[index] == Token::Word("cfg_attr"));
    let mut findings = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        let Token::Word(word) = token else {
            continue;
        };
        let after = &tokens[index + 1..];
        let opens = after.first() == Some(&Token::Symbol(b'('));
        let message = match *word {
            "allow" if opens => Some(ALLOW_FOUND),
            "expect" if opens && !has_reason(&after[1..]) => Some(EXPECT_FOUND),
            "ignore" if (name == Some(index) || conditional) && !has_value(after) => {
                Some(IGNORE_FOUND)
            }
            "path"
                if (name == Some(index) || conditional)
                    && after.first() == Some(&Token::Symbol(b'=')) =>
            {
                path_problem(file, &after[1..])
            }
            _ => None,
        };
        findings.extend(message.map(|message| Finding::at(file, line, message)));
    }
    findings
}

/// Whether `tokens` start with `= "…"`, a string that says something.
fn has_value(tokens: &[Token<'_>]) -> bool {
    matches!(tokens, [Token::Symbol(b'='), Token::Text(text), ..] if !is_blank(text))
}

/// Whether the arguments `tokens` of an `expect(`, up to the closing bracket, have a
/// `reason = "…"` that says something.
fn has_reason(tokens: &[Token<'_>]) -> bool {
    let mut depth = 1_usize;
    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Symbol(b'(') => depth += 1,
            Token::Symbol(b')') => {
                depth -= 1;
                if depth == 0 {
                    return false;
                }
            }
            Token::Word("reason") if has_value(&tokens[index + 1..]) => return true,
            _ => {}
        }
    }
    false
}

/// Whether the string literal `literal`, as written, has nothing in it but white space.
fn is_blank(literal: &str) -> bool {
    match (literal.find('"'), literal.rfind('"')) {
        (Some(open), Some(close)) if open < close => literal[open + 1..close].trim().is_empty(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{lines_of, source};
    use super::*;

    /// The findings for the source file made of `lines`.
    fn scan(lines: &[&str]) -> Vec<Finding> {
        scan_attributes("src/lib.rs", &source(lines))
    }

    #[test]
    fn allow_attribute_found() {
        let findings = scan(&["#[allow(dead_code)]", "fn f() {}"]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].file.as_deref(), Some("src/lib.rs"));
        assert_eq!(findings[0].line, Some(1));
        assert!(findings[0].message.contains("allow"), "{findings:?}");

        let findings = scan(&[
            "mod m {",
            "    #[allow(clippy::too_many_lines, dead_code)]",
            "    fn f() {}",
            "}",
        ]);
        assert_eq!(lines_of(&findings), [Some(2)]);
    }

    #[test]
    fn inner_allow_found() {
        let findings = scan(&["//! Docs.", "", "#![allow(missing_docs)]"]);
        assert_eq!(lines_of(&findings), [Some(3)]);
        let findings = scan(&["mod m {", "    #![allow(dead_code)]", "}"]);
        assert_eq!(lines_of(&findings), [Some(2)]);
    }

    #[test]
    fn cfg_attr_allow_found() {
        let findings = scan(&["#[cfg_attr(test, allow(dead_code))]", "fn f() {}"]);
        assert_eq!(lines_of(&findings), [Some(1)]);
        let findings = scan(&[
            "#[cfg_attr(",
            "    test,",
            "    allow(dead_code)",
            ")]",
            "fn f() {}",
        ]);
        assert_eq!(lines_of(&findings), [Some(1)]);
    }

    #[test]
    fn expect_without_reason_found() {
        for attribute in [
            "#[expect(dead_code)]",
            "#![expect(clippy::too_many_lines)]",
            "#[cfg_attr(test, expect(dead_code))]",
            "#[expect(dead_code, because = \"x\")]",
        ] {
            let findings = scan(&[attribute, "fn f() {}"]);
            assert_eq!(findings.len(), 1, "{attribute}: {findings:?}");
            assert_eq!(findings[0].line, Some(1), "{attribute}");
            assert!(findings[0].message.contains("expect"), "{attribute}");
        }
    }

    #[test]
    fn expect_with_reason_passes() {
        for attribute in [
            "#[expect(dead_code, reason = \"used by the tests\")]",
            "#![expect(clippy::too_many_lines, reason = \"one table\")]",
            "#[expect(dead_code, reason=\"no spaces\")]",
            "#[cfg_attr(test, expect(dead_code, reason = \"only tests use it\"))]",
            "#[expect(dead_code, reason = r#\"a \"raw\" reason\"#)]",
        ] {
            let findings = scan(&[attribute, "fn f() {}"]);
            assert!(findings.is_empty(), "{attribute}: {findings:?}");
        }
    }

    #[test]
    fn bare_ignore_found() {
        for attribute in [
            "#[ignore]",
            "#[cfg_attr(windows, ignore)]",
            "#[cfg_attr(not(windows), ignore)]",
        ] {
            let findings = scan(&["#[test]", attribute, "fn f() {}"]);
            assert_eq!(findings.len(), 1, "{attribute}: {findings:?}");
            assert_eq!(findings[0].line, Some(2), "{attribute}");
            assert!(findings[0].message.contains("ignore"), "{attribute}");
        }
    }

    #[test]
    fn ignore_with_reason_passes() {
        for attribute in [
            "#[ignore = \"needs the network\"]",
            "#[ignore=\"no spaces\"]",
            "#[cfg_attr(windows, ignore = \"slow on windows\")]",
            "#[cfg_attr(not(feature = \"slow\"), ignore = \"slow\")]",
        ] {
            let findings = scan(&["#[test]", attribute, "fn f() {}"]);
            assert!(findings.is_empty(), "{attribute}: {findings:?}");
        }
    }

    #[test]
    fn allow_in_doc_comment_ignored() {
        let findings = scan(&[
            "/// #[allow(dead_code)]",
            "//! #![allow(dead_code)]",
            "// #[allow(dead_code)]",
            "    /// #[allow(dead_code)]",
            "    // #[expect(dead_code)]",
            "let text = \"#[allow(dead_code)]\";",
            "fn f() {}",
        ]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn multi_line_attribute_joined() {
        // The reason is on a later line than the `expect(`.
        let findings = scan(&[
            "#[expect(",
            "    dead_code,",
            "    reason = \"used by the tests\"",
            ")]",
            "fn f() {}",
        ]);
        assert!(findings.is_empty(), "{findings:?}");

        // No reason on any of the lines: the finding is at the first line.
        let findings = scan(&["fn a() {}", "#[expect(", "    dead_code", ")]", "fn f() {}"]);
        assert_eq!(lines_of(&findings), [Some(2)]);

        // A bracket in a string does not end the attribute early.
        let findings = scan(&[
            "#[doc = \"a ] bracket\"]",
            "#[cfg_attr(",
            "    test,",
            "    doc = \"another ] one\",",
            "    allow(dead_code)",
            ")]",
        ]);
        assert_eq!(lines_of(&findings), [Some(2)]);

        // The next attribute is read from its own line.
        let findings = scan(&[
            "#[cfg_attr(",
            "    test,",
            "    derive(Debug)",
            ")]",
            "#[allow(dead_code)]",
            "struct S;",
        ]);
        assert_eq!(lines_of(&findings), [Some(5)]);
    }

    #[test]
    fn words_are_not_substrings() {
        let findings = scan(&[
            "#[disallow(x)]",
            "#[allow_internal_unstable(x)]",
            "#[should_panic(expected = \"x\")]",
            "#[cfg_attr(test, expected(x))]",
            "#[cfg(feature = \"ignore\")]",
            "#[derive(Debug)]",
            "#[doc = \"allow(x) and expect(y) and ignore\"]",
            "#[expect(dead_code, reason = \"the allow(x) rule\")]",
        ]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn comments_inside_an_attribute_do_not_count() {
        // A `reason` in a comment is no reason.
        let findings = scan(&[
            "#[expect(",
            "    dead_code, // reason = \"x\"",
            ")]",
            "#[expect(dead_code, /* reason = \"y\" */)]",
        ]);
        assert_eq!(lines_of(&findings), [Some(1), Some(4)]);
        // And an `allow(` in a comment is no allow.
        let findings = scan(&[
            "#[expect(",
            "    dead_code, // allow(x) would not do",
            "    reason = \"r\"",
            ")]",
        ]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_reason_must_say_something() {
        for attribute in [
            "#[expect(dead_code, reason = \"\")]",
            "#[expect(dead_code, reason = \"   \")]",
            "#[expect(dead_code, reason)]",
            "#[expect(dead_code, reason = reason)]",
            "#[ignore = \"\"]",
            "#[ignore = \" \"]",
            "#[cfg_attr(windows, ignore = \"\")]",
        ] {
            let findings = scan(&[attribute, "fn f() {}"]);
            assert_eq!(findings.len(), 1, "{attribute}: {findings:?}");
        }
    }

    #[test]
    fn every_attribute_on_a_line_is_read() {
        let findings = scan(&[
            "#[test] #[ignore] fn a() {}",
            "#[derive(Debug)] #[allow(dead_code)] struct S;",
            "#[test] #[ignore = \"slow\"] fn b() {}",
            "#[cfg_attr(test, derive(Debug))] #[expect(dead_code)]",
        ]);
        assert_eq!(lines_of(&findings), [Some(1), Some(2), Some(4)]);
    }

    #[test]
    fn an_attribute_that_is_never_closed_is_still_read() {
        let findings = scan(&["fn a() {}", "#[allow(dead_code"]);
        assert_eq!(lines_of(&findings), [Some(2)]);
    }

    #[test]
    fn findings_are_in_file_order_with_their_lines() {
        let findings = scan(&[
            "#[allow(a)]",
            "fn a() {}",
            "",
            "#[expect(b)]",
            "fn b() {}",
            "#[test]",
            "#[ignore]",
            "fn c() {}",
        ]);
        assert_eq!(lines_of(&findings), [Some(1), Some(4), Some(7)]);
    }

    #[test]
    fn crlf_line_ends_do_not_matter() {
        let findings = scan_attributes("a.rs", "#[allow(dead_code)]\r\nfn f() {}\r\n");
        assert_eq!(lines_of(&findings), [Some(1)]);
        let text = "#[expect(\r\n    dead_code,\r\n    reason = \"x\"\r\n)]\r\nfn f() {}\r\n";
        let findings = scan_attributes("a.rs", text);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn attributes_are_read_wherever_they_stand() {
        for text in [
            "#[rustfmt::skip] pub mod m { #![allow(clippy::unwrap_used)] pub fn f() {} }",
            "fn a() {} #[allow(dead_code)] fn b() {}",
            "fn f() { #[allow(unused)] let x = 1; }",
            "fn f() { #![allow(unused)] }",
            "macro_rules! m { () => { #[allow(dead_code)] fn x() {} } }",
            "macro_rules! m { () => { fn x() {} }; ($a:ident) => { #[allow(dead_code)] fn $a() {} } }",
            "#[allow(
    dead_code,
    unused
)]
fn f() {}",
            "#[cfg_attr(test, allow(dead_code))]
fn f() {}",
            "#[derive(Debug)] #[cfg_attr(unix, allow(dead_code))] struct S;",
            "mod m {
    #![allow(dead_code)]
}",
        ] {
            let findings = scan_attributes("a.rs", text);
            assert_eq!(findings.len(), 1, "{text}: {findings:?}");
        }
        let findings = scan(&["fn f() {", "    let a = 1; #[ignore] fn g() {}", "}"]);
        assert_eq!(lines_of(&findings), [Some(2)]);
        let findings = scan(&["fn f() { #[expect(dead_code)] let x = 1; }"]);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_path_attribute_is_read_wherever_it_stands() {
        let findings = scan_attributes(
            "crates/a/src/lib.rs",
            "pub fn a() {} #[path = \"../outside.rs\"] mod x;
",
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("src/"), "{findings:?}");
    }

    #[test]
    fn strings_comments_and_char_literals_never_make_an_attribute() {
        for text in [
            "const S: &str = \"#[allow(x)]\";",
            "const S: &str = r#\"#[allow(x)]\"#;",
            "const S: &str = r##\"a \"# #[allow(x)]\"##;",
            "// #[allow(x)]
fn f() {}",
            "/* #[allow(x)] */ fn f() {}",
            "/// #[allow(x)]
fn f() {}",
            "//! #![allow(x)]
fn f() {}",
            "fn f() -> char { '#' }",
            "fn f() { let c = b'['; let d = '\''; }",
        ] {
            let findings = scan_attributes("a.rs", text);
            assert!(findings.is_empty(), "{text}: {findings:?}");
        }
    }

    #[test]
    fn nested_block_comments_cannot_hide_an_attribute() {
        // rustc nests `/* */`: the quote below is inside the comment, so the attribute is code.
        let text = "/* /* */ \" */ #[allow(x)] fn f() {} // \"
";
        let findings = scan_attributes("a.rs", text);
        assert_eq!(findings.len(), 1, "{findings:?}");
        // And an attribute inside the nested comment is not one.
        let findings = scan_attributes(
            "a.rs",
            "/* a /* b */ #[allow(x)] */ fn f() {}
",
        );
        assert!(findings.is_empty(), "{findings:?}");
    }
}
