//! Tags in Rust sources: a `// AC: <id>` comment above a test.
//!
//! `syn` finds the tests (every `fn` with a test attribute, at any depth: inline modules and
//! function bodies included) and [`line_comments`] the `//` comments (a line inside a string is
//! not one), so nothing here lexes Rust. A tag is directly above its test when it is the last
//! `//` comment of a run of comment-only lines that ends on the line above the test's first
//! attribute or doc comment; a comment after an attribute on the same line is not in the way.
//! A test counts as proof unless it is switched off:
//!
//! - `#[ignore…]`, or a `#[cfg_attr(…, …ignore…)]` which may ignore it on some targets;
//! - a `#[cfg(…)]` whose predicate names anything but the target (`TARGET_WORDS`), or that is
//!   `any()`, which is always false.
//!
//! A file that no test target compiles proves nothing: its tags are dangling (Ruling 11).

use std::collections::BTreeMap;

use proc_macro2::{TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Attribute, ItemFn, Meta, Visibility};

use super::{Comment, Tag, classify, dangling, malformed};
use crate::verdict::Finding;
use crate::view::{LineComment, Source, line_comments, line_of};

/// What a predicate of `#[cfg(…)]` may name and still leave the test in every build of a target.
const TARGET_WORDS: [&str; 7] = [
    "windows",
    "unix",
    "target_os",
    "target_family",
    "any",
    "all",
    "not",
];

/// Finds the tags in the Rust file `source`; `in_test_target` says whether a test target compiles
/// it.
///
/// A comment that starts like a tag and is not one, and a tag that is not directly above a test,
/// are findings; so is every tag of a file that no test target compiles. A tag above a test that
/// is switched off is a [`Tag`] that does not count. A file that did not parse has none: the
/// view reports it.
#[must_use]
pub fn scan(source: &Source, in_test_target: bool) -> (Vec<Tag>, Vec<Finding>) {
    let (Some(file), Some(tokens)) = (&source.file, &source.tokens) else {
        return (Vec::new(), Vec::new());
    };
    let mut finder = TestFinder::default();
    finder.visit_file(file);
    let mut tags = Vec::new();
    let mut findings = Vec::new();
    for comment in line_comments(&source.text, tokens) {
        match classify(&comment.text) {
            Comment::Other => {}
            Comment::Malformed => findings.push(malformed(&source.path, comment.line)),
            Comment::Tag(ids) => {
                let test = finder.test_below(&comment);
                match (test, in_test_target) {
                    (Some(test), true) => tags.push(Tag {
                        file: source.path.clone(),
                        line: comment.line,
                        ids,
                        test: Some(test.name.clone()),
                        counts: test.counts,
                    }),
                    (_, false) => findings.push(dangling(
                        &source.path,
                        comment.line,
                        "the file is one that no test target compiles",
                    )),
                    (None, true) => findings.push(dangling(
                        &source.path,
                        comment.line,
                        "no test starts on the next line (the tag is the last comment line \
                         right above the test's attributes)",
                    )),
                }
            }
        }
    }
    (tags, findings)
}

/// A test function.
struct Test {
    name: String,
    counts: bool,
}

/// The tests of a file, by the line of their first attribute or doc comment.
#[derive(Default)]
struct TestFinder {
    tests: BTreeMap<u32, Test>,
}

impl TestFinder {
    /// The test that `comment` is directly above: it is alone on its line, and a test starts on
    /// the next line.
    fn test_below(&self, comment: &LineComment) -> Option<&Test> {
        if comment.alone {
            self.tests.get(&(comment.line + 1))
        } else {
            None
        }
    }
}

impl<'ast> Visit<'ast> for TestFinder {
    fn visit_item_fn(&mut self, function: &'ast ItemFn) {
        if let Some(first) = function.attrs.first()
            && function.attrs.iter().any(is_test_attribute)
            && !has_gap(function)
        {
            self.tests
                .entry(line_of(first.span()))
                .or_insert_with(|| Test {
                    name: function.sig.ident.to_string(),
                    counts: function.attrs.iter().all(keeps_test_running),
                });
        }
        syn::visit::visit_item_fn(self, function);
    }
}

/// Whether a blank line (or more) separates the attributes and doc comments of `function`
/// from each other or from the function: such a tag is not directly above its test.
fn has_gap(function: &ItemFn) -> bool {
    let item = if matches!(function.vis, Visibility::Inherited) {
        function.sig.span()
    } else {
        function.vis.span()
    };
    let mut previous_end: Option<usize> = None;
    let lines = function
        .attrs
        .iter()
        .map(Spanned::span)
        .chain([item])
        .map(|span| (span.start().line, span.end().line));
    for (start, end) in lines {
        if previous_end.is_some_and(|previous| start > previous + 1) {
            return true;
        }
        previous_end = Some(end);
    }
    false
}

