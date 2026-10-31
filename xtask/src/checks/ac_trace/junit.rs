//! The nextest `JUnit` report (`target/nextest/<profile>/junit.xml`): which tests passed.
//!
//! `ac-trace` shows the result of every tagged test next to it. The report is small and regular,
//! so this reads just what is needed with a tolerant scanner instead of an XML library: the
//! elements `testcase`, `failure`, `error` and `skipped`, with comments and `CDATA` sections
//! (where the output of tests is) skipped whole. Nothing from the report goes into an error.

use std::collections::BTreeMap;

use crate::{Error, Result};

/// What the scanner skips whole: where it starts and where it ends. `<!` comes last, for the
/// declarations that are not comments or `CDATA` sections.
const SKIPPED: [(&str, &str); 4] = [
    ("<!--", "-->"),
    ("<![CDATA[", "]]>"),
    ("<?", "?>"),
    ("<!", ">"),
];

/// A `testcase` element that is being read.
struct Case {
    name: String,
    failed: bool,
    skipped: bool,
}

/// Reads the report `xml` into a map from test name to whether the test passed.
///
/// A test with a `failure` or `error` element failed. One with a `skipped` element did not run
/// and is left out. A test that failed and then passed on a retry (`flakyFailure`) passed. When a
/// name is in the report more than once, such as the same test in two binaries, it passed only if
/// it passed every time.
///
/// # Errors
///
/// Fails when the text is not an XML document whose root is `testsuites` or `testsuite`, a
/// `testcase` has no name or is inside another one, or an element is cut off.
pub fn parse(xml: &str) -> Result<BTreeMap<String, bool>> {
    let mut results = BTreeMap::new();
    let mut open: Option<Case> = None;
    let mut root_seen = false;
    let mut rest = xml;
    while let Some(start) = rest.find('<') {
        rest = &rest[start..];
        if let Some((opening, closing)) = SKIPPED
            .iter()
            .find(|(opening, _)| rest.starts_with(opening))
        {
            let body = &rest[opening.len()..];
            let end = body.find(closing).ok_or_else(|| {
                error("a comment, a CDATA section or a declaration is not closed")
            })?;
            rest = &body[end + closing.len()..];
            continue;
        }
        let (tag, after) = split_tag(rest)?;
        rest = after;
        if let Some(name) = tag.strip_prefix('/') {
            if name.trim() == "testcase"
                && let Some(case) = open.take()
            {
                record(&mut results, case);
            }
            continue;
        }
        let (tag, self_closing) = tag
            .strip_suffix('/')
            .map_or((tag, false), |tag| (tag, true));
        let (name, attributes) = tag.split_once(char::is_whitespace).unwrap_or((tag, ""));
        if !root_seen {
            if !matches!(name, "testsuites" | "testsuite") {
                return Err(error("the root element is not `testsuites` or `testsuite`"));
            }
            root_seen = true;
        }
        match name {
            "testcase" => {
                if open.is_some() {
                    return Err(error("a `testcase` is inside another one"));
                }
                let name = attribute(attributes, "name")
                    .ok_or_else(|| error("a `testcase` has no `name`"))?;
                let case = Case {
                    name,
                    failed: false,
                    skipped: false,
                };
                if self_closing {
                    record(&mut results, case);
                } else {
                    open = Some(case);
                }
            }
            "failure" | "error" => {
                if let Some(case) = open.as_mut() {
                    case.failed = true;
                }
            }
            "skipped" => {
                if let Some(case) = open.as_mut() {
                    case.skipped = true;
                }
            }
            _ => {}
        }
    }
    if open.is_some() {
        return Err(error("a `testcase` is not closed"));
    }
    if !root_seen {
        return Err(error("there is no XML element"));
    }
    Ok(results)
}

fn error(message: &str) -> Error {
    Error::Parse {
        what: "the nextest JUnit report".to_owned(),
        message: message.to_owned(),
    }
}

/// Adds the finished `case` to the results.
fn record(results: &mut BTreeMap<String, bool>, case: Case) {
    if case.skipped && !case.failed {
        return;
    }
    let passed = !case.failed;
    results
        .entry(case.name)
        .and_modify(|before| *before = *before && passed)
        .or_insert(passed);
}

