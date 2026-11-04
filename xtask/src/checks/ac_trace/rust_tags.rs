//! Tags in Rust sources: a `// AC: <id>` comment above a test.
//!
//! After the tag only attribute lines and `///` lines may come before the `fn` line, so the tag
//! is directly above its test. An attribute runs until its brackets balance, which may take
//! several lines. The function must have a test attribute: `#[test]`, `#[<path>::test…]`,
//! `#[rstest…]` or `#[test_case…]`. The tag does not count when the test is switched off:
//!
//! - `#[ignore…]`, or `#[cfg_attr(…, ignore…)]` which ignores it on some targets;
//! - a `#[cfg(…)]` whose predicate names anything but the target (`windows`, `unix`,
//!   `target_os`, `target_family`) and the combinators `any`, `all` and `not`, or that is
//!   `any()`, which is always false.

use super::{Comment, Tag, classify, dangling, line_number, malformed};
use crate::verdict::Finding;

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

/// Finds the tags in the Rust source `text` of the file `file`.
///
/// A line that starts like a tag and is not one, and a tag that is not directly above a test,
/// are findings. A tag above a test that is switched off is a [`Tag`] that does not count.
#[must_use]
pub fn scan(file: &str, text: &str) -> (Vec<Tag>, Vec<Finding>) {
    let lines: Vec<&str> = text.lines().collect();
    let mut tags = Vec::new();
    let mut findings = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let number = line_number(index);
        match classify(line) {
            Comment::Other => {}
            Comment::Malformed => findings.push(malformed(file, number)),
            Comment::Tag(ids) => match target(&lines, index + 1) {
                Ok(Test { name, counts }) => tags.push(Tag {
                    file: file.to_owned(),
                    line: number,
                    ids,
                    test: Some(name),
                    counts,
                }),
                Err(why) => findings.push(dangling(file, number, &why)),
            },
        }
    }
    (tags, findings)
}

/// The test a tag is above.
struct Test {
    name: String,
    counts: bool,
}

/// An attribute: `#[path(arguments)]` or `#[path = value]`.
struct Attribute {
    /// The path, such as `test` or `tokio::test`.
    path: String,
    /// Everything after the path, trimmed.
    arguments: String,
}

impl Attribute {
    /// Reads `text`, which is a whole attribute from `#[` to its closing `]`.
    fn parse(text: &str) -> Self {
        let inner = text
            .strip_prefix("#[")
            .and_then(|rest| rest.strip_suffix(']'))
            .unwrap_or_default()
            .trim();
        let end = inner
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
            .unwrap_or(inner.len());
        Self {
            path: inner[..end].to_owned(),
            arguments: inner[end..].trim().to_owned(),
        }
    }

    /// The last segment of the path.
    fn name(&self) -> &str {
        self.path.rsplit("::").next().unwrap_or_default()
    }
}

/// The test below line `from` (counting from 0), or what stands in the way.
fn target(lines: &[&str], from: usize) -> Result<Test, String> {
    let trimmed = |index: usize| {
        lines
            .get(index)
            .map_or_else(String::new, |line| line.trim().to_owned())
    };
    let mut attributes = Vec::new();
    let mut index = from;
    // What is left to read of line `index`, and whether any of it was read yet.
    let mut rest = trimmed(index);
    let mut fresh = true;
    loop {
        if index >= lines.len() {
            return Err("the file ends first".to_owned());
        }
        if rest.is_empty() {
            if fresh {
                return Err("there is a blank line first".to_owned());
            }
            // An attribute ended the line.
            index += 1;
            rest = trimmed(index);
            fresh = true;
        } else if fresh && rest.starts_with("///") && !rest.starts_with("////") {
            index += 1;
            rest = trimmed(index);
        } else if rest.starts_with("#[") {
            let (attribute, last, remaining) = read_attribute(lines, index, &rest)?;
            attributes.push(attribute);
            index = last;
            rest = remaining;
            fresh = false;
        } else {
            return match fn_name(&rest) {
                Some(name) => test_of(name, &attributes),
                None => Err("the next line is not an attribute, a `///` line or a `fn`".to_owned()),
            };
        }
    }
}

/// Reads the attribute that starts `first`, the unread part of line `index`, and goes on to the
/// next lines until its brackets balance. Returns it, the line it ends on and what follows it on
/// that line.
fn read_attribute(
    lines: &[&str],
    index: usize,
    first: &str,
) -> Result<(Attribute, usize, String), String> {
    let mut text = first.to_owned();
    let mut last = index;
    loop {
        if let Some(end) = attribute_end(&text) {
            let attribute = Attribute::parse(&text[..end]);
            return Ok((attribute, last, text[end..].trim().to_owned()));
        }
        last += 1;
        let Some(next) = lines.get(last) else {
            return Err("an attribute is never closed".to_owned());
        };
        text.push('\n');
        text.push_str(next);
    }
}