/// Whether the last segment of the attribute's path is `test`, `rstest` or `test_case`.
fn is_test_attribute(attribute: &Attribute) -> bool {
    attribute.path().segments.last().is_some_and(|segment| {
        matches!(
            segment.ident.to_string().as_str(),
            "test" | "rstest" | "test_case"
        )
    })
}

/// Whether the attribute leaves its test running: it is not `ignore`, no `cfg_attr` can add an
/// `ignore`, and a `cfg` names only the target.
fn keeps_test_running(attribute: &Attribute) -> bool {
    let path = attribute.path();
    if path.is_ident("ignore") {
        return false;
    }
    let Meta::List(list) = &attribute.meta else {
        return true;
    };
    if path.is_ident("cfg_attr") {
        !has_ident(&list.tokens, |word| word == "ignore")
    } else if path.is_ident("cfg") {
        names_only_the_target(&list.tokens)
    } else {
        true
    }
}

/// Whether the predicate `tokens` of a `cfg` names only the target and the combinators, and
/// has no `any()`.
fn names_only_the_target(tokens: &TokenStream) -> bool {
    let trees: Vec<TokenTree> = tokens.clone().into_iter().collect();
    trees.iter().enumerate().all(|(index, tree)| match tree {
        TokenTree::Ident(word) => {
            TARGET_WORDS.contains(&word.to_string().as_str())
                && !(word == "any"
                    && matches!(trees.get(index + 1),
                        Some(TokenTree::Group(group)) if group.stream().is_empty()))
        }
        TokenTree::Group(group) => names_only_the_target(&group.stream()),
        TokenTree::Punct(_) | TokenTree::Literal(_) => true,
    })
}