/// Splits `text`, which starts with `<`, into what is inside the angle brackets and what comes
/// after them. A `>` inside the quotes of an attribute does not end the element.
fn split_tag(text: &str) -> Result<(&str, &str)> {
    let mut quote: Option<char> = None;
    for (offset, c) in text.char_indices().skip(1) {
        match (quote, c) {
            (None, '>') => return Ok((&text[1..offset], &text[offset + 1..])),
            (None, '"' | '\'') => quote = Some(c),
            (Some(open), _) if open == c => quote = None,
            _ => {}
        }
    }
    Err(error("an element is cut off"))
}

/// The value of the attribute `wanted` in `text`, which holds the attributes of an element.
fn attribute(text: &str, wanted: &str) -> Option<String> {
    let mut rest = text.trim_start();
    while let Some((key, after)) = rest.split_once('=') {
        let after = after.trim_start();
        let quote = after.chars().next().filter(|c| matches!(c, '"' | '\''))?;
        let value_and_rest = &after[1..];
        let end = value_and_rest.find(quote)?;
        if key.trim() == wanted {
            return Some(unescape(&value_and_rest[..end]));
        }
        rest = value_and_rest[end + 1..].trim_start();
    }
    None
}

/// `text` with the entities `&lt;`, `&gt;`, `&amp;`, `&quot;`, `&apos;` and `&#N;`/`&#xN;`
/// replaced; anything else that starts with `&` stays as it is.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let replacement = rest.find(';').and_then(|semicolon| {
            let entity = &rest[1..semicolon];
            let c = match entity {
                "lt" => Some('<'),
                "gt" => Some('>'),
                "amp" => Some('&'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => numeric_entity(entity),
            };
            c.map(|c| (c, semicolon + 1))
        });
        if let Some((c, length)) = replacement {
            out.push(c);
            rest = &rest[length..];
        } else {
            out.push('&');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

/// The character of `#65` or `#x41`.
fn numeric_entity(entity: &str) -> Option<char> {
    let digits = entity.strip_prefix('#')?;
    let code = match digits.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => digits.parse(),
    };
    char::from_u32(code.ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    const REPORT: &str = include_str!("../../../tests/fixtures/ac-trace/nextest-junit.xml");

    // AC: CE-08.6
    #[test]
    fn junit_parses_pass_and_fail() -> TestResult {
        let results = parse(REPORT)?;
        let expected: BTreeMap<String, bool> = [
            ("umbrella::tests::passes", true),
            ("umbrella::tests::fails", false),
            ("umbrella::tests::times_out", false),
            ("flaky::passes_on_retry", true),
            // Passed in one binary and failed in the other.
            ("shared::name", false),
            ("cases::quote\"d<1>&A", true),
            ("top_level_test", true),
        ]
        .into_iter()
        .map(|(name, passed)| (name.to_owned(), passed))
        .collect();
        assert_eq!(results, expected);
        Ok(())
    }

    #[test]
    fn an_empty_report_has_no_results() -> TestResult {
        let xml =
            "<?xml version=\"1.0\"?><testsuites name=\"nextest-run\" tests=\"0\"></testsuites>";
        assert!(parse(xml)?.is_empty());
        assert!(parse("<testsuite name=\"x\"/>")?.is_empty());
        Ok(())
    }

    #[test]
    fn text_that_is_not_a_report_is_an_error() {
        for xml in [
            "",
            "not xml at all",
            "<html><body/></html>",
            "<testsuites><testcase name=\"cut off\"",
            "<testsuites><testcase name=\"a\"><failure></testsuites>",
            "<testsuites><!-- never closed",
            "<testsuites><testcase name=\"a\"><![CDATA[never closed",
            "<testsuites><testcase></testcase></testsuites>",
            "<testsuites><testcase name=\"a\"><testcase name=\"b\"/></testcase></testsuites>",
        ] {
            let result = parse(xml);
            assert!(
                matches!(result, Err(Error::Parse { .. })),
                "{xml}: {result:?}"
            );
        }
    }
}