/// The test the function `name` is, given the attributes above it.
fn test_of(name: String, attributes: &[Attribute]) -> Result<Test, String> {
    let is_test = attributes
        .iter()
        .any(|attribute| matches!(attribute.name(), "test" | "rstest" | "test_case"));
    if !is_test {
        return Err(format!(
            "`fn {name}` has no test attribute (`#[test]`, `#[<path>::test]`, `#[rstest]` or \
             `#[test_case]`)"
        ));
    }
    let counts = attributes
        .iter()
        .all(|attribute| match attribute.path.as_str() {
            "ignore" => false,
            "cfg" => names_only_the_target(&attribute.arguments),
            "cfg_attr" => !identifiers(&attribute.arguments).contains(&"ignore"),
            _ => true,
        });
    Ok(Test { name, counts })
}

/// Whether the predicate `arguments` of a `cfg` names only the target and the combinators, and
/// is not `any()`.
fn names_only_the_target(arguments: &str) -> bool {
    let compact: String = arguments.chars().filter(|c| !c.is_whitespace()).collect();
    !compact.contains("any()")
        && identifiers(arguments)
            .iter()
            .all(|word| TARGET_WORDS.contains(word))
}

/// The name of the function declared by `line`, after the qualifiers `pub`, `pub(…)`, `async`,
/// `const` and `unsafe`; `None` when the line does not declare a function.
fn fn_name(line: &str) -> Option<String> {
    let mut rest = line.trim_start();
    loop {
        let (word, tail) = rest.split_once(char::is_whitespace)?;
        if word == "fn" {
            let name: String = tail
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            return (!name.is_empty()).then_some(name);
        }
        let qualifier = matches!(word, "pub" | "async" | "const" | "unsafe")
            || (word.starts_with("pub(") && word.ends_with(')'));
        if !qualifier {
            return None;
        }
        rest = tail.trim_start();
    }
}

/// The byte after the `]` that closes the attribute `text` starts with, or `None` when it is not
/// closed. Brackets in strings, characters and line comments do not count.
fn attribute_end(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0_usize;
    let mut i = 0;
    while i < bytes.len() {
        if let Some(end) = literal_end(text, i) {
            i = end;
            continue;
        }
        match bytes[i] {
            b'[' => depth += 1,
            b']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                i = text[i..]
                    .find('\n')
                    .map_or(bytes.len(), |newline| i + newline);
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The words of `text` (letters, digits and `_`, starting with a letter or `_`), without
/// those inside strings and characters.
fn identifiers(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let is_word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    let mut words = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(end) = literal_end(text, i) {
            i = end;
        } else if is_word(bytes[i]) && !bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && is_word(bytes[i]) {
                i += 1;
            }
            words.push(&text[start..i]);
        } else {
            i += 1;
        }
    }
    words
}

/// The byte after the string or character literal that starts at byte `start` of `text`, or
/// `None` when no literal starts there (a lifetime is not one).
fn literal_end(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    match *bytes.get(start)? {
        b'"' => Some(string_end(bytes, start + 1)),
        b'r' => raw_string_end(bytes, start + 1),
        b'\'' => char_end(text, start + 1),
        _ => None,
    }
}

/// The byte after the closing quote of a string whose body starts at `from`; the end of the text
/// when the string is not closed.
fn string_end(bytes: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

/// The byte after a raw string (`r"…"`, `r#"…"#`) whose `r` is just before `from`, or `None`
/// when what follows the `r` is not a raw string.
fn raw_string_end(bytes: &[u8], from: usize) -> Option<usize> {
    let hashes = bytes
        .get(from..)?
        .iter()
        .take_while(|b| **b == b'#')
        .count();
    let body = from + hashes;
    if bytes.get(body) != Some(&b'"') {
        return None;
    }
    let closing: Vec<u8> = std::iter::once(b'"')
        .chain(std::iter::repeat_n(b'#', hashes))
        .collect();
    let found = bytes[body + 1..]
        .windows(closing.len())
        .position(|window| window == closing.as_slice());
    Some(found.map_or(bytes.len(), |at| body + 1 + at + closing.len()))
}

/// The byte after a character literal whose opening quote is just before `from`, or `None` when
/// the quote starts a lifetime.
fn char_end(text: &str, from: usize) -> Option<usize> {
    let mut chars = text.get(from..)?.char_indices();
    let (_, first) = chars.next()?;
    if first == '\\' {
        // An escape: the literal ends at the first quote after the escaped character.
        chars.next()?;
        let (offset, _) = chars.find(|(_, c)| *c == '\'')?;
        Some(from + offset + 1)
    } else {
        let (offset, next) = chars.next()?;
        (next == '\'').then_some(from + offset + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lines joined into the text of a source file.
    fn source(lines: &[&str]) -> String {
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }

    fn scan_lines(lines: &[&str]) -> (Vec<Tag>, Vec<Finding>) {
        scan("src/lib.rs", &source(lines))
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
        let (tags, findings) = scan("a.rs", text);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(tags.len(), 1);
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
            &["// AC: CE-08.1", "#[test]"],
            &["// AC: CE-08.1"],
            &["// AC: CE-08.1", "#[test", "fn never_closed() {}"],
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
            "/// AC: CE-08.1",
            "//! AC: CE-08.1",
            "// ACs are acceptance criteria",
            "// ACL rules",
            "// the AC: CE-08.1 tag",
            "let text = \"// AC: CE-08.1\";",
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
}