/// Whether some identifier of `tokens`, at any depth, satisfies `test`. Strings are literals, so
/// the words in them are not identifiers.
fn has_ident(tokens: &TokenStream, test: fn(&str) -> bool) -> bool {
    tokens.clone().into_iter().any(|tree| match tree {
        TokenTree::Ident(word) => test(&word.to_string()),
        TokenTree::Group(group) => has_ident(&group.stream(), test),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::parse_source;

    /// The lines joined into the text of a source file.
    fn source(lines: &[&str]) -> String {
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }

    /// Scans `text` as `src/lib.rs`, which a test target compiles when `in_test_target`.
    fn scan_text(text: &str, in_test_target: bool) -> (Vec<Tag>, Vec<Finding>) {
        let (source, finding) = parse_source("src/lib.rs", text.as_bytes());
        assert!(finding.is_none(), "{finding:?}");
        scan(&source, in_test_target)
    }

    fn scan_lines(lines: &[&str]) -> (Vec<Tag>, Vec<Finding>) {
        scan_text(&source(lines), true)
    }

    /// The tag of a scan that found exactly one tag and nothing else.
    fn only_tag(lines: &[&str]) -> Tag {
        let (mut tags, findings) = scan_lines(lines);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(tags.len(), 1, "{tags:?}");
        tags.remove(0)
    }

    /// The tag above `attributes` and a function, and whether it counts.
    fn counts(attributes: &[&str]) -> bool {
        let mut lines = vec!["// AC: CE-08.1"];
        lines.extend_from_slice(attributes);
        lines.push("fn a_test() {}");
        only_tag(&lines).counts
    }

    // AC: CE-08.6
    #[test]
    fn rust_tag_above_test_counts() {
        let tag = only_tag(&["// AC: CE-08.1", "#[test]", "fn works() {}"]);
        assert_eq!(tag.file, "src/lib.rs");
        assert_eq!(tag.line, 1);
        assert_eq!(tag.ids, ["CE-08.1"]);
        assert_eq!(tag.test.as_deref(), Some("works"));
        assert!(tag.counts);
    }

    // AC: CE-08.6
    #[test]
    fn rust_tag_with_doc_and_attrs_counts() {
        let tag = only_tag(&[
            "mod tests {",
            "    // AC: CE-08.1, CE-08.2",
            "    /// What it proves.",
            "    #[tokio::test(flavor = \"multi_thread\")]",
            "    #[should_panic(",
            "        expected = \"a ] bracket\"",
            "    )]",
            "    pub async fn proves_it() {}",
            "}",
        ]);
        assert_eq!(tag.line, 2);
        assert_eq!(tag.ids, ["CE-08.1", "CE-08.2"]);
        assert_eq!(tag.test.as_deref(), Some("proves_it"));
        assert!(tag.counts);
    }

    #[test]
    fn the_attribute_and_the_function_may_share_a_line() {
        let tag = only_tag(&["// AC: CE-08.1", "#[test] fn one_line() {}"]);
        assert_eq!(tag.test.as_deref(), Some("one_line"));
        let tag = only_tag(&[
            "// AC: CE-08.1",
            "#[test]",
            "#[ignore] pub(crate) fn two() {}",
        ]);
        assert_eq!((tag.test.as_deref(), tag.counts), (Some("two"), false));
    }

    #[test]
    fn crlf_line_ends_do_not_matter() {
        let text = "// AC: CE-08.1\r\n#[test]\r\nfn works() {}\r\n";
        let (tags, findings) = scan_text(text, true);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].line, 1);
    }

    #[test]
    fn test_attributes_are_recognised_by_their_last_path_segment() {
        for attribute in [
            "#[test]",
            "#[tokio::test]",
            "#[tokio::test(flavor = \"current_thread\")]",
            "#[rstest]",
            "#[test_case(1, 2)]",
        ] {
            assert!(counts(&[attribute]), "{attribute}");
        }
    }

    // AC: CE-08.6
    #[test]
    fn rust_tag_above_helper_is_dangling() {
        for lines in [
            &["// AC: CE-08.1", "fn helper() {}"][..],
            &["// AC: CE-08.1", "#[inline]", "fn helper() {}"],
            &["// AC: CE-08.1", "#[derive(Debug)]", "struct Case;"],
            &["// AC: CE-08.1", "#[test]", "struct NotAFunction;"],
            &["// AC: CE-08.1"],
            &[
                "// AC: CE-08.1",
                "fn helper() {}",
                "#[test]",
                "fn works() {}",
            ],
        ] {
            let (tags, findings) = scan_lines(lines);
            assert!(tags.is_empty(), "{lines:?}: {tags:?}");
            assert_eq!(findings.len(), 1, "{lines:?}: {findings:?}");
            assert_eq!(findings[0].file.as_deref(), Some("src/lib.rs"));
            assert_eq!(findings[0].line, Some(1));
        }
    }

    // AC: CE-08.6
    #[test]
    fn rust_tag_with_blank_line_is_dangling() {
        for lines in [
            &["// AC: CE-08.1", "", "#[test]", "fn works() {}"][..],
            &["// AC: CE-08.1", "#[test]", "", "fn works() {}"],
            &[
                "// AC: CE-08.1",
                "// a plain comment",
                "#[test]",
                "fn works() {}",
            ],
            &[
                "// AC: CE-08.1",
                "//// not a doc line",
                "#[test]",
                "fn works() {}",
            ],
        ] {
            let (tags, findings) = scan_lines(lines);
            assert!(tags.is_empty(), "{lines:?}: {tags:?}");
            assert_eq!(findings.len(), 1, "{lines:?}: {findings:?}");
        }
    }

    // AC: CE-08.6
    #[test]
    fn ignored_test_does_not_count() {
        assert!(!counts(&["#[test]", "#[ignore]"]));
        assert!(!counts(&["#[test]", "#[ignore = \"slow\"]"]));
        assert!(!counts(&["#[ignore = \"slow\"]", "#[test]"]));
        assert!(!counts(&["#[test]", "#[cfg_attr(windows, ignore)]"]));
        assert!(!counts(&[
            "#[test]",
            "#[cfg_attr(not(windows), ignore = \"x\")]"
        ]));
        // Other conditional attributes do not stop a test from running.
        assert!(counts(&["#[test]", "#[cfg_attr(windows, should_panic)]"]));
    }

    // AC: CE-08.6
    #[test]
    fn cfg_any_test_does_not_count_cfg_windows_does() {
        let cases = [
            ("#[cfg(any(test, feature = \"x\"))]", false),
            ("#[cfg(feature = \"x\")]", false),
            ("#[cfg(test)]", false),
            ("#[cfg(any())]", false),
            ("#[cfg(all(unix, debug_assertions))]", false),
            ("#[cfg(windows)]", true),
            ("#[cfg(unix)]", true),
            ("#[cfg(not(windows))]", true),
            ("#[cfg(target_os = \"linux\")]", true),
            ("#[cfg(all(unix, target_family = \"unix\"))]", true),
            ("#[cfg(any(windows, target_os = \"macos\"))]", true),
            ("#[cfg(\n    windows\n)]", true),
        ];
        for (attribute, expected) in cases {
            let lines: Vec<&str> = attribute.lines().collect();
            for order in [[&lines[..], &["#[test]"]], [&["#[test]"], &lines[..]]] {
                let attributes: Vec<&str> = order.concat();
                assert_eq!(counts(&attributes), expected, "{attribute}");
            }
        }
    }

    // AC: CE-08.6
    #[test]
    fn malformed_tags_are_findings() {
        for line in [
            "//AC: x",
            "// AC - CE-08.1",
            "// AC: CE-08.1,CE-08.2",
            "// AC:CE-08.1",
            "// AC: ",
            "// AC:",
            "//  AC: CE-08.1",
            "// AC: CE-08.1 and more",
            "// AC: CE-08.1, ",
            "// AC: CE 08.1",
            "\t//AC",
        ] {
            let (tags, findings) = scan_lines(&[line, "#[test]", "fn works() {}"]);
            assert!(tags.is_empty(), "{line}: {tags:?}");
            assert_eq!(findings.len(), 1, "{line}: {findings:?}");
            assert_eq!(findings[0].line, Some(1), "{line}");
        }
    }

    #[test]
    fn comments_that_are_not_tags_are_left_alone() {
        let (tags, findings) = scan_lines(&[
            "//! AC: CE-08.1",
            "// ACs are acceptance criteria",
            "// ACL rules",
            "// the AC: CE-08.1 tag",
            "fn helper() {",
            "    let text = \"// AC: CE-08.1\";",
            "    drop(text);",
            "}",
            "/// AC: CE-08.1",
            "#[test]",
            "fn works() {}",
        ]);
        assert!(tags.is_empty(), "{tags:?}");
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn every_tag_of_a_file_is_found_with_its_line() {
        let (tags, findings) = scan_lines(&[
            "// AC: CE-08.1",
            "#[test]",
            "fn first() {}",
            "",
            "    // AC: CE-08.2",
            "    #[test]",
            "    fn second() {}",
            "// AC: CE-08.3",
            "// AC: CE-08.4",
            "#[test]",
            "fn third() {}",
        ]);
        let found: Vec<(u32, &str)> = tags
            .iter()
            .map(|tag| (tag.line, tag.ids[0].as_str()))
            .collect();
        assert_eq!(found, [(1, "CE-08.1"), (5, "CE-08.2"), (9, "CE-08.4")]);
        // The first of two tag lines in a row is not directly above the test.
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].line, Some(8));
    }

    // AC: CE-08.6
    #[test]
    fn a_tag_line_inside_a_string_is_not_a_tag() {
        let (tags, findings) = scan_lines(&[
            "const S: &str = \"",
            "// AC: CE-08.1",
            "//AC: broken",
            "\";",
            "",
            "// AC: CE-08.2",
            "#[test]",
            "fn works() {}",
        ]);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(tags.len(), 1, "{tags:?}");
        assert_eq!(
            (tags[0].line, tags[0].ids.as_slice()),
            (6, &["CE-08.2".to_owned()][..])
        );
    }

    // AC: CE-08.6
    #[test]
    fn a_tag_above_a_test_in_an_inline_module_counts() {
        let tag = only_tag(&[
            "#[cfg(test)]",
            "mod tests {",
            "    // AC: CE-08.1",
            "    #[test]",
            "    fn inner() {}",
            "}",
        ]);
        assert_eq!(
            (tag.line, tag.test.as_deref(), tag.counts),
            (3, Some("inner"), true)
        );
    }

    // AC: CE-08.6
    #[test]
    fn a_tag_above_a_test_in_a_function_body_counts() {
        let tag = only_tag(&[
            "fn outer() {",
            "    // AC: CE-08.1",
            "    #[test]",
            "    fn inner() {}",
            "}",
        ]);
        assert_eq!(
            (tag.line, tag.test.as_deref(), tag.counts),
            (2, Some("inner"), true)
        );
    }

    // AC: CE-08.6
    #[test]
    fn a_trailing_comment_after_an_attribute_keeps_the_tag() {
        let tag = only_tag(&["// AC: CE-08.1", "#[test] // slow", "fn works() {}"]);
        assert_eq!(
            (tag.line, tag.test.as_deref(), tag.counts),
            (1, Some("works"), true)
        );
    }

    // AC: CE-08.6
    #[test]
    fn tags_in_a_file_no_test_target_compiles_are_dangling() {
        let text = source(&["// AC: CE-08.1", "#[test]", "fn works() {}", "//AC: broken"]);
        let (tags, findings) = scan_text(&text, false);
        assert!(tags.is_empty(), "{tags:?}");
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert_eq!(findings[0].line, Some(1));
        assert!(
            findings[0].message.contains("no test target compiles"),
            "{findings:?}"
        );
        // A malformed tag is a finding wherever it is.
        assert_eq!(findings[1].line, Some(4));
    }
}
